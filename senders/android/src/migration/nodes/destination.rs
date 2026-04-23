use crate::migration::protocol::{DestinationFamily, DestinationInfo, NodeInfo, State};
use chrono::{DateTime, Duration, Utc};
use gst::prelude::*;
use gst_app::AppSrc;
use tracing::{error, info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestinationPipelineStage {
    Idle,
    Scheduled,
    Playing,
}

#[derive(Debug, Clone)]
pub struct DestinationPipelineProfile {
    pub family: DestinationFamily,
    pub elements: Vec<String>,
    pub wait_for_eos_on_stop: bool,
    pub stage: DestinationPipelineStage,
}

#[derive(Debug, Clone)]
pub struct LiveDestinationPipeline {
    pub pipeline: gst::Pipeline,
    pub video_appsrc: Option<AppSrc>,
    pub audio_appsrc: Option<AppSrc>,
}

impl DestinationPipelineProfile {
    fn from_family(family: &DestinationFamily, audio: bool, video: bool) -> Self {
        let mut elements = Vec::new();

        match family {
            DestinationFamily::Rtmp { .. } => {
                elements.extend([
                    "flvmux",
                    "queue",
                    "rtmp2sink",
                    "videoconvert",
                    "timecodestamper",
                    "timeoverlay",
                    "h264enc",
                    "h264parse",
                    "audioconvert",
                    "audioresample",
                    "avenc_aac",
                ]);
            }
            DestinationFamily::Udp { .. } => {
                elements.extend([
                    "mpegtsmux",
                    "udpsink",
                    "videoconvert",
                    "h264enc",
                    "h264parse",
                    "audioconvert",
                    "audioresample",
                    "avenc_aac",
                ]);
            }
            DestinationFamily::LocalFile { .. } => {
                elements.extend([
                    "splitmuxsink",
                    "multiqueue",
                    "videoconvert",
                    "h264enc",
                    "h264parse",
                    "audioconvert",
                    "audioresample",
                    "avenc_aac",
                ]);
            }
            DestinationFamily::LocalPlayback => {
                elements.extend([
                    "autovideosink",
                    "autoaudiosink",
                    "videoconvert",
                    "audioconvert",
                    "audioresample",
                    "queue",
                ]);
            }
        }

        if !audio {
            elements.retain(|el| !el.contains("audio"));
        }
        if !video {
            elements.retain(|el| !el.contains("video") && !el.contains("h264"));
        }

        Self {
            family: family.clone(),
            elements: elements.into_iter().map(str::to_string).collect(),
            wait_for_eos_on_stop: true,
            stage: DestinationPipelineStage::Idle,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DestinationNode {
    pub id: String,
    pub family: DestinationFamily,
    pub audio_enabled: bool,
    pub video_enabled: bool,
    pub audio_slot_id: Option<String>,
    pub video_slot_id: Option<String>,
    pub cue_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub state: State,
    pub pipeline: Option<DestinationPipelineProfile>,
    pub live_pipeline: Option<LiveDestinationPipeline>,
    pub last_error: Option<String>,
}

impl DestinationNode {
    fn gst_initialized() -> bool {
        unsafe { gst::ffi::gst_is_initialized() != 0 }
    }

    pub fn new(
        id: String,
        family: DestinationFamily,
        audio_enabled: bool,
        video_enabled: bool,
    ) -> Self {
        Self {
            id,
            family,
            audio_enabled,
            video_enabled,
            audio_slot_id: None,
            video_slot_id: None,
            cue_time: None,
            end_time: None,
            state: State::Initial,
            pipeline: None,
            live_pipeline: None,
            last_error: None,
        }
    }

    fn make_element(element: &str, name: Option<&str>) -> Result<gst::Element, String> {
        let factory = gst::ElementFactory::make(element);
        let builder = match name {
            Some(name) => factory.name(name),
            None => factory,
        };
        builder
            .build()
            .map_err(|err| format!("Failed to create element `{element}`: {}", &*err.message))
    }

    fn make_appsrc(id: &str, media: &str) -> Result<AppSrc, String> {
        let element =
            Self::make_element("appsrc", Some(&format!("destination-{media}-appsrc-{id}")))?;
        let appsrc = element
            .downcast::<AppSrc>()
            .map_err(|_| format!("Failed to downcast appsrc element for destination `{id}`"))?;
        appsrc.set_property("is-live", true);
        appsrc.set_property("do-timestamp", true);
        appsrc.set_property_from_str("format", "time");
        appsrc.set_property("block", false);
        Ok(appsrc)
    }

    fn make_first_available_element(
        candidates: &[&str],
        name: Option<&str>,
        purpose: &str,
    ) -> Result<gst::Element, String> {
        let mut failures = Vec::new();
        for (idx, candidate) in candidates.iter().enumerate() {
            match Self::make_element(candidate, name) {
                Ok(element) => {
                    if *candidate == "fakesink" {
                        error!(
                            purpose = %purpose,
                            element = %candidate,
                            "All real sink candidates failed; using fakesink (no output)"
                        );
                    } else if idx == 0 {
                        info!(purpose = %purpose, element = %candidate, "Using GStreamer element");
                    } else {
                        warn!(
                            purpose = %purpose,
                            element = %candidate,
                            "Using fallback GStreamer element"
                        );
                    }
                    return Ok(element);
                }
                Err(err) => failures.push(format!("{candidate}: {err}")),
            }
        }

        Err(format!(
            "Failed to create {purpose}. Attempted elements: {}",
            failures.join(" | ")
        ))
    }

    fn make_local_video_sink(id: &str) -> Result<gst::Element, String> {
        #[cfg(target_os = "android")]
        const CANDIDATES: &[&str] = &["glimagesink", "autovideosink", "fakesink"];
        #[cfg(not(target_os = "android"))]
        const CANDIDATES: &[&str] = &["autovideosink", "glimagesink", "fakesink"];

        let sink_name = format!("destination-video-sink-{id}");
        Self::make_first_available_element(
            CANDIDATES,
            Some(&sink_name),
            "local playback video sink",
        )
    }

    fn make_local_audio_sink(id: &str) -> Result<gst::Element, String> {
        #[cfg(target_os = "android")]
        const CANDIDATES: &[&str] = &[
            "openslessink",
            "autoaudiosink",
            "audiotracksink",
            "fakesink",
        ];
        #[cfg(not(target_os = "android"))]
        const CANDIDATES: &[&str] = &["autoaudiosink", "pulsesink", "alsasink", "fakesink"];

        let sink_name = format!("destination-audio-sink-{id}");
        Self::make_first_available_element(
            CANDIDATES,
            Some(&sink_name),
            "local playback audio sink",
        )
    }

    fn select_video_encoder(id: &str) -> Result<gst::Element, String> {
        for encoder in ["nvh264enc", "x264enc", "openh264enc"] {
            if let Ok(venc) = Self::make_element(encoder, Some(&format!("destination-venc-{id}"))) {
                if venc.has_property("tune") {
                    venc.set_property_from_str("tune", "zerolatency");
                } else if venc.has_property("zerolatency") {
                    venc.set_property("zerolatency", true);
                }
                if venc.has_property("key-int-max") {
                    venc.set_property("key-int-max", 30u32);
                } else if venc.has_property("gop-size") {
                    venc.set_property("gop-size", 30i32);
                }
                return Ok(venc);
            }
        }
        Err(
            "Failed to create a H.264 video encoder (tried nvh264enc, x264enc, openh264enc)"
                .to_string(),
        )
    }

    fn build_live_pipeline(
        &self,
        _profile: &DestinationPipelineProfile,
    ) -> Result<LiveDestinationPipeline, String> {
        let pipeline = gst::Pipeline::with_name(&format!("migration-destination-{}", self.id));

        let video_appsrc = if self.video_enabled {
            let appsrc = Self::make_appsrc(&self.id, "video")?;
            pipeline
                .add(appsrc.upcast_ref::<gst::Element>())
                .map_err(|err| {
                    format!("Failed to add video appsrc to destination pipeline: {err:?}")
                })?;
            Some(appsrc)
        } else {
            None
        };

        let audio_appsrc = if self.audio_enabled {
            let appsrc = Self::make_appsrc(&self.id, "audio")?;
            pipeline
                .add(appsrc.upcast_ref::<gst::Element>())
                .map_err(|err| {
                    format!("Failed to add audio appsrc to destination pipeline: {err:?}")
                })?;
            Some(appsrc)
        } else {
            None
        };

        match &self.family {
            DestinationFamily::Rtmp { uri } => {
                let mux = Self::make_element("flvmux", None)?;
                let mux_queue = Self::make_element("queue", None)?;
                let sink = Self::make_element("rtmp2sink", None)?;

                pipeline.add(&mux).map_err(|err| {
                    format!("Failed to add flvmux to destination pipeline: {err:?}")
                })?;
                pipeline.add(&mux_queue).map_err(|err| {
                    format!("Failed to add mux queue to destination pipeline: {err:?}")
                })?;
                pipeline.add(&sink).map_err(|err| {
                    format!("Failed to add rtmp2sink to destination pipeline: {err:?}")
                })?;

                sink.set_property("location", uri.clone());
                if sink.has_property("tls-validation-flags") {
                    sink.set_property_from_str("tls-validation-flags", "generic-error");
                }
                if mux.has_property("streamable") {
                    mux.set_property("streamable", true);
                }
                if mux.has_property("latency") {
                    mux.set_property("latency", 1_000_000_000u64);
                }
                gst::Element::link_many([&mux, &mux_queue, &sink].as_slice())
                    .map_err(|err| format!("Failed to link rtmp mux chain: {err:?}"))?;

                if let Some(appsrc) = video_appsrc.as_ref() {
                    let vconv = Self::make_element("videoconvert", None)?;
                    let timecodestamper = Self::make_element("timecodestamper", None)?;
                    let timeoverlay = Self::make_element("timeoverlay", None)?;
                    let venc = Self::select_video_encoder(&self.id)?;
                    let vparse = Self::make_element("h264parse", None)?;
                    let venc_queue = Self::make_element("queue", None)?;

                    pipeline.add(&vconv).map_err(|err| {
                        format!("Failed to add videoconvert to rtmp pipeline: {err:?}")
                    })?;
                    pipeline.add(&timecodestamper).map_err(|err| {
                        format!("Failed to add timecodestamper to rtmp pipeline: {err:?}")
                    })?;
                    pipeline.add(&timeoverlay).map_err(|err| {
                        format!("Failed to add timeoverlay to rtmp pipeline: {err:?}")
                    })?;
                    pipeline.add(&venc).map_err(|err| {
                        format!("Failed to add video encoder to rtmp pipeline: {err:?}")
                    })?;
                    pipeline.add(&vparse).map_err(|err| {
                        format!("Failed to add h264parse to rtmp pipeline: {err:?}")
                    })?;
                    pipeline.add(&venc_queue).map_err(|err| {
                        format!("Failed to add video queue to rtmp pipeline: {err:?}")
                    })?;

                    if vparse.has_property("config-interval") {
                        vparse.set_property("config-interval", -1i32);
                    }
                    if timecodestamper.has_property("source") {
                        timecodestamper.set_property_from_str("source", "rtc");
                    }
                    if timeoverlay.has_property("time-mode") {
                        timeoverlay.set_property_from_str("time-mode", "time-code");
                    }

                    gst::Element::link_many(
                        [
                            appsrc.upcast_ref::<gst::Element>(),
                            &vconv,
                            &timecodestamper,
                            &timeoverlay,
                            &venc,
                            &vparse,
                            &venc_queue,
                            &mux,
                        ]
                        .as_slice(),
                    )
                    .map_err(|err| format!("Failed to link rtmp video chain: {err:?}"))?;
                }

                if let Some(appsrc) = audio_appsrc.as_ref() {
                    let aconv = Self::make_element("audioconvert", None)?;
                    let aresample = Self::make_element("audioresample", None)?;
                    let aenc = Self::make_element("avenc_aac", None)?;
                    let aenc_queue = Self::make_element("queue", None)?;

                    pipeline.add(&aconv).map_err(|err| {
                        format!("Failed to add audioconvert to rtmp pipeline: {err:?}")
                    })?;
                    pipeline.add(&aresample).map_err(|err| {
                        format!("Failed to add audioresample to rtmp pipeline: {err:?}")
                    })?;
                    pipeline.add(&aenc).map_err(|err| {
                        format!("Failed to add avenc_aac to rtmp pipeline: {err:?}")
                    })?;
                    pipeline.add(&aenc_queue).map_err(|err| {
                        format!("Failed to add audio queue to rtmp pipeline: {err:?}")
                    })?;

                    gst::Element::link_many(
                        [
                            appsrc.upcast_ref::<gst::Element>(),
                            &aconv,
                            &aresample,
                            &aenc,
                            &aenc_queue,
                            &mux,
                        ]
                        .as_slice(),
                    )
                    .map_err(|err| format!("Failed to link rtmp audio chain: {err:?}"))?;
                }
            }
            DestinationFamily::Udp { host } => {
                let mux = Self::make_element("mpegtsmux", None)?;
                let sink = Self::make_element("udpsink", None)?;

                pipeline.add(&mux).map_err(|err| {
                    format!("Failed to add mpegtsmux to destination pipeline: {err:?}")
                })?;
                pipeline.add(&sink).map_err(|err| {
                    format!("Failed to add udpsink to destination pipeline: {err:?}")
                })?;

                sink.set_property("host", host.clone());
                sink.set_property("port", 5005i32);
                if mux.has_property("alignment") {
                    mux.set_property("alignment", 7i32);
                }

                if let Some(appsrc) = video_appsrc.as_ref() {
                    let vconv = Self::make_element("videoconvert", None)?;
                    let venc = Self::select_video_encoder(&self.id)?;
                    let vparse = Self::make_element("h264parse", None)?;

                    pipeline.add(&vconv).map_err(|err| {
                        format!("Failed to add videoconvert to udp pipeline: {err:?}")
                    })?;
                    pipeline.add(&venc).map_err(|err| {
                        format!("Failed to add video encoder to udp pipeline: {err:?}")
                    })?;
                    pipeline.add(&vparse).map_err(|err| {
                        format!("Failed to add h264parse to udp pipeline: {err:?}")
                    })?;

                    gst::Element::link_many(
                        [
                            appsrc.upcast_ref::<gst::Element>(),
                            &vconv,
                            &venc,
                            &vparse,
                            &mux,
                        ]
                        .as_slice(),
                    )
                    .map_err(|err| format!("Failed to link udp video chain: {err:?}"))?;
                }
                if let Some(appsrc) = audio_appsrc.as_ref() {
                    let aconv = Self::make_element("audioconvert", None)?;
                    let aresample = Self::make_element("audioresample", None)?;
                    let aenc = Self::make_element("avenc_aac", None)?;

                    pipeline.add(&aconv).map_err(|err| {
                        format!("Failed to add audioconvert to udp pipeline: {err:?}")
                    })?;
                    pipeline.add(&aresample).map_err(|err| {
                        format!("Failed to add audioresample to udp pipeline: {err:?}")
                    })?;
                    pipeline.add(&aenc).map_err(|err| {
                        format!("Failed to add avenc_aac to udp pipeline: {err:?}")
                    })?;

                    gst::Element::link_many(
                        [
                            appsrc.upcast_ref::<gst::Element>(),
                            &aconv,
                            &aresample,
                            &aenc,
                            &mux,
                        ]
                        .as_slice(),
                    )
                    .map_err(|err| format!("Failed to link udp audio chain: {err:?}"))?;
                }

                mux.link(&sink)
                    .map_err(|err| format!("Failed to link mpegtsmux to udpsink: {err:?}"))?;
            }
            DestinationFamily::LocalFile {
                base_name,
                max_size_time,
            } => {
                let multiqueue = Self::make_element("multiqueue", None)?;
                let sink = Self::make_element("splitmuxsink", None)?;

                pipeline.add(&multiqueue).map_err(|err| {
                    format!("Failed to add multiqueue to local-file pipeline: {err:?}")
                })?;
                pipeline.add(&sink).map_err(|err| {
                    format!("Failed to add splitmuxsink to local-file pipeline: {err:?}")
                })?;

                match max_size_time {
                    Some(max_size_time) => {
                        let max_size_time_ns = (*max_size_time as u64) * 1_000_000;
                        sink.set_property("max-size-time", max_size_time_ns);
                        if sink.has_property("use-robust-muxing") {
                            sink.set_property("use-robust-muxing", true);
                        }
                        sink.set_property("location", format!("{base_name}%05d.mp4"));
                    }
                    None => {
                        sink.set_property("location", format!("{base_name}.mp4"));
                    }
                }

                if let Some(appsrc) = video_appsrc.as_ref() {
                    let vconv = Self::make_element("videoconvert", None)?;
                    let venc = Self::select_video_encoder(&self.id)?;
                    let vparse = Self::make_element("h264parse", None)?;

                    pipeline.add(&vconv).map_err(|err| {
                        format!("Failed to add videoconvert to local-file pipeline: {err:?}")
                    })?;
                    pipeline.add(&venc).map_err(|err| {
                        format!("Failed to add video encoder to local-file pipeline: {err:?}")
                    })?;
                    pipeline.add(&vparse).map_err(|err| {
                        format!("Failed to add h264parse to local-file pipeline: {err:?}")
                    })?;

                    gst::Element::link_many(
                        [appsrc.upcast_ref::<gst::Element>(), &vconv, &venc, &vparse].as_slice(),
                    )
                    .map_err(|err| format!("Failed to link local-file video chain: {err:?}"))?;

                    vparse
                        .link_pads(None, &multiqueue, Some("sink_0"))
                        .map_err(|err| {
                            format!("Failed to link local-file video queue sink: {err:?}")
                        })?;
                    multiqueue
                        .link_pads(Some("src_0"), &sink, Some("video"))
                        .map_err(|err| {
                            format!("Failed to link local-file video queue source: {err:?}")
                        })?;
                }

                if let Some(appsrc) = audio_appsrc.as_ref() {
                    let aconv = Self::make_element("audioconvert", None)?;
                    let aresample = Self::make_element("audioresample", None)?;
                    let aenc = Self::make_element("avenc_aac", None)?;

                    pipeline.add(&aconv).map_err(|err| {
                        format!("Failed to add audioconvert to local-file pipeline: {err:?}")
                    })?;
                    pipeline.add(&aresample).map_err(|err| {
                        format!("Failed to add audioresample to local-file pipeline: {err:?}")
                    })?;
                    pipeline.add(&aenc).map_err(|err| {
                        format!("Failed to add avenc_aac to local-file pipeline: {err:?}")
                    })?;

                    gst::Element::link_many(
                        [
                            appsrc.upcast_ref::<gst::Element>(),
                            &aconv,
                            &aresample,
                            &aenc,
                        ]
                        .as_slice(),
                    )
                    .map_err(|err| format!("Failed to link local-file audio chain: {err:?}"))?;

                    aenc.link_pads(None, &multiqueue, Some("sink_1"))
                        .map_err(|err| {
                            format!("Failed to link local-file audio queue sink: {err:?}")
                        })?;
                    multiqueue
                        .link_pads(Some("src_1"), &sink, Some("audio_0"))
                        .map_err(|err| {
                            format!("Failed to link local-file audio queue source: {err:?}")
                        })?;
                }
            }
            DestinationFamily::LocalPlayback => {
                if let Some(appsrc) = video_appsrc.as_ref() {
                    let vqueue = Self::make_element("queue", None)?;
                    let vconv = Self::make_element("videoconvert", None)?;
                    let vsink = Self::make_local_video_sink(&self.id)?;

                    pipeline.add(&vqueue).map_err(|err| {
                        format!("Failed to add video queue to local-playback pipeline: {err:?}")
                    })?;
                    pipeline.add(&vconv).map_err(|err| {
                        format!("Failed to add videoconvert to local-playback pipeline: {err:?}")
                    })?;
                    pipeline.add(&vsink).map_err(|err| {
                        format!("Failed to add video sink to local-playback pipeline: {err:?}")
                    })?;

                    gst::Element::link_many(
                        [appsrc.upcast_ref::<gst::Element>(), &vqueue, &vconv, &vsink].as_slice(),
                    )
                    .map_err(|err| format!("Failed to link local-playback video chain: {err:?}"))?;
                }

                if let Some(appsrc) = audio_appsrc.as_ref() {
                    let aqueue = Self::make_element("queue", None)?;
                    let aconv = Self::make_element("audioconvert", None)?;
                    let aresample = Self::make_element("audioresample", None)?;
                    let asink = Self::make_local_audio_sink(&self.id)?;

                    pipeline.add(&aqueue).map_err(|err| {
                        format!("Failed to add audio queue to local-playback pipeline: {err:?}")
                    })?;
                    pipeline.add(&aconv).map_err(|err| {
                        format!("Failed to add audioconvert to local-playback pipeline: {err:?}")
                    })?;
                    pipeline.add(&aresample).map_err(|err| {
                        format!("Failed to add audioresample to local-playback pipeline: {err:?}")
                    })?;
                    pipeline.add(&asink).map_err(|err| {
                        format!("Failed to add audio sink to local-playback pipeline: {err:?}")
                    })?;

                    gst::Element::link_many(
                        [
                            appsrc.upcast_ref::<gst::Element>(),
                            &aqueue,
                            &aconv,
                            &aresample,
                            &asink,
                        ]
                        .as_slice(),
                    )
                    .map_err(|err| format!("Failed to link local-playback audio chain: {err:?}"))?;
                }
            }
        }

        Ok(LiveDestinationPipeline {
            pipeline,
            video_appsrc,
            audio_appsrc,
        })
    }

    fn teardown_live_pipeline(&mut self) {
        if let Some(live) = self.live_pipeline.take() {
            let _ = live.pipeline.set_state(gst::State::Null);
        }
    }

    fn ensure_live_pipeline(&mut self) -> Result<(), String> {
        if self.live_pipeline.is_some() {
            return Ok(());
        }
        let Some(profile) = self.pipeline.as_ref() else {
            return Err(format!(
                "Destination {} has no active pipeline profile to realize",
                self.id
            ));
        };

        self.live_pipeline = Some(self.build_live_pipeline(profile)?);
        Ok(())
    }

    fn poll_bus_messages(&mut self) -> Result<(), String> {
        let Some(live) = self.live_pipeline.as_ref() else {
            return Ok(());
        };
        let Some(bus) = live.pipeline.bus() else {
            return Ok(());
        };

        let mut saw_eos = false;
        let mut last_error = None;
        while let Some(message) = bus.timed_pop_filtered(
            gst::ClockTime::ZERO,
            &[gst::MessageType::Error, gst::MessageType::Eos],
        ) {
            match message.view() {
                gst::MessageView::Eos(..) => saw_eos = true,
                gst::MessageView::Error(err) => {
                    last_error = Some(format!(
                        "Destination {} pipeline error from {:?}: {} ({:?})",
                        self.id,
                        err.src().map(|s| s.path_string()),
                        err.error(),
                        err.debug()
                    ));
                }
                _ => {}
            }
        }

        if let Some(err) = last_error {
            self.last_error = Some(err.clone());
            self.state = State::Stopped;
            if let Some(profile) = self.pipeline.as_mut() {
                profile.stage = DestinationPipelineStage::Idle;
            }
            self.teardown_live_pipeline();
            return Err(err);
        }

        if saw_eos {
            self.state = State::Stopped;
            if let Some(profile) = self.pipeline.as_mut() {
                profile.stage = DestinationPipelineStage::Idle;
            }
            self.teardown_live_pipeline();
        }

        Ok(())
    }

    fn wait_for_eos_on_stop(&mut self) {
        let should_wait = self
            .pipeline
            .as_ref()
            .map(|profile| profile.wait_for_eos_on_stop)
            .unwrap_or(false);

        if !should_wait {
            return;
        }

        let Some(live) = self.live_pipeline.as_ref() else {
            return;
        };
        let Some(bus) = live.pipeline.bus() else {
            return;
        };

        self.state = State::Stopping;
        if let Some(video_appsrc) = live.video_appsrc.as_ref() {
            let _ = video_appsrc.end_of_stream();
        }
        if let Some(audio_appsrc) = live.audio_appsrc.as_ref() {
            let _ = audio_appsrc.end_of_stream();
        }

        let wait_deadline = Utc::now() + Duration::seconds(5);
        while Utc::now() < wait_deadline {
            let Some(message) = bus.timed_pop_filtered(
                gst::ClockTime::from_mseconds(100),
                &[gst::MessageType::Error, gst::MessageType::Eos],
            ) else {
                continue;
            };

            match message.view() {
                gst::MessageView::Eos(..) => break,
                gst::MessageView::Error(err) => {
                    self.last_error = Some(format!(
                        "Destination {} pipeline error while waiting for EOS from {:?}: {} ({:?})",
                        self.id,
                        err.src().map(|s| s.path_string()),
                        err.error(),
                        err.debug()
                    ));
                    break;
                }
                _ => {}
            }
        }
    }

    fn sync_live_pipeline(&mut self) -> Result<(), String> {
        if !Self::gst_initialized() {
            return Ok(());
        }

        self.poll_bus_messages()?;

        let stage = self
            .pipeline
            .as_ref()
            .map(|profile| profile.stage)
            .unwrap_or(DestinationPipelineStage::Idle);

        match stage {
            DestinationPipelineStage::Idle => {
                self.teardown_live_pipeline();
                Ok(())
            }
            DestinationPipelineStage::Scheduled | DestinationPipelineStage::Playing => {
                self.ensure_live_pipeline()?;
                let target_state = if stage == DestinationPipelineStage::Scheduled {
                    gst::State::Paused
                } else {
                    gst::State::Playing
                };

                if let Some(live) = self.live_pipeline.as_ref() {
                    live.pipeline.set_state(target_state).map_err(|err| {
                        format!(
                            "Failed to set destination pipeline state to {target_state:?}: {err:?}"
                        )
                    })?;
                }

                self.poll_bus_messages()
            }
        }
    }

    pub fn refresh(&mut self) -> Result<(), String> {
        self.advance_schedule(Utc::now());
        self.sync_live_pipeline()
    }

    fn schedule_transition_due(&self, now: DateTime<Utc>) -> Option<State> {
        match self.state {
            State::Initial => {
                if self.cue_time.map_or(true, |cue| now >= cue) {
                    Some(State::Starting)
                } else {
                    None
                }
            }
            State::Starting => Some(State::Started),
            State::Started => {
                if self.end_time.is_some_and(|end| now >= end) {
                    Some(State::Stopping)
                } else {
                    None
                }
            }
            State::Stopping => Some(State::Stopped),
            State::Stopped => None,
        }
    }

    fn apply_state_to_stage(&mut self) {
        if let Some(profile) = self.pipeline.as_mut() {
            profile.stage = match self.state {
                State::Initial | State::Stopping | State::Stopped => DestinationPipelineStage::Idle,
                State::Starting | State::Started => DestinationPipelineStage::Playing,
            };
        }
    }

    fn advance_schedule(&mut self, now: DateTime<Utc>) -> bool {
        let mut changed = false;
        while let Some(next_state) = self.schedule_transition_due(now) {
            if next_state == self.state {
                break;
            }

            if next_state == State::Stopping {
                self.state = State::Stopping;
                self.stop();
                changed = true;
                break;
            }

            self.state = next_state;
            changed = true;
        }

        self.apply_state_to_stage();
        changed
    }

    pub fn connect_input(&mut self, link_id: &str, audio: bool, video: bool) -> Result<(), String> {
        if audio {
            if self.audio_slot_id.is_some() {
                return Err(format!(
                    "Destination {} already has an audio input slot",
                    self.id
                ));
            }
            self.audio_slot_id = Some(link_id.to_string());
        }
        if video {
            if self.video_slot_id.is_some() {
                return Err(format!(
                    "Destination {} already has a video input slot",
                    self.id
                ));
            }
            self.video_slot_id = Some(link_id.to_string());
        }
        Ok(())
    }

    pub fn disconnect_input(&mut self, link_id: &str) {
        if self.audio_slot_id.as_deref() == Some(link_id) {
            self.audio_slot_id = None;
        }
        if self.video_slot_id.as_deref() == Some(link_id) {
            self.video_slot_id = None;
        }
    }

    pub fn live_audio_appsrc(&self) -> Option<AppSrc> {
        self.live_pipeline
            .as_ref()
            .and_then(|live| live.audio_appsrc.clone())
    }

    pub fn live_video_appsrc(&self) -> Option<AppSrc> {
        self.live_pipeline
            .as_ref()
            .and_then(|live| live.video_appsrc.clone())
    }

    fn ensure_start_ready(&self) -> Result<(), String> {
        if self.audio_enabled && self.audio_slot_id.is_none() {
            return Err(format!(
                "Destination {} must have its audio slot connected before starting",
                self.id
            ));
        }
        if self.video_enabled && self.video_slot_id.is_none() {
            return Err(format!(
                "Destination {} must have its video slot connected before starting",
                self.id
            ));
        }
        Ok(())
    }

    /// Mirrors old destination behavior:
    /// - validates required slots before scheduling
    /// - builds family-specific pipeline profile on activation
    pub fn schedule(
        &mut self,
        cue_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<(), String> {
        self.ensure_start_ready()?;
        self.cue_time = cue_time;
        self.end_time = end_time;
        self.last_error = None;
        if matches!(
            self.state,
            State::Starting | State::Stopping | State::Stopped
        ) {
            self.state = State::Initial;
        }
        self.pipeline = Some(DestinationPipelineProfile::from_family(
            &self.family,
            self.audio_enabled,
            self.video_enabled,
        ));
        self.advance_schedule(Utc::now());

        if let Err(err) = self.sync_live_pipeline() {
            self.last_error = Some(err.clone());
            self.state = State::Stopped;
            if let Some(profile) = self.pipeline.as_mut() {
                profile.stage = DestinationPipelineStage::Idle;
            }
            self.teardown_live_pipeline();
            return Err(err);
        }

        Ok(())
    }

    pub fn stop(&mut self) {
        self.wait_for_eos_on_stop();
        self.teardown_live_pipeline();
        self.state = State::Stopped;
        if let Some(pipeline) = self.pipeline.as_mut() {
            pipeline.stage = DestinationPipelineStage::Idle;
        }
    }

    pub fn mark_error(&mut self, message: String) {
        self.last_error = Some(message);
    }

    pub fn as_info(&self) -> NodeInfo {
        NodeInfo::Destination(DestinationInfo {
            family: self.family.clone(),
            audio_slot_id: self.audio_slot_id.clone(),
            video_slot_id: self.video_slot_id.clone(),
            cue_time: self.cue_time,
            end_time: self.end_time,
            state: self.state,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_without_gstreamer_init_or_runtime_available_is_handled() {
        let mut node = DestinationNode::new(
            "destination-test".to_string(),
            DestinationFamily::LocalPlayback,
            false,
            true,
        );

        node.connect_input("link-video", false, true).unwrap();
        if node.schedule(None, None).is_ok() {
            assert_eq!(node.state, State::Started);
            node.stop();
        } else {
            assert_eq!(node.state, State::Stopped);
            assert_eq!(
                node.pipeline.as_ref().map(|profile| profile.stage),
                Some(DestinationPipelineStage::Idle)
            );
        }
        assert!(node.live_pipeline.is_none());

        let cue = Utc::now() + chrono::Duration::seconds(15);
        if node.schedule(Some(cue), None).is_ok() {
            assert_eq!(node.state, State::Initial);
            node.stop();
        } else {
            assert_eq!(node.state, State::Stopped);
            assert_eq!(
                node.pipeline.as_ref().map(|profile| profile.stage),
                Some(DestinationPipelineStage::Idle)
            );
        }
        assert!(node.live_pipeline.is_none());
    }

    #[test]
    fn advance_schedule_starts_at_cue_time() {
        let mut node = DestinationNode::new(
            "destination-test".to_string(),
            DestinationFamily::LocalPlayback,
            false,
            true,
        );

        node.connect_input("link-video", false, true).unwrap();
        let cue = Utc::now() + chrono::Duration::seconds(30);
        assert!(node.schedule(Some(cue), None).is_ok());
        assert_eq!(node.state, State::Initial);

        node.advance_schedule(cue + chrono::Duration::seconds(1));
        assert_eq!(node.state, State::Started);
        assert_eq!(
            node.pipeline.as_ref().map(|profile| profile.stage),
            Some(DestinationPipelineStage::Playing)
        );
    }

    #[test]
    fn schedule_requires_connected_enabled_slots() {
        let mut node = DestinationNode::new(
            "destination-test".to_string(),
            DestinationFamily::LocalPlayback,
            true,
            true,
        );

        let err = node.schedule(None, None).unwrap_err();
        assert!(err.contains("audio slot connected"));

        node.connect_input("audio-link", true, false).unwrap();
        let err = node.schedule(None, None).unwrap_err();
        assert!(err.contains("video slot connected"));
    }

    #[test]
    fn connect_input_rejects_duplicate_media_slots() {
        let mut node = DestinationNode::new(
            "destination-test".to_string(),
            DestinationFamily::LocalPlayback,
            true,
            true,
        );

        node.connect_input("slot-1", true, true).unwrap();
        let err = node.connect_input("slot-2", true, false).unwrap_err();
        assert!(err.contains("already has an audio input slot"));

        let err = node.connect_input("slot-3", false, true).unwrap_err();
        assert!(err.contains("already has a video input slot"));
    }

    #[test]
    fn disconnect_input_clears_only_matching_slot() {
        let mut node = DestinationNode::new(
            "destination-test".to_string(),
            DestinationFamily::LocalPlayback,
            true,
            true,
        );

        node.connect_input("slot-a", true, false).unwrap();
        node.connect_input("slot-v", false, true).unwrap();

        node.disconnect_input("slot-a");
        assert!(node.audio_slot_id.is_none());
        assert_eq!(node.video_slot_id.as_deref(), Some("slot-v"));

        node.disconnect_input("slot-v");
        assert!(node.video_slot_id.is_none());
    }

    #[test]
    fn as_info_reflects_current_slots_and_state() {
        let mut node = DestinationNode::new(
            "destination-test".to_string(),
            DestinationFamily::Udp {
                host: "127.0.0.1".to_string(),
            },
            true,
            false,
        );

        node.connect_input("slot-a", true, false).unwrap();
        node.state = State::Started;

        let info = node.as_info();
        match info {
            NodeInfo::Destination(dest) => {
                assert_eq!(dest.audio_slot_id.as_deref(), Some("slot-a"));
                assert!(dest.video_slot_id.is_none());
                assert_eq!(dest.state, State::Started);
            }
            other => panic!("expected destination info, got {other:?}"),
        }
    }
}
