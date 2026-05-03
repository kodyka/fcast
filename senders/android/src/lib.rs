use anyhow::{bail, Result};
use fcast_sender_sdk::{context::CastContext, device, device::DeviceInfo};
use gst::prelude::{BufferPoolExt, BufferPoolExtManual};
use gst_video::{VideoColorimetry, VideoFrameExt};
use jni::{
    objects::{JByteBuffer, JObject, JString},
    JavaVM,
};
use mcore::{transmission::WhepSink, DeviceEvent, Event, ShouldQuit, SourceConfig};
use parking_lot::{Condvar, Mutex};
#[cfg(target_os = "android")]
use serde_json::{json, Value};
use std::{collections::HashMap, net::Ipv6Addr, sync::Arc};
#[cfg(target_os = "android")]
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{debug, error};
#[cfg(target_os = "android")]
use tracing::{info, warn};

pub mod migration;

#[cfg(target_os = "android")]
type PlatformApp = slint::android::AndroidApp;

#[cfg(not(target_os = "android"))]
#[derive(Clone, Debug, Default)]
struct PlatformApp;

lazy_static::lazy_static! {
    pub static ref GLOB_EVENT_CHAN: (crossbeam_channel::Sender<Event>, crossbeam_channel::Receiver<Event>)
        = crossbeam_channel::bounded(2);
    pub static ref FRAME_PAIR: (Mutex<Option<gst_video::VideoFrame<gst_video::video_frame::Writable>>>, Condvar) = (Mutex::new(None), Condvar::new());
    pub static ref FRAME_POOL: Mutex<gst_video::VideoBufferPool> = Mutex::new(gst_video::VideoBufferPool::new());
}

#[cfg(target_os = "android")]
static CAPTURE_ACTIVE: AtomicBool = AtomicBool::new(false);

slint::include_modules!();

macro_rules! log_err {
    ($res:expr, $msg: expr) => {
        if let Err(err) = ($res) {
            error!(?err, $msg);
        }
    };
}

#[cfg(target_os = "android")]
const MIGRATION_COMMAND_BIND_ENV: &str = "MIGRATION_COMMAND_BIND";
#[cfg(target_os = "android")]
const LEGACY_COMMAND_BIND_ADDR: &str = "0.0.0.0:8080";

#[cfg(target_os = "android")]
fn ensure_gstreamer_initialized() -> std::result::Result<(), String> {
    use std::sync::OnceLock;

    static GST_INIT: OnceLock<std::result::Result<(), String>> = OnceLock::new();
    GST_INIT
        .get_or_init(|| {
            gst::init().map_err(|err| format!("Failed to initialize GStreamer: {err}"))
        })
        .clone()
}

#[cfg(not(target_os = "android"))]
fn ensure_gstreamer_initialized() -> std::result::Result<(), String> {
    gst::init().map_err(|err| format!("Failed to initialize GStreamer: {err}"))
}

#[cfg(target_os = "android")]
fn set_capture_active(active: bool) {
    CAPTURE_ACTIVE.store(active, Ordering::SeqCst);
    if !active {
        let (lock, cvar) = &*FRAME_PAIR;
        let mut frame = lock.lock();
        *frame = None;
        cvar.notify_all();
    }
}

#[cfg(target_os = "android")]
fn command_probe_addr(bind_addr: &str) -> String {
    if let Some(port) = bind_addr.strip_prefix("0.0.0.0:") {
        return format!("127.0.0.1:{port}");
    }
    if let Some(port) = bind_addr.strip_prefix("[::]:") {
        return format!("[::1]:{port}");
    }
    bind_addr.to_string()
}

#[cfg(target_os = "android")]
fn send_http_request(
    bind_addr: &str,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> std::result::Result<String, String> {
    use std::io::{Read, Write};

    let connect_addr = command_probe_addr(bind_addr);
    let mut stream = std::net::TcpStream::connect(&connect_addr)
        .map_err(|err| format!("Failed to connect to migrated server {connect_addr}: {err}"))?;

    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(3)));
    let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(3)));

    let body_text = body.unwrap_or("");
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {connect_addr}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body_text}",
        body_text.len()
    );

    stream
        .write_all(request.as_bytes())
        .map_err(|err| format!("Failed to write HTTP request to migrated server: {err}"))?;
    stream
        .flush()
        .map_err(|err| format!("Failed to flush HTTP request to migrated server: {err}"))?;

    let mut response_bytes = Vec::new();
    stream
        .read_to_end(&mut response_bytes)
        .map_err(|err| format!("Failed to read HTTP response from migrated server: {err}"))?;

    let response = String::from_utf8_lossy(&response_bytes);
    let mut sections = response.splitn(2, "\r\n\r\n");
    let headers = sections.next().unwrap_or("");
    let response_body = sections.next().unwrap_or("").to_string();
    let status_line = headers.lines().next().unwrap_or("HTTP/1.1 000");
    if !status_line.contains(" 200 ") {
        return Err(format!(
            "Migrated server returned non-200 status: {status_line}; body={response_body}"
        ));
    }
    Ok(response_body)
}

#[cfg(target_os = "android")]
fn start_migrated_command_server(bind_addr: &str) -> std::result::Result<String, String> {
    ensure_gstreamer_initialized()?;
    std::env::set_var(MIGRATION_COMMAND_BIND_ENV, bind_addr);
    crate::migration::runtime::start_graph_runtime()
        .map_err(|err| format!("Failed to start migrated graph runtime: {err}"))?;
    let health_body = send_http_request(bind_addr, "GET", "/health", None)?;
    Ok(format!(
        "migrated server active bind={bind_addr} health={}",
        health_body.trim()
    ))
}

#[cfg(target_os = "android")]
fn run_graph_http_command(bind_addr: &str, payload: Value) -> std::result::Result<Value, String> {
    let payload_json = payload.to_string();
    let body = send_http_request(bind_addr, "POST", "/command", Some(&payload_json))?;
    let response: Value = serde_json::from_str(&body)
        .map_err(|err| format!("Failed to parse migrated server response: {err}; raw={body}"))?;
    let result = response
        .get("result")
        .ok_or_else(|| format!("Missing result in migrated server response: {body}"))?;

    if let Some(err) = result.get("error").and_then(Value::as_str) {
        return Err(format!("Migrated server command error: {err}"));
    }

    Ok(response)
}

#[cfg(target_os = "android")]
fn run_graph_command(action: &str, params: Value) -> std::result::Result<Value, String> {
    let payload = json!({ action: params });
    let response_json = crate::migration::runtime::try_handle_command_json(&payload.to_string());
    let root: Value = serde_json::from_str(&response_json)
        .map_err(|err| format!("{action} parse failure: {err}; raw={response_json}"))?;
    let result = root
        .get("result")
        .cloned()
        .ok_or_else(|| format!("{action} missing result field; raw={response_json}"))?;
    match &result {
        Value::String(ok) if ok == "success" => Ok(result),
        Value::Object(map) => {
            if let Some(err) = map.get("error").and_then(Value::as_str) {
                Err(format!("{action} error: {err}"))
            } else {
                Ok(result)
            }
        }
        _ => Err(format!(
            "{action} unsupported result shape: {response_json}"
        )),
    }
}

#[cfg(target_os = "android")]
fn run_legacy_http_getinfo_test(bind_addr: &str) -> String {
    if let Err(err) = start_migrated_command_server(bind_addr) {
        return format!("FAIL {err}");
    }

    match run_graph_http_command(bind_addr, json!({ "getinfo": {} })) {
        Ok(info) => {
            let node_count = info
                .get("result")
                .and_then(|result| result.get("info"))
                .and_then(|info| info.get("nodes"))
                .and_then(Value::as_object)
                .map(|nodes| nodes.len())
                .unwrap_or(0);
            format!("PASS legacy getinfo (/command) nodes={node_count}")
        }
        Err(err) => format!("FAIL {err}"),
    }
}

#[cfg(target_os = "android")]
fn run_legacy_http_crossfade_test(bind_addr: &str) -> String {
    if let Err(err) = start_migrated_command_server(bind_addr) {
        return format!("FAIL {err}");
    }

    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let mixer_id = format!("legacy-channel-{millis}");
    let destination_id = format!("legacy-output-{millis}");
    let link_id = format!("{mixer_id}->{destination_id}-{millis}");
    let slot_source_id = format!("legacy-source-slot-{millis}");
    let slot_link_id = format!("{slot_source_id}->{mixer_id}-{millis}");

    let mut mixer_created = false;
    let mut destination_created = false;
    let mut slot_source_created = false;

    let result = (|| -> std::result::Result<String, String> {
        // Derived from scripts_test_api/crossfade.py bootstrap sequence.
        run_graph_http_command(
            bind_addr,
            json!({
                "createmixer": {
                    "id": mixer_id.clone(),
                    "config": {
                        "width": 1280,
                        "height": 720,
                        "sample-rate": 44100
                    }
                }
            }),
        )?;
        mixer_created = true;

        run_graph_http_command(
            bind_addr,
            json!({
                "createdestination": {
                    "id": destination_id.clone(),
                    "family": "LocalPlayback"
                }
            }),
        )?;
        destination_created = true;

        run_graph_http_command(
            bind_addr,
            json!({
                "connect": {
                    "link_id": link_id.clone(),
                    "src_id": mixer_id.clone(),
                    "sink_id": destination_id.clone()
                }
            }),
        )?;
        run_graph_http_command(
            bind_addr,
            json!({
                "start": {
                    "id": destination_id.clone()
                }
            }),
        )?;
        run_graph_http_command(
            bind_addr,
            json!({
                "start": {
                    "id": mixer_id.clone()
                }
            }),
        )?;

        run_graph_http_command(
            bind_addr,
            json!({
                "createvideogenerator": {
                    "id": slot_source_id.clone()
                }
            }),
        )?;
        slot_source_created = true;

        run_graph_http_command(
            bind_addr,
            json!({
                "connect": {
                    "link_id": slot_link_id.clone(),
                    "src_id": slot_source_id.clone(),
                    "sink_id": mixer_id.clone(),
                    "audio": false,
                    "video": true,
                    "config": {
                        "video::zorder": 2,
                        "video::alpha": 1.0,
                        "video::width": 1280,
                        "video::height": 720,
                        "video::sizing-policy": "keep-aspect-ratio"
                    }
                }
            }),
        )?;
        run_graph_http_command(
            bind_addr,
            json!({
                "start": {
                    "id": slot_source_id.clone()
                }
            }),
        )?;

        let info = run_graph_http_command(bind_addr, json!({ "getinfo": {} }))?;
        let node_count = info
            .get("result")
            .and_then(|result| result.get("info"))
            .and_then(|info| info.get("nodes"))
            .and_then(Value::as_object)
            .map(|nodes| nodes.len())
            .unwrap_or(0);
        Ok(format!(
            "legacy crossfade bootstrap ok mixer={mixer_id} destination={destination_id} slot_source={slot_source_id} nodes={node_count}"
        ))
    })();

    if slot_source_created {
        let _ = run_graph_http_command(
            bind_addr,
            json!({
                "remove": {
                    "id": slot_source_id.clone()
                }
            }),
        );
    }
    if destination_created {
        let _ = run_graph_http_command(
            bind_addr,
            json!({
                "remove": {
                    "id": destination_id.clone()
                }
            }),
        );
    }
    if mixer_created {
        let _ = run_graph_http_command(
            bind_addr,
            json!({
                "remove": {
                    "id": mixer_id.clone()
                }
            }),
        );
    }

    match result {
        Ok(success) => format!("PASS {success}"),
        Err(err) => format!("FAIL {err}"),
    }
}

#[cfg(target_os = "android")]
fn run_graph_smoke_test() -> String {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let source_id = format!("slint-smoke-videogen-{millis}");
    let mixer_id = format!("slint-smoke-mixer-{millis}");
    let link_id = format!("slint-smoke-link-{millis}");

    let mut source_created = false;
    let mut mixer_created = false;
    let result = (|| -> std::result::Result<String, String> {
        run_graph_command("createvideogenerator", json!({ "id": source_id.clone() }))?;
        source_created = true;

        run_graph_command(
            "createmixer",
            json!({
                "id": mixer_id.clone(),
                "audio": false,
                "video": true
            }),
        )?;
        mixer_created = true;

        run_graph_command(
            "connect",
            json!({
                "link_id": link_id.clone(),
                "src_id": source_id.clone(),
                "sink_id": mixer_id.clone(),
                "audio": false,
                "video": true
            }),
        )?;
        run_graph_command("start", json!({ "id": mixer_id.clone() }))?;
        run_graph_command("start", json!({ "id": source_id.clone() }))?;

        let info = run_graph_command("getinfo", json!({}))?;
        let node_count = info
            .get("info")
            .and_then(|info| info.get("nodes"))
            .and_then(Value::as_object)
            .map(|nodes| nodes.len())
            .unwrap_or(0);

        Ok(format!(
            "smoke ok source={source_id} mixer={mixer_id} link={link_id} nodes={node_count}"
        ))
    })();

    if source_created {
        let _ = run_graph_command("remove", json!({ "id": source_id.clone() }));
    }
    if mixer_created {
        let _ = run_graph_command("remove", json!({ "id": mixer_id.clone() }));
    }

    match result {
        Ok(success) => format!("PASS {success}"),
        Err(err) => format!("FAIL {err}"),
    }
}

#[cfg(target_os = "android")]
fn log_ui_test_status(test_name: &'static str, status: &str) {
    if status.starts_with("PASS") {
        info!(test = test_name, status = status, "UI test completed");
    } else {
        warn!(test = test_name, status = status, "UI test failed");
    }
}

#[derive(Debug)]
enum JavaMethod {
    StopCapture,
    ScanQr,
}

#[cfg(target_os = "android")]
fn call_java_method_no_args(app: &PlatformApp, method: JavaMethod) {
    let vm = unsafe {
        let ptr = app.vm_as_ptr() as *mut jni::sys::JavaVM;
        assert!(!ptr.is_null(), "JavaVM ptr is null");
        JavaVM::from_raw(ptr).unwrap()
    };
    let activity = unsafe {
        let ptr = app.activity_as_ptr() as *mut jni::sys::_jobject;
        assert!(!ptr.is_null(), "Activity ptr is null");
        JObject::from_raw(ptr)
    };

    let method_name = match method {
        JavaMethod::StopCapture => "stopCapture",
        JavaMethod::ScanQr => "scanQr",
    };

    match vm.get_env() {
        Ok(mut env) => match env.call_method(activity, method_name, "()V", &[]) {
            Ok(_) => (),
            Err(err) => error!(?err, ?method, "Failed to call java method"),
        },
        Err(err) => error!(?err, "Failed to get env from VM"),
    }
}

#[cfg(not(target_os = "android"))]
fn call_java_method_no_args(_app: &PlatformApp, _method: JavaMethod) {}

struct Application {
    ui_weak: slint::Weak<MainWindow>,
    event_tx: tokio::sync::mpsc::UnboundedSender<Event>,
    devices: HashMap<String, DeviceInfo>,
    cast_ctx: CastContext,
    active_device: Option<Arc<dyn device::CastingDevice>>,
    current_device_id: usize,
    local_address: Option<fcast_sender_sdk::IpAddr>,
    android_app: PlatformApp,
    tx_sink: Option<WhepSink>,
    our_source_url: Option<String>,
}


fn build_status_items(receiver_name: &str, encoder: &str, network: &str) -> Vec<crate::StatusItem> {
    vec![
        crate::StatusItem {
            label: "Receiver".into(),
            value: receiver_name.into(),
            severity: crate::StatusSeverity::Info,
        },
        crate::StatusItem {
            label: "Encoder".into(),
            value: encoder.into(),
            severity: crate::StatusSeverity::Info,
        },
        crate::StatusItem {
            label: "Network".into(),
            value: network.into(),
            severity: crate::StatusSeverity::Info,
        },
    ]
}

impl Application {
    pub async fn new(
        ui_weak: slint::Weak<MainWindow>,
        event_tx: tokio::sync::mpsc::UnboundedSender<Event>,
        android_app: PlatformApp,
    ) -> Result<Self> {
        std::thread::spawn({
            let event_tx = event_tx.clone();
            move || loop {
                match GLOB_EVENT_CHAN.1.recv() {
                    Ok(event) => {
                        if let Err(err) = event_tx.send(event) {
                            error!("Failed to forward event to event loop: {err}");
                            break;
                        }
                    }
                    Err(err) => {
                        error!("Failed to receive event from the global event channel: {err}");
                        break;
                    }
                }
            }
        });

        Ok(Self {
            ui_weak,
            event_tx,
            devices: HashMap::new(),
            cast_ctx: CastContext::new()?,
            active_device: None,
            current_device_id: 0,
            local_address: None,
            android_app,
            tx_sink: None,
            our_source_url: None,
        })
    }

    fn update_receivers_in_ui(&mut self) -> Result<()> {
        let receivers = self
            .devices
            .iter()
            .filter(|(_, info)| !info.addresses.is_empty() && info.port != 0)
            .map(|(name, _)| slint::SharedString::from(name))
            .collect::<Vec<slint::SharedString>>();
        self.ui_weak.upgrade_in_event_loop(move |ui| {
            let model = std::rc::Rc::new(slint::VecModel::<slint::SharedString>::from_iter(
                receivers.into_iter(),
            ));
            ui.global::<Bridge>().set_devices(model.into());
        })?;

        Ok(())
    }

    fn add_or_update_device(&mut self, device_info: DeviceInfo) -> Result<()> {
        self.devices.insert(device_info.name.clone(), device_info);
        self.update_receivers_in_ui()?;
        Ok(())
    }

    async fn stop_cast(&mut self, stop_playback: bool) -> Result<()> {
        let android_app = self.android_app.clone();
        self.ui_weak.upgrade_in_event_loop(move |_| {
            call_java_method_no_args(&android_app, JavaMethod::StopCapture);
        })?;

        if let Some(active_device) = self.active_device.take() {
            tokio::spawn(async move {
                if stop_playback {
                    debug!("Stopping playback");
                    log_err!(active_device.stop_playback(), "Failed to stop playback");
                    // NOTE: Instead of waiting for the PlaybackState::Idle event in the main loop we just sleep here
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
                debug!("Disconnecting from active device");
                log_err!(
                    active_device.disconnect(),
                    "Failed to disconnect from active device"
                );
            });
        }

        if let Some(mut tx_sink) = self.tx_sink.take() {
            tx_sink.shutdown();
        }

        Ok(())
    }

    fn connect_with_device_info(&mut self, device_info: DeviceInfo) -> Result<()> {
        let device = self.cast_ctx.create_device_from_info(device_info);
        self.current_device_id += 1;
        device
            .connect(
                None,
                Arc::new(mcore::DeviceHandler::new(
                    self.current_device_id,
                    self.event_tx.clone(),
                )),
                1000,
            )
            .unwrap();
        self.active_device = Some(device);
        self.ui_weak.upgrade_in_event_loop(|ui| {
            ui.global::<Bridge>()
                .invoke_change_state(AppState::Connecting);
        })?;

        Ok(())
    }

    /// Returns `true` if the event loop should quit
    async fn handle_event(&mut self, event: Event) -> Result<ShouldQuit> {
        debug!("Handling event: {event:?}");

        match event {
            Event::EndSession { .. } => {
                self.ui_weak.upgrade_in_event_loop(|ui| {
                    ui.global::<Bridge>().set_status_items(std::rc::Rc::new(slint::VecModel::default()).into());
                    ui.global::<Bridge>()
                        .invoke_change_state(AppState::Disconnected);
                })?;

                self.stop_cast(true).await?;
            }
            Event::ConnectToDevice(device_name) => {
                if let Some(device_info) = self.devices.get(&device_name) {
                    self.connect_with_device_info(device_info.clone())?;
                } else {
                    error!("No device with name `{device_name}` found");
                }
            }
            Event::SignallerStarted {
                bound_port_v4,
                bound_port_v6,
            } => {
                let Some(addr) = self.local_address.as_ref() else {
                    error!("Local address is missing");
                    return Ok(ShouldQuit::No);
                };
                let bound_port = match addr {
                    fcast_sender_sdk::IpAddr::V4 { .. } => bound_port_v4,
                    fcast_sender_sdk::IpAddr::V6 { .. } => bound_port_v6,
                };

                let (content_type, url) = self
                    .tx_sink
                    .as_ref()
                    .unwrap()
                    .get_play_msg(addr.into(), bound_port);

                debug!(content_type, url, "Sending play message");
                self.our_source_url = Some(url.clone());

                match self.active_device.as_ref() {
                    Some(device) => {
                        device.load(device::LoadRequest::Url {
                            content_type,
                            url,
                            resume_position: None,
                            speed: None,
                            volume: None,
                            metadata: None,
                            request_headers: None,
                        })?;
                    }
                    None => error!("Active device is missing, cannot send play message"),
                }

                // self.ui_weak.upgrade_in_event_loop(|ui| {
                //     ui.global::<Bridge>().invoke_change_state(AppState::Casting);
                // })?;
            }
            Event::Quit => return Ok(ShouldQuit::Yes),
            Event::DeviceAvailable(device_info) => self.add_or_update_device(device_info)?,
            Event::DeviceRemoved(device_name) => {
                if self.devices.remove(&device_name).is_some() {
                    self.update_receivers_in_ui()?;
                } else {
                    debug!(device_name, "Tried to remove device but it was not found");
                }
            }
            Event::DeviceChanged(device_info) => self.add_or_update_device(device_info)?,
            Event::FromDevice { id, event } => {
                if id != self.current_device_id {
                    debug!(
                        "Got message from old device (id: {id} current: {})",
                        self.current_device_id
                    );
                } else {
                    match event {
                        DeviceEvent::StateChanged(device_connection_state) => {
                            match device_connection_state {
                                device::DeviceConnectionState::Connected { local_addr, .. } => {
                                    self.local_address = Some(local_addr);

                                    self.ui_weak.upgrade_in_event_loop(|ui| {
                                        ui.global::<Bridge>()
                                            .invoke_change_state(AppState::SelectingSettings);
                                    })?;
                                }
                                _ => (),
                            }
                        }
                        DeviceEvent::SourceChanged(new_source) => {
                            if self.tx_sink.is_some() {
                                match new_source {
                                    fcast_sender_sdk::device::Source::Url { ref url, .. } => {
                                        if Some(url) != self.our_source_url.as_ref() {
                                            // At this point the receiver has stopped playing our stream
                                            debug!(
                                                ?new_source,
                                                "The source on the receiver changed, disconnecting"
                                            );

                                            self.ui_weak.upgrade_in_event_loop(|ui| {
                                                ui.global::<Bridge>().set_status_items(std::rc::Rc::new(slint::VecModel::default()).into());
                                                ui.global::<Bridge>()
                                                    .invoke_change_state(AppState::Disconnected);
                                            })?;

                                            self.stop_cast(false).await?;
                                        }
                                    }
                                    _ => (),
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            #[cfg(target_os = "android")]
            Event::CaptureStopped => {
                set_capture_active(false);
            }
            #[cfg(target_os = "android")]
            Event::CaptureCancelled => {
                set_capture_active(false);
                self.ui_weak.upgrade_in_event_loop(|ui| {
                    ui.global::<Bridge>().set_status_items(std::rc::Rc::new(slint::VecModel::default()).into());
                    ui.global::<Bridge>()
                        .invoke_change_state(AppState::Disconnected);
                })?;

                self.stop_cast(false).await?;
            }
            #[cfg(target_os = "android")]
            Event::QrScanResult(result) => {
                match fcast_sender_sdk::device::device_info_from_url(result) {
                    Some(device_info) => {
                        self.connect_with_device_info(device_info)?;
                    }
                    None => {
                        error!("QR code scan result is not a valid device");
                    }
                }
            }
            #[cfg(target_os = "android")]
            Event::CaptureStarted => {
                set_capture_active(true);
                let appsrc = gst_app::AppSrc::builder()
                    .caps(
                        &gst_video::VideoCapsBuilder::new()
                            .format(gst_video::VideoFormat::I420)
                            // .framerate(gst::Fraction::new(0, 1))
                            .build(),
                    )
                    .is_live(true)
                    .do_timestamp(true)
                    .format(gst::Format::Time)
                    .max_buffers(1)
                    .build();

                let mut caps = None::<gst::Caps>;
                appsrc.set_callbacks(
                    gst_app::AppSrcCallbacks::builder()
                        .need_data(move |appsrc, _| {
                            let frame = {
                                let (lock, cvar) = &*FRAME_PAIR;
                                let mut frame = lock.lock();
                                while (*frame).is_none()
                                    && CAPTURE_ACTIVE.load(Ordering::SeqCst)
                                {
                                    cvar.wait_for(&mut frame, std::time::Duration::from_millis(100));
                                }
                                (*frame).take()
                            };
                            let Some(frame) = frame else {
                                if !CAPTURE_ACTIVE.load(Ordering::SeqCst) {
                                    let _ = appsrc.end_of_stream();
                                }
                                return;
                            };

                            use gst_video::prelude::*;

                            let now_caps = gst_video::VideoInfo::builder(
                                frame.format(),
                                frame.width(),
                                frame.height(),
                            )
                            .build()
                            .unwrap()
                            .to_caps()
                            .unwrap();

                            match &caps {
                                Some(old_caps) => {
                                    if *old_caps != now_caps {
                                        appsrc.set_caps(Some(&now_caps));
                                        caps = Some(now_caps);
                                    }
                                }
                                None => {
                                    appsrc.set_caps(Some(&now_caps));
                                    caps = Some(now_caps);
                                }
                            }

                            let _ = appsrc.push_buffer(frame.into_buffer());
                        })
                        .build(),
                );

                let source_config = SourceConfig::Video(mcore::VideoSource::Source(appsrc));

                self.tx_sink = Some(mcore::transmission::WhepSink::new(
                    source_config,
                    self.event_tx.clone(),
                    tokio::runtime::Handle::current(),
                    1920,
                    1080,
                    30,
                )?);

                let receiver_name = self.active_device.as_ref().map(|d| d.name()).unwrap_or_default();
                let encoder_name = "Hardware"; // Blocked by P0-1: Placeholder until encoder selection works
                let network_info = self.local_address.as_ref().map(|a| a.to_string()).unwrap_or_default();
                let status_items = build_status_items(&receiver_name, encoder_name, &network_info);

                self.ui_weak.upgrade_in_event_loop(move |ui| {
                    let status_model = std::rc::Rc::new(slint::VecModel::from(status_items));
                    ui.global::<Bridge>().set_status_items(status_model.into());
                    ui.global::<Bridge>().invoke_change_state(AppState::Casting);
                })?;
            }
            #[cfg(target_os = "android")]
            Event::StartCast {
                scale_width,
                scale_height,
                max_framerate,
            } => {
                let android_app = self.android_app.clone();
                self.ui_weak.upgrade_in_event_loop(move |ui| {
                    let vm = unsafe {
                        let ptr = android_app.vm_as_ptr() as *mut jni::sys::JavaVM;
                        assert!(!ptr.is_null(), "JavaVM ptr is null");
                        JavaVM::from_raw(ptr).unwrap()
                    };
                    let activity = unsafe {
                        let ptr = android_app.activity_as_ptr() as *mut jni::sys::_jobject;
                        assert!(!ptr.is_null(), "Activity ptr is null");
                        JObject::from_raw(ptr)
                    };

                    let scale_width = scale_width as jni::sys::jint;
                    let scale_height = scale_height as jni::sys::jint;
                    let max_framerate = max_framerate as jni::sys::jint;

                    match vm.get_env() {
                        Ok(mut env) => match env.call_method(
                            activity,
                            "startScreenCapture",
                            "(III)V",
                            &[
                                scale_width.into(),
                                scale_height.into(),
                                max_framerate.into(),
                            ],
                        ) {
                            Ok(_) => (),
                            Err(err) => error!(
                                ?err,
                                method = "startScreenCapture",
                                "Failed to call java method"
                            ),
                        },
                        Err(err) => error!(?err, "Failed to get env from VM"),
                    }

                    ui.global::<Bridge>()
                        .invoke_change_state(AppState::WaitingForMedia);
                })?;
            }
            #[cfg(not(target_os = "android"))]
            Event::StartCast {
                scale_width: _,
                scale_height: _,
                max_framerate: _,
                ..
            } => {
                debug!("Ignoring StartCast in non-android build of android-sender");
            }
            _ => {}
        }

        Ok(ShouldQuit::No)
    }

    pub async fn run_event_loop(
        mut self,
        mut event_rx: tokio::sync::mpsc::UnboundedReceiver<Event>,
    ) -> Result<()> {
        tracing_gstreamer::integrate_events();
        gst::log::remove_default_log_function();
        gst::log::set_default_threshold(gst::DebugLevel::Fixme);
        ensure_gstreamer_initialized()
            .map_err(|err| anyhow::anyhow!("Failed to initialize GStreamer: {err}"))?;
        debug!("GStreamer version: {:?}", gst::version());
        if let Err(err) = crate::migration::runtime::start_graph_runtime() {
            error!(?err, "Failed to start migrated graph runtime");
        }

        // self.add_or_update_device(fcast_sender_sdk::device::DeviceInfo::fcast("Localhost for android emulator".to_owned(), vec![fcast_sender_sdk::IpAddr::v4(10, 0, 2, 2)], 46899))?;

        loop {
            let Some(event) = event_rx.recv().await else {
                debug!("No more events");
                break;
            };

            if self.handle_event(event).await? == ShouldQuit::Yes {
                break;
            }
        }

        debug!("Quitting event loop");
        if let Err(err) = crate::migration::runtime::shutdown_graph_runtime() {
            error!(?err, "Failed to shut down migrated graph runtime");
        }

        Ok(())
    }
}

// TODO: handle errs
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: PlatformApp) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Debug),
    );

    let app_clone = app.clone();

    slint::android::init(app).unwrap();

    let ui = MainWindow::new().unwrap();


    let mut actions = vec![
        QuickAction { id: "scan-qr".into(), title: "Scan QR".into(), enabled: true, active: false },
    ];
    let show_debug = cfg!(debug_assertions);
    ui.global::<Bridge>().set_show_debug(show_debug);
    if show_debug {
        actions.extend([
            QuickAction { id: "migrated-server".into(), title: "Start Server".into(), enabled: true, active: false },
            QuickAction { id: "test-getinfo".into(),    title: "GetInfo".into(),      enabled: true, active: false },
            QuickAction { id: "test-crossfade".into(),  title: "Crossfade".into(),    enabled: true, active: false },
            QuickAction { id: "test-smoke".into(),      title: "Smoke Graph".into(),  enabled: true, active: false },
        ]);
    }
    let model = std::rc::Rc::new(slint::VecModel::from(actions));
    ui.global::<Bridge>().set_quick_actions(model.into());

    let runtime = tokio::runtime::Runtime::new().unwrap();

    let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();

    ui.global::<Bridge>().on_connect_receiver({
        let event_tx = event_tx.clone();
        move |device_name| {
            event_tx
                .send(Event::ConnectToDevice(device_name.to_string()))
                .unwrap();
        }
    });

    ui.global::<Bridge>().on_start_casting({
        let event_tx = event_tx.clone();
        move |scale_width: i32, scale_height: i32, max_framerate: i32| {
            event_tx
                .send(Event::StartCast {
                    scale_width: scale_width as u32,
                    scale_height: scale_height as u32,
                    max_framerate: max_framerate as u32,
                })
                .unwrap();
        }
    });

    ui.global::<Bridge>().on_stop_casting({
        let event_tx = event_tx.clone();
        move || {
            event_tx
                .send(Event::EndSession { disconnect: true })
                .unwrap();
        }
    });

    ui.global::<Bridge>().on_invoke_action({
        let app_clone = app_clone.clone();
        let ui_weak = ui.as_weak();
        move |id| {
            let id_str = id.as_str();
            match id_str {
                "scan-qr" => {
                    call_java_method_no_args(&app_clone, JavaMethod::ScanQr);
                }
                "migrated-server" => {
                    let status = match start_migrated_command_server(LEGACY_COMMAND_BIND_ADDR) {
                        Ok(message) => format!("PASS {message}"),
                        Err(err) => format!("FAIL {err}"),
                    };
                    log_ui_test_status("start-migrated-server", &status);
                    let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                        ui.global::<Bridge>().set_test_status(status.into());
                    });
                }
                "test-getinfo" => {
                    let _ = ui_weak.upgrade_in_event_loop(|ui| ui.global::<Bridge>().set_test_status("Running legacy getinfo test...".into()));
                    let ui_weak_clone = ui_weak.clone();
                    std::thread::spawn(move || {
                        let status = run_legacy_http_getinfo_test(LEGACY_COMMAND_BIND_ADDR);
                        log_ui_test_status("legacy-getinfo", &status);
                        let _ = ui_weak_clone.upgrade_in_event_loop(move |ui| ui.global::<Bridge>().set_test_status(status.into()));
                    });
                }
                "test-crossfade" => {
                    let _ = ui_weak.upgrade_in_event_loop(|ui| ui.global::<Bridge>().set_test_status("Running legacy crossfade test...".into()));
                    let ui_weak_clone = ui_weak.clone();
                    std::thread::spawn(move || {
                        let status = run_legacy_http_crossfade_test(LEGACY_COMMAND_BIND_ADDR);
                        log_ui_test_status("legacy-crossfade", &status);
                        let _ = ui_weak_clone.upgrade_in_event_loop(move |ui| ui.global::<Bridge>().set_test_status(status.into()));
                    });
                }
                "test-smoke" => {
                    let _ = ui_weak.upgrade_in_event_loop(|ui| ui.global::<Bridge>().set_test_status("Running graph smoke test...".into()));
                    let ui_weak_clone = ui_weak.clone();
                    std::thread::spawn(move || {
                        let status = run_graph_smoke_test();
                        log_ui_test_status("graph-smoke", &status);
                        let _ = ui_weak_clone.upgrade_in_event_loop(move |ui| ui.global::<Bridge>().set_test_status(status.into()));
                    });
                }
                _ => {}
            }
        }
    });





    let ui_weak = ui.as_weak();

    let event_tx_clone = event_tx.clone();
    let app_jh = runtime.spawn(async move {
        Application::new(ui_weak, event_tx_clone, app_clone)
            .await
            .unwrap()
            .run_event_loop(event_rx)
            .await
            .unwrap();
    });

    ui.run().unwrap();

    runtime.block_on(async move {
        if let Err(err) = event_tx.send(Event::Quit) {
            error!(?err, "Failed to send quit event");
        }
        if let Err(err) = app_jh.await {
            error!(?err, "Android application task join failed");
        }
    });

    debug!("Finished");
}

#[cfg(target_os = "android")]
fn jstring_to_string<'local>(env: &mut jni::JNIEnv<'local>, s: &JString<'local>) -> Result<String> {
    Ok(env.get_string(s)?.to_string_lossy().to_string())
}

#[cfg(target_os = "android")]
#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "C" fn Java_org_fcast_android_sender_MainActivity_nativeGraphCommand<'local>(
    mut env: jni::JNIEnv<'local>,
    _class: jni::objects::JClass<'local>,
    command_json: jni::objects::JString<'local>,
) -> jni::sys::jstring {
    let response = match jstring_to_string(&mut env, &command_json) {
        Ok(json) => crate::migration::runtime::try_handle_command_json(&json),
        Err(err) => {
            error!(?err, "Failed to decode graph command payload from Java");
            crate::migration::runtime::try_handle_command_json("")
        }
    };

    match env.new_string(response) {
        Ok(jstr) => jstr.into_raw(),
        Err(err) => {
            error!(?err, "Failed to allocate Java response string");
            std::ptr::null_mut()
        }
    }
}

#[cfg(target_os = "android")]
#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "C" fn Java_org_fcast_android_sender_FCastDiscoveryListener_serviceFound<'local>(
    mut env: jni::JNIEnv<'local>,
    _class: jni::objects::JClass<'local>,
    name: JString<'local>,
    addrs: jni::objects::JObject,
    port: jni::sys::jint,
) {
    let name = match jstring_to_string(&mut env, &name) {
        Ok(name) => name,
        Err(err) => {
            error!(?err, "Failed to convert jstring to string");
            return;
        }
    };
    let port = port as u16;
    let addrs = match jni::objects::JList::from_env(&mut env, &addrs) {
        Ok(addrs) => addrs,
        Err(err) => {
            error!(?err, "Failed to get address list from env");
            return;
        }
    };
    let mut ip_addrs = Vec::<fcast_sender_sdk::IpAddr>::new();
    let n_addrs = match addrs.size(&mut env) {
        Ok(n) => n,
        Err(err) => {
            error!(?err, "Failed to get JList size");
            return;
        }
    };
    for i in 0..n_addrs {
        let Ok(Some(addr)) = addrs.get(&mut env, i) else {
            continue;
        };
        let buffer = unsafe { JByteBuffer::from_raw(*addr) };

        let buffer_cap = match env.get_direct_buffer_capacity(&buffer) {
            Ok(cap) => cap,
            Err(err) => {
                error!(?err, "Failed to get capacity of the byte buffer");
                continue;
            }
        };

        debug!(buffer_cap);

        let buffer_ptr = match env.get_direct_buffer_address(&buffer) {
            Ok(ptr) => {
                assert!(!ptr.is_null());
                ptr
            }
            Err(err) => {
                error!(?err, "Failed to get buffer address");
                continue;
            }
        };

        let buffer_slice: &[u8] = unsafe { std::slice::from_raw_parts(buffer_ptr, buffer_cap) };

        ip_addrs.push(match buffer_slice.len() {
            4 => fcast_sender_sdk::IpAddr::v4(
                buffer_slice[0],
                buffer_slice[1],
                buffer_slice[2],
                buffer_slice[3],
            ),
            20 => {
                let mut addr_slice = [0; 16];
                for i in 0..addr_slice.len() {
                    addr_slice[i] = buffer_slice[i];
                }
                let addr = Ipv6Addr::from(addr_slice);
                let scope_id_slice = &buffer_slice[16..20];
                let this_scope_id = i32::from_le_bytes([
                    scope_id_slice[0],
                    scope_id_slice[1],
                    scope_id_slice[2],
                    scope_id_slice[3],
                ]) as u32;
                let mut ip = fcast_sender_sdk::IpAddr::from(std::net::IpAddr::V6(addr));
                match &mut ip {
                    fcast_sender_sdk::IpAddr::V6 { scope_id, .. } => *scope_id = this_scope_id,
                    _ => (),
                }
                ip
            }
            len => {
                error!(len, "Invalid address buffer length");
                continue;
            }
        });
    }

    let device_info = fcast_sender_sdk::device::DeviceInfo::fcast(name, ip_addrs, port);
    debug!(?device_info, "Found device");

    log_err!(
        GLOB_EVENT_CHAN.0.send(Event::DeviceAvailable(device_info)),
        "Failed to send device available event"
    );
}

#[cfg(target_os = "android")]
#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "C" fn Java_org_fcast_android_sender_FCastDiscoveryListener_serviceLost<'local>(
    mut env: jni::JNIEnv<'local>,
    _class: jni::objects::JClass<'local>,
    name: jni::objects::JString<'local>,
) {
    match jstring_to_string(&mut env, &name) {
        Ok(name) => log_err!(
            GLOB_EVENT_CHAN.0.send(Event::DeviceRemoved(name)),
            "Failed to send device removed event"
        ),
        Err(err) => error!(?err, "Failed to convert jstring to string"),
    }
}

#[cfg(target_os = "android")]
#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "C" fn Java_org_fcast_android_sender_MainActivity_nativeCaptureStarted<'local>(
    _env: jni::JNIEnv<'local>,
    _class: jni::objects::JClass<'local>,
) {
    debug!("Screen capture was started");
    log_err!(
        GLOB_EVENT_CHAN.0.send(Event::CaptureStarted),
        "Failed to send capture started event"
    );
}

#[cfg(target_os = "android")]
#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "C" fn Java_org_fcast_android_sender_MainActivity_nativeCaptureStopped<'local>(
    _env: jni::JNIEnv<'local>,
    _class: jni::objects::JClass<'local>,
) {
    debug!("Screen capture was stopped");
    log_err!(
        GLOB_EVENT_CHAN.0.send(Event::CaptureStopped),
        "Failed to send capture stopped event"
    );
}

#[cfg(target_os = "android")]
#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "C" fn Java_org_fcast_android_sender_MainActivity_nativeCaptureCancelled<'local>(
    _env: jni::JNIEnv<'local>,
    _class: jni::objects::JClass<'local>,
) {
    debug!("Screen capture was cancelled");
    log_err!(
        GLOB_EVENT_CHAN.0.send(Event::CaptureCancelled),
        "Failed to send capture cancelled event"
    );
}

#[cfg(target_os = "android")]
fn process_frame<'local>(
    env: jni::JNIEnv<'local>,
    width: jni::sys::jint,
    height: jni::sys::jint,
    buffer_y: JByteBuffer<'local>,
    buffer_u: JByteBuffer<'local>,
    buffer_v: JByteBuffer<'local>,
) -> Result<()> {
    let width = width as usize;
    let height = height as usize;

    fn buffer_as_slice<'local>(
        env: &jni::JNIEnv<'local>,
        buffer: &JByteBuffer<'local>,
        size: usize,
    ) -> Result<&'local [u8]> {
        let buffer_cap = match env.get_direct_buffer_capacity(&buffer) {
            Ok(cap) => cap,
            Err(err) => {
                bail!("Failed to get capacity of the byte buffer: {err}");
            }
        };

        if buffer_cap < size {
            bail!("buffer_cap < size: {buffer_cap} < {size}");
        }

        let buffer_ptr = match env.get_direct_buffer_address(&buffer) {
            Ok(ptr) => {
                assert!(!ptr.is_null());
                ptr
            }
            Err(err) => {
                bail!("Failed to get buffer address: {err}");
            }
        };

        unsafe { Ok(std::slice::from_raw_parts(buffer_ptr, buffer_cap)) }
    }

    let slice_y = buffer_as_slice(&env, &buffer_y, width * height)?;
    let slice_u = buffer_as_slice(&env, &buffer_u, (width / 2) * (height / 2))?;
    let slice_v = buffer_as_slice(&env, &buffer_v, (width / 2) * (height / 2))?;

    let info = match gst_video::VideoInfo::builder(
        gst_video::VideoFormat::I420,
        width as u32,
        height as u32,
    )
    .colorimetry(&VideoColorimetry::new(
        gst_video::VideoColorRange::Range0_255,
        gst_video::VideoColorMatrix::Bt709,
        gst_video::VideoTransferFunction::Bt709,
        gst_video::VideoColorPrimaries::Bt709,
    ))
    .build()
    {
        Ok(info) => info,
        Err(err) => {
            bail!("Failed to crate video info: {err}");
        }
    };

    let new_caps = match info.to_caps() {
        Ok(caps) => caps,
        Err(err) => {
            bail!("Failed to create caps from video info: {err}");
        }
    };

    fn init_frame_pool(
        pool: &gst_video::VideoBufferPool,
        mut old_config: gst::BufferPoolConfig,
        new_caps: &gst::Caps,
        frame_size: u32,
    ) -> Result<()> {
        pool.set_config({
            old_config.set_params(Some(&new_caps), frame_size, 1, 30);
            old_config
        })?;
        pool.set_active(true)?;
        Ok(())
    }

    let mut frame_pool = FRAME_POOL.lock();
    let frame_size = width * height + 2 * ((width / 2) * (height / 2));
    let needs_reconfigure = if !frame_pool.is_active() {
        true
    } else {
        match frame_pool.config().params() {
            Some((caps, size, _, _)) => {
                caps.as_ref() != Some(&new_caps) || size != frame_size as u32
            }
            None => true,
        }
    };
    if needs_reconfigure {
        let old_config = frame_pool.config();
        if frame_pool.is_active() {
            let _ = frame_pool.set_active(false);
        }
        init_frame_pool(&frame_pool, old_config, &new_caps, frame_size as u32)?;
    }

    let buffer = match frame_pool.acquire_buffer(None) {
        Ok(buffer) => buffer,
        Err(err) => {
            bail!("Failed to acquire buffer from pool: {err}");
        }
    };
    let Ok(mut vframe) = gst_video::VideoFrame::from_buffer_writable(buffer, &info) else {
        bail!("Failed to crate VideoFrame from buffer");
    };

    fn copy(
        vframe: &mut gst_video::VideoFrame<gst_video::video_frame::Writable>,
        plane_idx: u32,
        src_plane: &[u8],
    ) -> Result<()> {
        let dest_y_stride = *vframe
            .plane_stride()
            .get(plane_idx as usize)
            .ok_or(anyhow::anyhow!("Could not get plane stride"))?
            as usize;
        let dest_y = vframe.plane_data_mut(plane_idx)?;
        for (dest, src) in dest_y
            .chunks_exact_mut(dest_y_stride)
            .zip(src_plane.chunks_exact(dest_y_stride))
        {
            dest[..dest_y_stride].copy_from_slice(&src[..dest_y_stride]);
        }

        Ok(())
    }

    copy(&mut vframe, 0, slice_y)?;
    copy(&mut vframe, 1, slice_u)?;
    copy(&mut vframe, 2, slice_v)?;

    let (lock, cvar) = &*FRAME_PAIR;
    let mut frame = lock.lock();
    *frame = Some(vframe);
    cvar.notify_one();

    Ok(())
}

#[cfg(target_os = "android")]
#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "C" fn Java_org_fcast_android_sender_MainActivity_nativeProcessFrame<'local>(
    env: jni::JNIEnv<'local>,
    _class: jni::objects::JClass<'local>,
    width: jni::sys::jint,
    height: jni::sys::jint,
    buffer_y: JByteBuffer<'local>,
    buffer_u: JByteBuffer<'local>,
    buffer_v: JByteBuffer<'local>,
) {
    if let Err(err) = process_frame(env, width, height, buffer_y, buffer_u, buffer_v) {
        error!(?err, "Failed to process frame");
    }
}

#[cfg(target_os = "android")]
#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "C" fn Java_org_fcast_android_sender_MainActivity_nativeQrScanResult<'local>(
    mut env: jni::JNIEnv<'local>,
    _class: jni::objects::JClass<'local>,
    result: jni::objects::JString<'local>,
) {
    match jstring_to_string(&mut env, &result) {
        Ok(result) => log_err!(
            GLOB_EVENT_CHAN.0.send(Event::QrScanResult(result)),
            "Failed to send device removed event"
        ),
        Err(err) => error!(?err, "Failed to convert jstring to string"),
    }
}
