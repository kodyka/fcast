use anyhow::Result;
use fcast_sender_sdk::{context::CastContext, device, device::DeviceInfo};
use mcore::{DeviceEvent, Event, ShouldQuit};
#[cfg(not(target_os = "android"))]
use mcore::transmission::WhepSink;
use parking_lot::{Condvar, Mutex};
#[cfg(target_os = "android")]
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc};
use std::sync::atomic::AtomicU64;
#[cfg(target_os = "android")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(not(target_os = "android"))]
use std::sync::atomic::Ordering;
use tracing::{debug, error};
#[cfg(target_os = "android")]
use tracing::{info, warn};
#[cfg(target_os = "android")]
use anyhow::bail;
#[cfg(target_os = "android")]
use gst::prelude::{BufferPoolExt, BufferPoolExtManual};
#[cfg(target_os = "android")]
use gst_video::{VideoColorimetry, VideoFrameExt};
#[cfg(target_os = "android")]
use jni::{objects::{JByteBuffer, JObject, JString}, JavaVM};
#[cfg(target_os = "android")]
use std::net::Ipv6Addr;

pub mod log_ring;

pub mod migration;
mod whep_signaller_compat;

#[derive(Default)]
struct RecordingTickerState {
    state: RecordingState,
    started_at: Option<std::time::Instant>,
    paused_for: std::time::Duration,
    pause_started: Option<std::time::Instant>,
}

fn spawn_recording_ticker(
    ui_handle: slint::Weak<MainWindow>,
    state: Arc<tokio::sync::Mutex<RecordingTickerState>>,
) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let s = state.lock().await;
            if s.state == RecordingState::Recording {
                if let Some(started) = s.started_at {
                    let elapsed = started.elapsed().saturating_sub(s.paused_for).as_secs() as i32;
                    let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                        ui.global::<Bridge>().set_recording_elapsed_s(elapsed);
                    });
                }
            }
        }
    });
}

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

#[cfg(target_os = "android")]
const CAST_SOURCE_ID: &str = "cast-screen-1";
#[cfg(target_os = "android")]
const CAST_DESTINATION_ID: &str = "cast-whep-1";
#[cfg(target_os = "android")]
const CAST_LINK_ID: &str = "cast-link-1";

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

fn migration_test_log_name(test_id: &str) -> &'static str {
    match test_id {
        "getinfo" => "legacy-getinfo",
        "crossfade" => "legacy-crossfade",
        "smoke" => "graph-smoke",
        _ => "unknown",
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
    #[cfg(not(target_os = "android"))]
    tx_sink: Option<WhepSink>,
    our_source_url: Option<String>,
    #[cfg(target_os = "android")]
    last_cast_request_scale_width: Option<u32>,
    #[cfg(target_os = "android")]
    last_cast_request_scale_height: Option<u32>,
    #[cfg(target_os = "android")]
    last_cast_request_max_framerate: Option<u32>,
}


// Phase 8 (deferred): producer of Bridge.status-items. Currently unused —
// CastingView renders mock-status-items inline. Keep this helper so the
// Rust side of Phase 8 is a one-line wire-up.
#[allow(dead_code)]
fn build_status_items(receiver_name: &str, encoder: &str, network: &str) -> Vec<crate::StatusItem> {
    vec![
        crate::StatusItem {
            label: "Receiver".into(),
            value: receiver_name.into(),
            severity: crate::StatusSeverity::Info,
            icon_glyph: "📺".into(),
        },
        crate::StatusItem {
            label: "Encoder".into(),
            value: encoder.into(),
            severity: crate::StatusSeverity::Info,
            icon_glyph: "⚙️".into(),
        },
        crate::StatusItem {
            label: "Network".into(),
            value: network.into(),
            severity: crate::StatusSeverity::Info,
            icon_glyph: "📶".into(),
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
            #[cfg(not(target_os = "android"))]
            tx_sink: None,
            our_source_url: None,
            #[cfg(target_os = "android")]
            last_cast_request_scale_width: None,
            #[cfg(target_os = "android")]
            last_cast_request_scale_height: None,
            #[cfg(target_os = "android")]
            last_cast_request_max_framerate: None,
        })
    }

    // Helper for any callback that needs to flash a banner. Centralised so we
    // only own one upgrade-on-event-loop pattern.
    //
    // A monotonic generation counter is bumped on every `set_banner` /
    // `clear_banner` call. `flash_banner` captures the generation it
    // installed and its spawned auto-hide task only clears the banner if
    // that generation is still current — otherwise a newer banner has
    // taken over and the old timer is a no-op. This avoids an earlier
    // flash_banner racing with a later one and hiding it prematurely.
    fn banner_generation() -> &'static AtomicU64 {
        static GEN: AtomicU64 = AtomicU64::new(0);
        &GEN
    }

    fn set_banner(
        ui_handle: slint::Weak<MainWindow>,
        msg: String,
        severity: BannerSeverity,
    ) -> u64 {
        let gen = Self::banner_generation().fetch_add(1, Ordering::SeqCst) + 1;
        let _ = ui_handle.upgrade_in_event_loop(move |ui| {
            let bridge = ui.global::<Bridge>();
            bridge.set_banner_message(msg.into());
            bridge.set_banner_severity(severity);
            bridge.set_banner_visible(true);
        });
        gen
    }

    fn clear_banner(ui_handle: slint::Weak<MainWindow>) {
        Self::banner_generation().fetch_add(1, Ordering::SeqCst);
        let _ = ui_handle.upgrade_in_event_loop(move |ui| {
            ui.global::<Bridge>().set_banner_visible(false);
        });
    }

    fn flash_banner(
        ui_handle: slint::Weak<MainWindow>,
        msg: String,
        severity: BannerSeverity,
        duration: std::time::Duration,
    ) {
        let gen = Self::set_banner(ui_handle.clone(), msg, severity);
        tokio::spawn(async move {
            tokio::time::sleep(duration).await;
            // Only clear if no newer set_banner / clear_banner has run.
            if Self::banner_generation().load(Ordering::SeqCst) == gen {
                Self::clear_banner(ui_handle);
            }
        });
    }

    fn update_receivers_in_ui(&mut self) -> Result<()> {
        let receivers = self
            .devices
            .iter()
            .filter(|(_, info)| !info.addresses.is_empty() && info.port != 0)
            .map(|(name, info)| {
                let first_addr = info
                    .addresses
                    .first()
                    .map(|addr| addr.to_string())
                    .unwrap_or_default();
                let address = match info.addresses.first() {
                    Some(fcast_sender_sdk::IpAddr::V6 { .. }) => {
                        format!("[{first_addr}]:{}", info.port)
                    }
                    Some(_) => format!("{first_addr}:{}", info.port),
                    None => String::new(),
                };
                let kind = match info.protocol {
                    device::ProtocolType::FCast => "fcast",
                };

                ReceiverItem {
                    id: name.clone().into(),
                    name: name.clone().into(),
                    address: address.into(),
                    ip: first_addr.into(),
                    port: i32::from(info.port),
                    kind: kind.into(),
                    is_default: false,
                }
            })
            .collect::<Vec<ReceiverItem>>();
        self.ui_weak.upgrade_in_event_loop(move |ui| {
            let model =
                std::rc::Rc::new(slint::VecModel::<ReceiverItem>::from_iter(receivers.into_iter()));
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
        #[cfg(target_os = "android")]
        set_capture_active(false);

        let android_app = self.android_app.clone();
        self.ui_weak.upgrade_in_event_loop(move |_| {
            call_java_method_no_args(&android_app, JavaMethod::StopCapture);
        })?;

        self.our_source_url = None;

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

        #[cfg(target_os = "android")]
        {
            let _ = crate::migration::runtime::handle_command(crate::migration::Command::Disconnect {
                link_id: CAST_LINK_ID.into(),
            });
            for id in [CAST_SOURCE_ID, CAST_DESTINATION_ID] {
                let _ = crate::migration::runtime::handle_command(crate::migration::Command::Remove {
                    id: id.into(),
                });
            }
        }

        #[cfg(not(target_os = "android"))]
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
                    // Phase 8 (deferred): clear Bridge.status-items here.
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

                let (content_type, url) =
                    mcore::transmission::build_whep_play_msg(addr.into(), bound_port);

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
                            if let device::DeviceConnectionState::Connected { local_addr, .. } = device_connection_state {
                                self.local_address = Some(local_addr);

                                self.ui_weak.upgrade_in_event_loop(|ui| {
                                    ui.global::<Bridge>()
                                        .invoke_change_state(AppState::SelectingSettings);
                                })?;
                            }
                        }
                        DeviceEvent::SourceChanged(new_source) => {
                            #[cfg(target_os = "android")]
                            let should_monitor_source = self.our_source_url.is_some();
                            #[cfg(not(target_os = "android"))]
                            let should_monitor_source = self.tx_sink.is_some();

                            if should_monitor_source {
                                if let fcast_sender_sdk::device::Source::Url { ref url, .. } = new_source {
                                    if Some(url) != self.our_source_url.as_ref() {
                                        // At this point the receiver has stopped playing our stream
                                        debug!(
                                            ?new_source,
                                            "The source on the receiver changed, disconnecting"
                                        );

                                        self.ui_weak.upgrade_in_event_loop(|ui| {
                                            // Phase 8 (deferred): clear Bridge.status-items here.
                                            ui.global::<Bridge>()
                                                .invoke_change_state(AppState::Disconnected);
                                        })?;

                                        self.stop_cast(false).await?;
                                    }
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
                    // Phase 8 (deferred): clear Bridge.status-items here.
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
                self.our_source_url = None;

                if let Err(err) = crate::migration::runtime::start_graph_runtime() {
                    error!(?err, "Failed to start migrated graph runtime");
                    self.stop_cast(false).await?;
                    return Ok(ShouldQuit::No);
                }

                let scale_width = self.last_cast_request_scale_width.unwrap_or(1280);
                let scale_height = self.last_cast_request_scale_height.unwrap_or(720);
                let fps = self.last_cast_request_max_framerate.unwrap_or(30);

                let commands = [
                    crate::migration::Command::CreateScreenCaptureSource {
                        id: CAST_SOURCE_ID.into(),
                        width: scale_width,
                        height: scale_height,
                        fps,
                    },
                    crate::migration::Command::CreateDestination {
                        id: CAST_DESTINATION_ID.into(),
                        family: crate::migration::DestinationFamily::Whep { server_port: 0 },
                        audio: false,
                        video: true,
                    },
                    crate::migration::Command::Connect {
                        link_id: CAST_LINK_ID.into(),
                        src_id: CAST_SOURCE_ID.into(),
                        sink_id: CAST_DESTINATION_ID.into(),
                        audio: false,
                        video: true,
                        config: None,
                    },
                    crate::migration::Command::Start {
                        id: CAST_DESTINATION_ID.into(),
                        cue_time: None,
                        end_time: None,
                    },
                    crate::migration::Command::Start {
                        id: CAST_SOURCE_ID.into(),
                        cue_time: None,
                        end_time: None,
                    },
                ];

                for command in commands {
                    if let crate::migration::CommandResult::Error(err) =
                        crate::migration::runtime::handle_command(command)
                    {
                        error!(?err, "Failed to build unified cast graph");
                        self.stop_cast(false).await?;
                        return Ok(ShouldQuit::No);
                    }
                }

                let event_tx = self.event_tx.clone();
                tokio::spawn(async move {
                    for _ in 0..200 {
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        let info = crate::migration::runtime::handle_command(
                            crate::migration::Command::GetInfo {
                                id: Some(CAST_DESTINATION_ID.into()),
                            },
                        );

                        if let crate::migration::CommandResult::Info(snapshot) = info {
                            if let Some(crate::migration::NodeInfo::Destination(destination)) =
                                snapshot.nodes.get(CAST_DESTINATION_ID)
                            {
                                if let (Some(bound_port_v4), Some(bound_port_v6)) =
                                    (destination.bound_port_v4, destination.bound_port_v6)
                                {
                                    let _ = event_tx.send(Event::SignallerStarted {
                                        bound_port_v4,
                                        bound_port_v6,
                                    });
                                    return;
                                }
                            }
                        }
                    }

                    error!("WHEP destination did not publish bound ports within 20s");
                });

                let _receiver_name = self.active_device.as_ref().map(|d| d.name()).unwrap_or_default();
                let _encoder_name = "Hardware"; // Blocked by P0-1: Placeholder until encoder selection works
                let _network_info = self.local_address.as_ref().map(|a| a.to_string()).unwrap_or_default();
                // Phase 8 (deferred): wire Bridge.status-items here from
                // build_status_items(&_receiver_name, _encoder_name, &_network_info).

                self.ui_weak.upgrade_in_event_loop(move |ui| {
                    ui.global::<Bridge>().invoke_change_state(AppState::Casting);
                })?;
            }
            #[cfg(target_os = "android")]
            Event::StartCast {
                scale_width,
                scale_height,
                max_framerate,
            } => {
                self.last_cast_request_scale_width = Some(scale_width);
                self.last_cast_request_scale_height = Some(scale_height);
                self.last_cast_request_max_framerate = Some(max_framerate);

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
        // PHASE-9: start the migration runtime on demand at its call sites.
        // `shutdown_graph_runtime()` below remains safe if nothing started it.

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

fn default_presets() -> Vec<crate::BitratePreset> {
    vec![
        crate::BitratePreset { id: "low".into(),  name: "Low".into(),     bitrate_kbps: 1500,  active: false },
        crate::BitratePreset { id: "med".into(),  name: "Medium".into(),  bitrate_kbps: 4000,  active: true  },
        crate::BitratePreset { id: "high".into(), name: "High".into(),    bitrate_kbps: 8000,  active: false },
        crate::BitratePreset { id: "max".into(),  name: "Maximum".into(), bitrate_kbps: 15000, active: false },
    ]
}

fn default_quick_actions() -> Vec<crate::QuickAction> {
    let mut actions = vec![
        crate::QuickAction { id: "settings".into(),   title: "Settings".into(),    enabled: true,  active: false, is_macro: false },
        crate::QuickAction { id: "debug".into(),      title: "Debug".into(),       enabled: true,  active: false, is_macro: false },
        crate::QuickAction { id: "codec-test".into(), title: "Codec test".into(),  enabled: true,  active: false, is_macro: false },
        crate::QuickAction { id: "scan-qr".into(),    title: "Scan QR".into(),     enabled: true,  active: false, is_macro: false },
        crate::QuickAction { id: "record".into(),     title: "Record".into(),      enabled: true,  active: false, is_macro: false },
        crate::QuickAction { id: "pair".into(),       title: "Pair".into(),        enabled: true,  active: false, is_macro: false },
        crate::QuickAction { id: "bitrate".into(),    title: "Bitrate".into(),     enabled: true,  active: false, is_macro: false },
    ];
    if cfg!(debug_assertions) {
        actions.extend([
            crate::QuickAction { id: "migrated-server".into(), title: "Migrated srv".into(), enabled: true, active: false, is_macro: false },
            crate::QuickAction { id: "test-getinfo".into(),    title: "GetInfo".into(),      enabled: true, active: false, is_macro: false },
            crate::QuickAction { id: "test-crossfade".into(),  title: "Crossfade".into(),    enabled: true, active: false, is_macro: false },
            crate::QuickAction { id: "test-smoke".into(),      title: "Smoke Graph".into(),  enabled: true, active: false, is_macro: false },
        ]);
    }
    actions
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

    // Cached snapshot. Re-pushed in full whenever any signal changes.
    #[derive(Clone, Default)]
    struct StatusSnapshot {
        network_label: String,
        thermal_label: String,
        battery_pct: i32,
        charging: bool,
    }

    fn push_status(ui_handle: slint::Weak<MainWindow>, snap: StatusSnapshot) {
        let _ = ui_handle.upgrade_in_event_loop(move |ui| {
            let bridge = ui.global::<Bridge>();
            let items = vec![
                StatusItem {
                    label: "network".into(),
                    value: snap.network_label.into(),
                    severity: StatusSeverity::Info,
                    icon_glyph: "📶".into(),
                },
                StatusItem {
                    label: "thermal".into(),
                    value: snap.thermal_label.clone().into(),
                    severity: match snap.thermal_label.as_str() {
                        "Critical" => StatusSeverity::Error,
                        "Serious" => StatusSeverity::Warning,
                        _ => StatusSeverity::Info,
                    },
                    icon_glyph: if snap.thermal_label == "Critical" { "🔥".into() } else { "🌡".into() },
                },
                StatusItem {
                    label: "battery".into(),
                    value: format!("{}%", snap.battery_pct).into(),
                    severity: if snap.battery_pct < 20 { StatusSeverity::Error } else { StatusSeverity::Info },
                    icon_glyph: if snap.charging { "⚡".into() } else { "🔋".into() },
                },
            ];
            let model: slint::ModelRc<StatusItem> = std::rc::Rc::new(slint::VecModel::from(items)).into();
            bridge.set_status_items(model);
        });
    }

    let show_debug = cfg!(debug_assertions);
    ui.global::<Bridge>().set_show_debug(show_debug);
    ui.global::<Bridge>()
        .set_app_version(env!("CARGO_PKG_VERSION").into());

    let bar_actions: Arc<Mutex<Vec<QuickAction>>> =
        Arc::new(Mutex::new(default_quick_actions()));
    // Initial push is synchronous — we still hold the strong `ui` handle,
    // so the control bar is populated before `ui.run()` paints the first
    // frame. Subsequent mutations from callbacks use `push_bar()` which
    // hops through `upgrade_in_event_loop` because they only have a weak.
    {
        let snapshot = bar_actions.lock().clone();
        ui.global::<Bridge>().set_quick_actions(
            std::rc::Rc::new(slint::VecModel::from(snapshot)).into(),
        );
    }
    let push_bar = {
        let bar_actions = bar_actions.clone();
        let ui_weak = ui.as_weak();
        move || {
            let snapshot = bar_actions.lock().clone();
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                ui.global::<Bridge>().set_quick_actions(
                    std::rc::Rc::new(slint::VecModel::from(snapshot)).into(),
                );
            });
        }
    };

    ui.global::<Bridge>().on_move_bar_action({
        let bar_actions = bar_actions.clone();
        let push        = push_bar.clone();
        move |from, to| {
            let mut g = bar_actions.lock();
            if let (Ok(from_u), Ok(to_u)) = (usize::try_from(from), usize::try_from(to)) {
                if from_u < g.len() && to_u < g.len() && from_u != to_u {
                    let item = g.remove(from_u);
                    g.insert(to_u, item);
                }
            }
            drop(g);
            push();
        }
    });

    ui.global::<Bridge>().on_set_bar_action_enabled({
        let bar_actions = bar_actions.clone();
        let push        = push_bar.clone();
        move |idx, enabled| {
            let mut g = bar_actions.lock();
            if let Ok(i) = usize::try_from(idx) {
                if let Some(a) = g.get_mut(i) { a.enabled = enabled; }
            }
            drop(g);
            push();
        }
    });

    ui.global::<Bridge>().on_save_bar_actions({
        let _bar_actions = bar_actions.clone();
        let push        = push_bar.clone();
        move || {
            // Phase 11: persist to DataStore via JNI here.
            // For now, just re-push the in-memory state.
            push();
        }
    });

    let history: Arc<Mutex<Vec<crate::CastHistoryEntry>>> = Arc::new(Mutex::new(vec![
        crate::CastHistoryEntry {
            id: "h1".into(), receiver: "Living Room TV".into(),
            started_at: "Today 12:34".into(), duration_s: 765,
            status: "Completed".into(),
        },
        crate::CastHistoryEntry {
            id: "h2".into(), receiver: "Bedroom TV".into(),
            started_at: "Yesterday 22:10".into(), duration_s: 68,
            status: "Cancelled".into(),
        },
        crate::CastHistoryEntry {
            id: "h3".into(), receiver: "Office Mac".into(),
            started_at: "Yesterday 09:00".into(), duration_s: 1920,
            status: "Completed".into(),
        },
    ]));

    use slint::Model;
    use std::sync::atomic::AtomicUsize;
    let macros: Arc<std::sync::Mutex<Vec<Macro>>> = Arc::new(std::sync::Mutex::new(vec![]));
    let draft_macro_steps: Arc<std::sync::Mutex<Vec<MacroStep>>> = Arc::new(std::sync::Mutex::new(vec![]));
    let next_macro_id = Arc::new(AtomicUsize::new(0));

    // Both push_* helpers apply synchronously via `upgrade()` rather than
    // `upgrade_in_event_loop()`. Every caller is a Slint callback (e.g.
    // on_save_macro, on_draft_move_step) that already runs on the UI
    // thread, so deferral is unnecessary — and would let the consumer
    // page render one frame with stale data when a panel switch happens
    // immediately after the callback (see on_load_draft_macro for the
    // same rationale).
    let push_macros = {
        let macros = macros.clone();
        let ui_weak = ui.as_weak();
        move || {
            let snap = macros.lock().unwrap().clone();
            if let Some(ui) = ui_weak.upgrade() {
                ui.global::<Bridge>().set_macros(
                    std::rc::Rc::new(slint::VecModel::from(snap)).into(),
                );
            }
        }
    };
    push_macros();

    let push_draft_steps = {
        let draft_macro_steps = draft_macro_steps.clone();
        let ui_weak = ui.as_weak();
        move || {
            let snap = draft_macro_steps.lock().unwrap().clone();
            if let Some(ui) = ui_weak.upgrade() {
                ui.global::<Bridge>().set_draft_macro_steps(
                    std::rc::Rc::new(slint::VecModel::from(snap)).into(),
                );
            }
        }
    };
    push_draft_steps();

    ui.global::<Bridge>().on_save_macro({
        let macros  = macros.clone();
        let next_id = next_macro_id.clone();
        let push    = push_macros.clone();
        move |id, name, steps, enabled| {
            let steps_vec: Vec<MacroStep> = steps.iter().collect();
            let mut g = macros.lock().unwrap();
            if id.is_empty() {
                let new_id = format!("macro-{}", next_id.fetch_add(1, Ordering::Relaxed));
                g.push(Macro {
                    id:      new_id.into(),
                    name:    name.into(),
                    steps:   std::rc::Rc::new(slint::VecModel::from(steps_vec)).into(),
                    enabled,
                });
            } else if let Some(m) = g.iter_mut().find(|m| m.id == id) {
                m.name    = name.into();
                m.enabled = enabled;
                m.steps   = std::rc::Rc::new(slint::VecModel::from(steps_vec)).into();
            }
            drop(g);
            push();
        }
    });

    ui.global::<Bridge>().on_delete_macro({
        let macros = macros.clone();
        let push   = push_macros.clone();
        move |id| {
            macros.lock().unwrap().retain(|m| m.id != id);
            push();
        }
    });

    ui.global::<Bridge>().on_run_macro({
        let macros = macros.clone();
        let ui_weak = ui.as_weak();
        move |id| {
            let snap = macros.lock().unwrap().iter()
                .find(|m| m.id == id).cloned();
            let Some(m) = snap else {
                Application::flash_banner(
                    ui_weak.clone(),
                    format!("Macro {} not found", id),
                    BannerSeverity::Error,
                    std::time::Duration::from_secs(3),
                );
                return;
            };
            // Phase 11: real macro engine (iterate m.steps, dispatch each via on_invoke_action).
            Application::flash_banner(
                ui_weak.clone(),
                format!("Ran macro: {}", m.name),
                BannerSeverity::Success,
                std::time::Duration::from_secs(2),
            );
        }
    });

    ui.global::<Bridge>().on_load_draft_macro({
        let macros = macros.clone();
        let draft_macro_steps = draft_macro_steps.clone();
        let ui_weak = ui.as_weak();
        move |id| {
            let mut draft_name = "".to_string();
            let mut draft_enabled = true;
            let steps_snap: Vec<MacroStep> = {
                let mut draft_g = draft_macro_steps.lock().unwrap();
                if id.is_empty() {
                    draft_g.clear();
                } else {
                    let mg = macros.lock().unwrap();
                    if let Some(m) = mg.iter().find(|m| m.id == id) {
                        *draft_g = m.steps.iter().collect();
                        draft_name = m.name.to_string();
                        draft_enabled = m.enabled;
                    } else {
                        draft_g.clear();
                    }
                }
                draft_g.clone()
            };
            // Slint callbacks run on the UI thread, so we can apply the
            // draft state synchronously. This matters because callers
            // (macros_page.slint) switch to Panel.macro-edit immediately
            // after this callback returns — a deferred upgrade_in_event_loop
            // would let MacroEditPage render one frame with stale values.
            if let Some(ui) = ui_weak.upgrade() {
                let bridge = ui.global::<Bridge>();
                bridge.set_draft_macro_name(draft_name.into());
                bridge.set_draft_macro_enabled(draft_enabled);
                bridge.set_draft_macro_steps(
                    std::rc::Rc::new(slint::VecModel::from(steps_snap)).into(),
                );
            }
        }
    });

    ui.global::<Bridge>().on_draft_add_step({
        let draft_macro_steps = draft_macro_steps.clone();
        let push = push_draft_steps.clone();
        move |action_id| {
            let mut g = draft_macro_steps.lock().unwrap();
            let label = match action_id.as_str() {
                "scan-qr" => "Scan QR",
                "audio" => "Open Audio",
                "camera" => "Open Camera",
                "record" => "Start Recording",
                "stop-recording" => "Stop Recording",
                "stop-cast" => "Stop Cast",
                _ => action_id.as_str(),
            };
            g.push(MacroStep {
                action_id: action_id.clone(),
                label: label.into(),
            });
            drop(g);
            push();
        }
    });

    ui.global::<Bridge>().on_draft_remove_step({
        let draft_macro_steps = draft_macro_steps.clone();
        let push = push_draft_steps.clone();
        move |idx| {
            let mut g = draft_macro_steps.lock().unwrap();
            if let Ok(i) = usize::try_from(idx) {
                if i < g.len() {
                    g.remove(i);
                }
            }
            drop(g);
            push();
        }
    });

    ui.global::<Bridge>().on_draft_move_step({
        let draft_macro_steps = draft_macro_steps.clone();
        let push = push_draft_steps.clone();
        move |from, to| {
            let mut g = draft_macro_steps.lock().unwrap();
            if let (Ok(from_u), Ok(to_u)) = (usize::try_from(from), usize::try_from(to)) {
                if from_u < g.len() && to_u < g.len() && from_u != to_u {
                    let s = g.remove(from_u);
                    g.insert(to_u, s);
                }
            }
            drop(g);
            push();
        }
    });

    let push_history = {
        let history = history.clone();
        let ui_weak = ui.as_weak();
        move || {
            let snap = history.lock().clone();
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                ui.global::<Bridge>().set_history(
                    std::rc::Rc::new(slint::VecModel::from(snap)).into(),
                );
            });
        }
    };
    push_history();

    // ── Bitrate presets (Phase 8 / Cluster C1) — data + pusher ──────────
    // Created here (above the D1 handlers) so `on_reset_settings` can
    // restore the factory presets list. The C1 callback registrations
    // (save / delete / set-active) live further down and capture these
    // same handles by clone.
    //
    // The factory-default literal lives in `default_presets()` so init
    // and reset share a single source of truth — same pattern as
    // `default_quick_actions()`.
    let presets: Arc<Mutex<Vec<BitratePreset>>> = Arc::new(Mutex::new(default_presets()));
    // Monotonic id source for user-created presets. Never use `g.len()`:
    // after a delete-then-add cycle len() can collide with a previously
    // issued id.
    let next_preset_id: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));
    let push_presets = {
        let presets = presets.clone();
        let ui_weak = ui.as_weak();
        move || {
            let snapshot = presets.lock().clone();
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                ui.global::<Bridge>().set_presets(
                    std::rc::Rc::new(slint::VecModel::from(snapshot)).into(),
                );
            });
        }
    };
    push_presets();

    // Create the tokio runtime *before* registering Slint callbacks that
    // call `tokio::spawn` (directly or via `Application::flash_banner`).
    // Slint callbacks run on the UI thread during `ui.run()`, which has no
    // tokio context by default — `tokio::spawn` would panic with "there is
    // no reactor running". The `_runtime_guard` registers this thread as a
    // runtime context for the lifetime of the guard. It MUST be dropped
    // before `runtime.block_on(...)` later in this function, otherwise
    // `block_on` panics ("Cannot start a runtime from within a runtime").
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _runtime_guard = runtime.enter();

    // ── Phase 8 / Cluster D1 — Backup / reset handlers ──────────────────
    ui.global::<Bridge>().on_export_settings({
        let ui_weak = ui.as_weak();
        move || {
            let ui_weak = ui_weak.clone();
            tokio::spawn(async move {
                // Phase 11: ACTION_CREATE_DOCUMENT via JNI; serialise + write.
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                Application::flash_banner(
                    ui_weak,
                    "Settings exported (placeholder).".into(),
                    BannerSeverity::Success,
                    std::time::Duration::from_secs(3),
                );
            });
        }
    });

    ui.global::<Bridge>().on_import_settings({
        let ui_weak = ui.as_weak();
        move || {
            let ui_weak = ui_weak.clone();
            tokio::spawn(async move {
                // Phase 11: ACTION_OPEN_DOCUMENT, parse JSON, write to DataStore.
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                Application::flash_banner(
                    ui_weak,
                    "Settings imported (placeholder).".into(),
                    BannerSeverity::Success,
                    std::time::Duration::from_secs(3),
                );
            });
        }
    });

    ui.global::<Bridge>().on_reset_settings({
        let bar_actions    = bar_actions.clone();
        let history        = history.clone();
        let presets        = presets.clone();
        let next_preset_id = next_preset_id.clone();
        let macros         = macros.clone();
        let next_macro_id  = next_macro_id.clone();
        let push_bar       = push_bar.clone();
        let push_history   = push_history.clone();
        let push_presets   = push_presets.clone();
        let push_macros    = push_macros.clone();
        let ui_weak        = ui.as_weak();
        move || {
            // Reset every Cluster-C/D model owned by Rust to factory
            // defaults.
            //
            // `next_preset_id` / `next_macro_id` are also rewound so
            // user-created ids restart at `custom-0` / `macro-0` after
            // a reset, matching the factory state. Without this, a
            // freshly-reset device would still hand out `custom-N` /
            // `macro-N` for some N > 0 the moment the user added an
            // entry.
            *bar_actions.lock() = default_quick_actions();
            *presets.lock()     = default_presets();
            next_preset_id.store(0, Ordering::Relaxed);
            history.lock().clear();
            macros.lock().unwrap().clear();
            next_macro_id.store(0, Ordering::Relaxed);

            push_bar();
            push_presets();
            push_history();
            push_macros();

            // Phase 11: also clear DataStore / SharedPreferences via JNI.

            Application::flash_banner(
                ui_weak.clone(),
                "Settings reset to defaults".into(),
                BannerSeverity::Success,
                std::time::Duration::from_secs(3),
            );
        }
    });

    ui.global::<Bridge>().on_clear_cast_history({
        let history      = history.clone();
        let push_history = push_history.clone();
        let ui_weak      = ui.as_weak();
        move || {
            history.lock().clear();
            push_history();

            Application::flash_banner(
                ui_weak.clone(),
                "Cast history cleared".into(),
                BannerSeverity::Success,
                std::time::Duration::from_secs(2),
            );
        }
    });

    ui.global::<Bridge>().on_clear_known_receivers({
        let ui_weak = ui.as_weak();
        move || {
            // Phase 11: clear known-receivers DataStore. For now, announce.
            Application::flash_banner(
                ui_weak.clone(),
                "Known receivers cleared".into(),
                BannerSeverity::Success,
                std::time::Duration::from_secs(2),
            );
        }
    });

    // ── Phase 8 / Cluster D2 — Cast history handlers ────────────────────
    ui.global::<Bridge>().on_clear_history({
        let history      = history.clone();
        let push_history = push_history.clone();
        move || {
            history.lock().clear();
            push_history();
        }
    });

    ui.global::<Bridge>().on_delete_history_entry({
        let history      = history.clone();
        let push_history = push_history.clone();
        move |id| {
            let id = id.to_string();
            history.lock().retain(|e| e.id != id);
            push_history();
        }
    });

    ui.global::<Bridge>().on_recast({
        let history = history.clone();
        let ui_weak = ui.as_weak();
        move |id| {
            let id = id.to_string();
            let entry_opt = history.lock().iter()
                .find(|e| e.id == id).cloned();
            let Some(entry) = entry_opt else { return; };
            // Phase 11: trigger reconnection + start_casting with the same receiver.
            Application::flash_banner(
                ui_weak.clone(),
                format!("Recasting to {}", entry.receiver),
                BannerSeverity::Info,
                std::time::Duration::from_secs(2),
            );
        }
    });

    // Push selected-history-entry whenever the id changes. Slint does NOT
    // auto-generate on_<property>_changed; the `changed` handler in
    // bridge.slint re-emits via the explicit `selected-history-id-changed`
    // callback bound below.
    //
    // The callback fires on the Slint UI thread, so we use `ui_weak.upgrade()`
    // (synchronous) instead of `upgrade_in_event_loop` (deferred). The
    // consumer page sets `Bridge.active-panel = Panel.cast-history-detail`
    // immediately after setting the id; a deferred property write would
    // render the detail page one frame with stale/empty data.
    ui.global::<Bridge>().on_selected_history_id_changed({
        let history = history.clone();
        let ui_weak = ui.as_weak();
        move |id: slint::SharedString| {
            let id = id.to_string();
            let entry = history.lock().iter()
                .find(|e| e.id == id).cloned();
            let Some(entry) = entry else { return; };
            if let Some(ui) = ui_weak.upgrade() {
                ui.global::<Bridge>().set_selected_history_entry(entry);
            }
        }
    });

    let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();

    let ui_handle = ui.as_weak();
    // NOTE: The shared `Arc<Mutex<StatusSnapshot>>` cache that producers
    // (battery / thermal / network listeners) will update lands with
    // Cluster B (Phase 8 Section 3). For now the ticker just rebuilds a
    // hardcoded snapshot on every tick — no shared state needed yet.
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(5));
        loop {
            tick.tick().await;
            let snap_now = StatusSnapshot {
                network_label: "Wi-Fi".into(),
                thermal_label: "Nominal".into(),
                battery_pct: 87,
                charging: false,
            };
            push_status(ui_handle.clone(), snap_now);
        }
    });
    fn enumerate_interfaces() -> Vec<NetworkInterface> {
        vec![
            NetworkInterface {
                name: "wlan0".into(),
                kind: "wifi".into(),
                address_v4: "192.168.1.42".into(),
                address_v6: "fe80::1234".into(),
                enabled: true,
            },
            NetworkInterface {
                name: "rmnet0".into(),
                kind: "cellular".into(),
                address_v4: "10.20.30.40".into(),
                address_v6: "".into(),
                enabled: false,
            },
            NetworkInterface {
                name: "lo".into(),
                kind: "loopback".into(),
                address_v4: "127.0.0.1".into(),
                address_v6: "::1".into(),
                enabled: true,
            },
        ]
    }
    fn push_interfaces(ui_handle: slint::Weak<MainWindow>, list: Vec<NetworkInterface>) {
        let _ = ui_handle.upgrade_in_event_loop(move |ui| {
            let model: slint::ModelRc<NetworkInterface> = std::rc::Rc::new(slint::VecModel::from(list)).into();
            ui.global::<Bridge>().set_network_interfaces(model);
        });
    }
    push_interfaces(ui.as_weak(), enumerate_interfaces());
    let interfaces = std::sync::Arc::new(tokio::sync::Mutex::new(enumerate_interfaces()));
    let interfaces_for_callback = interfaces.clone();
    let ui_for_callback = ui.as_weak();
    ui.global::<Bridge>()
        .on_set_interface_enabled(move |name, value| {
            let interfaces = interfaces_for_callback.clone();
            let ui_handle = ui_for_callback.clone();
            tokio::spawn(async move {
                let mut list = interfaces.lock().await;
                if let Some(iface) = list.iter_mut().find(|i| i.name == name.as_str()) {
                    iface.enabled = value;
                }
                push_interfaces(ui_handle, list.clone());
            });
        });
    let log_ring = log_ring::LogRing::new(ui.as_weak());
    let log_ring_for_clear = log_ring.clone();
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::filter::LevelFilter;
    use tracing_subscriber::Layer;
    // Cap the LogRing layer at DEBUG so the firehose of GStreamer `Fixme`
    // / TRACE events forwarded by `tracing_gstreamer::integrate_events`
    // (see `Application::run_event_loop`) never reaches the ring buffer.
    // Without this filter, an active media pipeline can produce thousands
    // of TRACE events per second, each one mutating the ring and dirtying
    // the UI pusher — pointless for a human-readable debug log.
    //
    // `try_init` (not `init`) so re-entries of `android_main` (Android can
    // trigger on activity destroy/recreate) don't panic from
    // `set_global_default()` being called twice. Mirrors `init_once` above.
    if let Err(err) = tracing_subscriber::registry()
        .with(log_ring.clone().with_filter(LevelFilter::DEBUG))
        .try_init()
    {
        debug!(?err, "tracing subscriber already initialised — re-entry of android_main");
    }
    ui.global::<Bridge>().on_clear_log_entries(move || {
        log_ring_for_clear.clear();
    });
    // ── Bitrate presets (Phase 8 / Cluster C1) — callbacks ──────────────
    // The shared `presets` Arc + `push_presets` closure are declared
    // above (next to `history`) so the D1 `on_reset_settings` handler
    // can also restore the factory list.
    ui.global::<Bridge>().on_save_preset({
        let presets = presets.clone();
        let next_id = next_preset_id.clone();
        let push    = push_presets.clone();
        move |id, name, kbps| {
            let mut g = presets.lock();
            if id.is_empty() {
                let new_id = format!("custom-{}", next_id.fetch_add(1, Ordering::Relaxed));
                g.push(BitratePreset {
                    id:           new_id.into(),
                    name:         name.into(),
                    bitrate_kbps: kbps,
                    active:       false,
                });
            } else if let Some(p) = g.iter_mut().find(|p| p.id == id) {
                p.name         = name.into();
                p.bitrate_kbps = kbps;
            }
            drop(g);
            push();
        }
    });
    ui.global::<Bridge>().on_delete_preset({
        let presets = presets.clone();
        let push    = push_presets.clone();
        move |id| {
            let mut g = presets.lock();
            g.retain(|p| p.id != id);
            // If the deleted preset was the active one, promote the first
            // remaining preset to active so the user is never left without
            // a selection.
            if !g.iter().any(|p| p.active) {
                if let Some(first) = g.first_mut() {
                    first.active = true;
                }
            }
            drop(g);
            push();
        }
    });
    ui.global::<Bridge>().on_set_active_preset({
        let presets = presets.clone();
        let push    = push_presets.clone();
        move |id| {
            let mut g = presets.lock();
            for p in g.iter_mut() {
                p.active = p.id == id;
            }
            drop(g);
            push();
        }
    });

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

    let recorder_state = Arc::new(tokio::sync::Mutex::new(RecordingTickerState::default()));

    ui.global::<Bridge>().on_start_recording({
        let recorder_state = recorder_state.clone();
        let ui_handle = ui.as_weak();
        move || {
            let recorder_state = recorder_state.clone();
            let ui_handle = ui_handle.clone();
            tokio::spawn(async move {
                let mut s = recorder_state.lock().await;
                s.started_at = Some(std::time::Instant::now());
                s.paused_for = std::time::Duration::ZERO;
                s.pause_started = None;
                s.state = RecordingState::Recording;
                let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                    ui.global::<Bridge>().set_recording_state(RecordingState::Recording);
                    ui.global::<Bridge>().set_recording_elapsed_s(0);
                });
            });
        }
    });

    ui.global::<Bridge>().on_pause_recording({
        let recorder_state = recorder_state.clone();
        let ui_handle = ui.as_weak();
        move || {
            let recorder_state = recorder_state.clone();
            let ui_handle = ui_handle.clone();
            tokio::spawn(async move {
                let mut s = recorder_state.lock().await;
                if s.state != RecordingState::Recording { return; }
                s.pause_started = Some(std::time::Instant::now());
                s.state = RecordingState::Paused;
                let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                    ui.global::<Bridge>().set_recording_state(RecordingState::Paused);
                });
            });
        }
    });

    ui.global::<Bridge>().on_resume_recording({
        let recorder_state = recorder_state.clone();
        let ui_handle = ui.as_weak();
        move || {
            let recorder_state = recorder_state.clone();
            let ui_handle = ui_handle.clone();
            tokio::spawn(async move {
                let mut s = recorder_state.lock().await;
                if s.state != RecordingState::Paused { return; }
                if let Some(started) = s.pause_started.take() {
                    s.paused_for += started.elapsed();
                }
                s.state = RecordingState::Recording;
                let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                    ui.global::<Bridge>().set_recording_state(RecordingState::Recording);
                });
            });
        }
    });

    ui.global::<Bridge>().on_stop_recording({
        let recorder_state = recorder_state.clone();
        let ui_handle = ui.as_weak();
        move || {
            let recorder_state = recorder_state.clone();
            let ui_handle = ui_handle.clone();
            tokio::spawn(async move {
                {
                    let mut s = recorder_state.lock().await;
                    s.state = RecordingState::Finalizing;
                }
                let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                    ui.global::<Bridge>().set_recording_state(RecordingState::Finalizing);
                });

                let mut s = recorder_state.lock().await;
                s.started_at = None;
                s.paused_for = std::time::Duration::ZERO;
                s.pause_started = None;
                s.state = RecordingState::Idle;
                let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                    let bridge = ui.global::<Bridge>();
                    bridge.set_recording_state(RecordingState::Idle);
                    bridge.set_recording_elapsed_s(0);
                });
            });
        }
    });

    spawn_recording_ticker(ui.as_weak(), recorder_state.clone());

    ui.global::<Bridge>().on_engage_lock({
        let ui_handle = ui.as_weak();
        move || {
            let _ = ui_handle.upgrade_in_event_loop(|ui| {
                ui.global::<Bridge>().set_lifecycle(LifecycleMode::LockScreen);
            });
        }
    });

    ui.global::<Bridge>().on_engage_stealth({
        let ui_handle = ui.as_weak();
        move || {
            let _ = ui_handle.upgrade_in_event_loop(|ui| {
                ui.global::<Bridge>().set_lifecycle(LifecycleMode::Stealth);
            });
        }
    });

    ui.global::<Bridge>().on_start_snapshot_countdown({
        let ui_handle = ui.as_weak();
        // Monotonic generation counter. Each new countdown bumps it and
        // captures the new value; the spawned timer only resets lifecycle
        // if its captured generation is still current. This makes any
        // older, still-sleeping timer a no-op when a newer countdown is
        // triggered. Mirrors `Application::banner_generation`.
        static SNAPSHOT_GEN: AtomicU64 = AtomicU64::new(0);
        move |seconds: i32| {
            let ui_handle = ui_handle.clone();
            let gen = SNAPSHOT_GEN.fetch_add(1, Ordering::SeqCst) + 1;
            tokio::spawn(async move {
                let _ = ui_handle.upgrade_in_event_loop(|ui| {
                    ui.global::<Bridge>().set_lifecycle(LifecycleMode::SnapshotCountdown);
                });
                tokio::time::sleep(std::time::Duration::from_secs(seconds.max(0) as u64)).await;
                // Only reset to Normal if (a) no newer countdown has started
                // (otherwise we'd cut the new one short) and (b) we are still
                // in SnapshotCountdown (otherwise the user cancelled or
                // engaged lock/stealth and we must not clobber their choice).
                if SNAPSHOT_GEN.load(Ordering::SeqCst) != gen {
                    return;
                }
                let _ = ui_handle.upgrade_in_event_loop(|ui| {
                    let bridge = ui.global::<Bridge>();
                    if bridge.get_lifecycle() == LifecycleMode::SnapshotCountdown {
                        bridge.set_lifecycle(LifecycleMode::Normal);
                    }
                });
            });
        }
    });

    ui.global::<Bridge>().on_exit_lifecycle({
        let ui_handle = ui.as_weak();
        move || {
            let _ = ui_handle.upgrade_in_event_loop(|ui| {
                ui.global::<Bridge>().set_lifecycle(LifecycleMode::Normal);
            });
        }
    });

    ui.global::<Bridge>().on_set_wifi_aware({
        let ui_handle = ui.as_weak();
        move |enabled| {
            let ui_handle = ui_handle.clone();
            tokio::spawn(async move {
                let success = true;
                let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                    let bridge = ui.global::<Bridge>();
                    bridge.set_wifi_aware_enabled(enabled && success);
                });

                Application::flash_banner(
                    ui_handle,
                    if enabled {
                        "Wi-Fi Aware enabled (placeholder — no permission requested).".into()
                    } else {
                        "Wi-Fi Aware disabled.".into()
                    },
                    BannerSeverity::Info,
                    std::time::Duration::from_secs(3),
                );
            });
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
                    let _ = ui_weak.upgrade_in_event_loop(|ui| {
                        ui.global::<Bridge>()
                            .invoke_start_migration_server(LEGACY_COMMAND_BIND_ADDR.into());
                    });
                }
                "test-getinfo" => {
                    let _ = ui_weak.upgrade_in_event_loop(|ui| {
                        ui.global::<Bridge>()
                            .invoke_run_migration_test("getinfo".into());
                    });
                }
                "test-crossfade" => {
                    let _ = ui_weak.upgrade_in_event_loop(|ui| {
                        ui.global::<Bridge>()
                            .invoke_run_migration_test("crossfade".into());
                    });
                }
                "test-smoke" => {
                    let _ = ui_weak.upgrade_in_event_loop(|ui| {
                        ui.global::<Bridge>()
                            .invoke_run_migration_test("smoke".into());
                    });
                }
                _ => {}
            }
        }
    });

    #[cfg(debug_assertions)]
    {
        ui.global::<Bridge>().on_start_migration_server({
            let ui_weak = ui.as_weak();
            move |bind_addr| {
                let status = match start_migrated_command_server(bind_addr.as_str()) {
                    Ok(message) => format!("PASS {message}"),
                    Err(err) => format!("FAIL {err}"),
                };
                log_ui_test_status("start-migration-server", &status);
                let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                    ui.global::<Bridge>().set_test_status(status.into());
                });
            }
        });

        ui.global::<Bridge>().on_run_migration_test({
            let ui_weak = ui.as_weak();
            move |test_id| {
                let test_id = test_id.to_string();
                let _ = ui_weak.upgrade_in_event_loop({
                    let test_id = test_id.clone();
                    move |ui| {
                        ui.global::<Bridge>().set_test_status(
                            format!("Running migration test '{test_id}'...").into(),
                        );
                    }
                });
                let ui_weak_clone = ui_weak.clone();
                std::thread::spawn(move || {
                    let status = match test_id.as_str() {
                        "getinfo" => run_legacy_http_getinfo_test(LEGACY_COMMAND_BIND_ADDR),
                        "crossfade" => run_legacy_http_crossfade_test(LEGACY_COMMAND_BIND_ADDR),
                        "smoke" => run_graph_smoke_test(),
                        other => format!("FAIL unknown migration-test id: {other}"),
                    };
                    log_ui_test_status(migration_test_log_name(&test_id), &status);
                    let _ = ui_weak_clone.upgrade_in_event_loop(move |ui| {
                        ui.global::<Bridge>().set_test_status(status.into());
                    });
                });
            }
        });

        ui.global::<Bridge>().on_stop_migration_server({
            let ui_weak = ui.as_weak();
            move || {
                let status = match crate::migration::runtime::shutdown_graph_runtime() {
                    Ok(()) => "PASS migration server stopped".to_string(),
                    Err(err) => format!("FAIL migration server stop: {err}"),
                };
                log_ui_test_status("stop-migration-server", &status);
                let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                    ui.global::<Bridge>().set_test_status(status.into());
                });
            }
        });
    }





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

    // Drop the runtime context guard before `block_on`, which panics if
    // called from within a tokio runtime context.
    drop(_runtime_guard);

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
    if let Err(err) = crate::migration::runtime::start_graph_runtime() {
        error!(?err, "Failed to start migrated graph runtime from JNI hook");
    }

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

#[cfg(test)]
mod phase9_dispatch_tests {
    use super::migration_test_log_name;

    #[test]
    fn migration_test_log_name_known_ids() {
        assert_eq!(migration_test_log_name("getinfo"), "legacy-getinfo");
        assert_eq!(migration_test_log_name("crossfade"), "legacy-crossfade");
        assert_eq!(migration_test_log_name("smoke"), "graph-smoke");
    }

    #[test]
    fn migration_test_log_name_unknown_id() {
        assert_eq!(migration_test_log_name(""), "unknown");
        assert_eq!(migration_test_log_name("bogus"), "unknown");
        assert_eq!(migration_test_log_name("GetInfo"), "unknown");
    }

    #[test]
    fn migration_test_id_count_invariant() {
        const KNOWN: &[&str] = &["getinfo", "crossfade", "smoke"];

        for id in KNOWN {
            assert_ne!(
                migration_test_log_name(id),
                "unknown",
                "test id {id} should be in the dispatch table"
            );
        }
    }
}
