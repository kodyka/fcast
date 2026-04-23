use crate::migration::protocol::{NodeInfo, SourceInfo, State};
use chrono::{DateTime, Duration, Utc};
use gst::prelude::*;
use gst_app::AppSink;
use std::collections::BTreeSet;

const PREROLL_LEAD_TIME_SECONDS: i64 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourcePipelineStage {
    Idle,
    Prerolling,
    Playing,
}

#[derive(Debug, Clone)]
pub struct SourcePipelineProfile {
    pub uri: String,
    pub manual_unblock: bool,
    pub immediate_fallback: bool,
    pub elements: Vec<String>,
    pub stage: SourcePipelineStage,
}

#[derive(Debug, Clone)]
pub struct LiveSourcePipeline {
    pub pipeline: gst::Pipeline,
    pub source: gst::Element,
    pub uses_fallbacksrc: bool,
    pub audio_appsink: Option<AppSink>,
    pub video_appsink: Option<AppSink>,
}

impl SourcePipelineProfile {
    fn new(uri: String) -> Self {
        Self {
            uri,
            manual_unblock: true,
            immediate_fallback: true,
            elements: vec![
                "fallbacksrc".to_string(),
                "deinterlace".to_string(),
                "audioconvert".to_string(),
                "level".to_string(),
                "appsink".to_string(),
            ],
            stage: SourcePipelineStage::Idle,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SourceNode {
    pub id: String,
    pub uri: String,
    pub audio_enabled: bool,
    pub video_enabled: bool,
    pub audio_consumer_slot_ids: BTreeSet<String>,
    pub video_consumer_slot_ids: BTreeSet<String>,
    pub cue_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub state: State,
    pub pipeline: SourcePipelineProfile,
    pub live_pipeline: Option<LiveSourcePipeline>,
    pub last_error: Option<String>,
}

impl SourceNode {
    fn gst_initialized() -> bool {
        unsafe { gst::ffi::gst_is_initialized() != 0 }
    }

    pub fn new(id: String, uri: String, audio_enabled: bool, video_enabled: bool) -> Self {
        Self {
            id,
            uri: uri.clone(),
            audio_enabled,
            video_enabled,
            audio_consumer_slot_ids: BTreeSet::new(),
            video_consumer_slot_ids: BTreeSet::new(),
            cue_time: None,
            end_time: None,
            state: State::Initial,
            pipeline: SourcePipelineProfile::new(uri),
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

    fn build_source_element(
        id: &str,
        uri: &str,
        profile: &SourcePipelineProfile,
        audio_enabled: bool,
        video_enabled: bool,
    ) -> Result<(gst::Element, bool), String> {
        match Self::make_element("fallbacksrc", Some(&format!("source-fallbacksrc-{id}"))) {
            Ok(src) => {
                src.set_property("uri", uri);
                src.set_property("manual-unblock", profile.manual_unblock);
                src.set_property("immediate-fallback", profile.immediate_fallback);
                src.set_property("enable-audio", audio_enabled);
                src.set_property("enable-video", video_enabled);
                Ok((src, true))
            }
            Err(fallback_error) => {
                let src = Self::make_element("uridecodebin", Some(&format!("source-uridecodebin-{id}")))
                    .map_err(|decode_error| {
                        format!(
                            "Unable to create source element (fallbacksrc + uridecodebin failed): {fallback_error}; {decode_error}"
                        )
                    })?;
                src.set_property("uri", uri);
                Ok((src, false))
            }
        }
    }

    fn detect_pad_kind(pad: &gst::Pad) -> Option<&'static str> {
        if let Some(caps) = pad.current_caps().or_else(|| Some(pad.query_caps(None))) {
            if let Some(structure) = caps.structure(0) {
                let name = structure.name();
                if name.starts_with("video/") {
                    return Some("video");
                }
                if name.starts_with("audio/") {
                    return Some("audio");
                }
            }
        }

        let pad_name = pad.name();
        if pad_name.contains("video") {
            Some("video")
        } else if pad_name.contains("audio") {
            Some("audio")
        } else {
            None
        }
    }

    fn connect_video_pad(
        pipeline: &gst::Pipeline,
        pad: &gst::Pad,
        sink: &gst::Element,
    ) -> Result<(), String> {
        let sink_pad = sink
            .static_pad("sink")
            .ok_or_else(|| "Video sink element is missing sink pad".to_string())?;

        if sink_pad.is_linked() {
            return Ok(());
        }

        let deinterlace = Self::make_element("deinterlace", None)?;
        pipeline
            .add(&deinterlace)
            .map_err(|err| format!("Failed to add deinterlace to source pipeline: {err:?}"))?;
        deinterlace.sync_state_with_parent().map_err(|err| {
            format!("Failed to sync deinterlace with source pipeline state: {err:?}")
        })?;

        let deinterlace_sink = deinterlace
            .static_pad("sink")
            .ok_or_else(|| "deinterlace is missing sink pad".to_string())?;
        pad.link(&deinterlace_sink)
            .map_err(|err| format!("Failed to link source video pad to deinterlace: {err:?}"))?;
        deinterlace
            .link(sink)
            .map_err(|err| format!("Failed to link deinterlace to video appsink: {err:?}"))?;
        Ok(())
    }

    fn connect_audio_pad(
        pipeline: &gst::Pipeline,
        pad: &gst::Pad,
        sink: &gst::Element,
    ) -> Result<(), String> {
        let sink_pad = sink
            .static_pad("sink")
            .ok_or_else(|| "Audio sink element is missing sink pad".to_string())?;

        if sink_pad.is_linked() {
            return Ok(());
        }

        let aconv = Self::make_element("audioconvert", None)?;
        let level = Self::make_element("level", None)?;

        pipeline
            .add(&aconv)
            .map_err(|err| format!("Failed to add audioconvert to source pipeline: {err:?}"))?;
        pipeline
            .add(&level)
            .map_err(|err| format!("Failed to add level to source pipeline: {err:?}"))?;
        aconv.sync_state_with_parent().map_err(|err| {
            format!("Failed to sync audioconvert with source pipeline state: {err:?}")
        })?;
        level
            .sync_state_with_parent()
            .map_err(|err| format!("Failed to sync level with source pipeline state: {err:?}"))?;

        let aconv_sink = aconv
            .static_pad("sink")
            .ok_or_else(|| "audioconvert is missing sink pad".to_string())?;
        pad.link(&aconv_sink)
            .map_err(|err| format!("Failed to link source audio pad to audioconvert: {err:?}"))?;
        aconv
            .link(&level)
            .map_err(|err| format!("Failed to link audioconvert to level: {err:?}"))?;
        level
            .link(sink)
            .map_err(|err| format!("Failed to link level to audio appsink: {err:?}"))?;
        Ok(())
    }

    fn build_live_pipeline(&self) -> Result<LiveSourcePipeline, String> {
        let pipeline = gst::Pipeline::with_name(&format!("migration-source-{}", self.id));
        let (source, uses_fallbacksrc) = Self::build_source_element(
            &self.id,
            &self.uri,
            &self.pipeline,
            self.audio_enabled,
            self.video_enabled,
        )?;

        pipeline
            .add(&source)
            .map_err(|err| format!("Failed to add source element to source pipeline: {err:?}"))?;

        let audio_sink = if self.audio_enabled {
            let sink = Self::make_element(
                "appsink",
                Some(&format!("source-audio-appsink-{}", self.id)),
            )?
            .downcast::<AppSink>()
            .map_err(|_| {
                format!(
                    "Failed to downcast source audio appsink for source `{}`",
                    self.id
                )
            })?;
            pipeline
                .add(sink.upcast_ref::<gst::Element>())
                .map_err(|err| format!("Failed to add source audio appsink: {err:?}"))?;
            Some(sink)
        } else {
            None
        };

        let video_sink = if self.video_enabled {
            let sink = Self::make_element(
                "appsink",
                Some(&format!("source-video-appsink-{}", self.id)),
            )?
            .downcast::<AppSink>()
            .map_err(|_| {
                format!(
                    "Failed to downcast source video appsink for source `{}`",
                    self.id
                )
            })?;
            pipeline
                .add(sink.upcast_ref::<gst::Element>())
                .map_err(|err| format!("Failed to add source video appsink: {err:?}"))?;
            Some(sink)
        } else {
            None
        };

        let pipeline_weak = pipeline.downgrade();
        let video_sink_for_pad = video_sink.clone();
        let audio_sink_for_pad = audio_sink.clone();
        source.connect_pad_added(move |_src, pad| {
            let Some(pipeline) = pipeline_weak.upgrade() else {
                return;
            };

            match Self::detect_pad_kind(pad) {
                Some("video") => {
                    if let Some(video_sink) = video_sink_for_pad.as_ref() {
                        let _ = Self::connect_video_pad(
                            &pipeline,
                            pad,
                            video_sink.upcast_ref::<gst::Element>(),
                        );
                    }
                }
                Some("audio") => {
                    if let Some(audio_sink) = audio_sink_for_pad.as_ref() {
                        let _ = Self::connect_audio_pad(
                            &pipeline,
                            pad,
                            audio_sink.upcast_ref::<gst::Element>(),
                        );
                    }
                }
                _ => {}
            }
        });

        Ok(LiveSourcePipeline {
            pipeline,
            source,
            uses_fallbacksrc,
            audio_appsink: audio_sink,
            video_appsink: video_sink,
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

        self.live_pipeline = Some(self.build_live_pipeline()?);
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
                        "Source {} pipeline error from {:?}: {} ({:?})",
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
            self.pipeline.stage = SourcePipelineStage::Idle;
            self.state = State::Stopped;
            self.teardown_live_pipeline();
            return Err(err);
        }

        if saw_eos {
            self.pipeline.stage = SourcePipelineStage::Idle;
            self.state = State::Stopped;
            self.teardown_live_pipeline();
        }

        Ok(())
    }

    fn sync_live_pipeline(&mut self) -> Result<(), String> {
        if !Self::gst_initialized() {
            return Ok(());
        }

        self.poll_bus_messages()?;

        match self.pipeline.stage {
            SourcePipelineStage::Idle => {
                self.teardown_live_pipeline();
                Ok(())
            }
            SourcePipelineStage::Prerolling | SourcePipelineStage::Playing => {
                self.ensure_live_pipeline()?;

                let mut target_state = if self.pipeline.stage == SourcePipelineStage::Prerolling {
                    gst::State::Paused
                } else {
                    gst::State::Playing
                };

                if let Some(live) = self.live_pipeline.as_ref() {
                    if self.pipeline.stage == SourcePipelineStage::Prerolling
                        && live.uses_fallbacksrc
                        && self.pipeline.manual_unblock
                    {
                        target_state = gst::State::Playing;
                    }

                    live.pipeline.set_state(target_state).map_err(|err| {
                        format!("Failed to set source pipeline state to {target_state:?}: {err:?}")
                    })?;

                    if self.pipeline.stage == SourcePipelineStage::Playing
                        && live.uses_fallbacksrc
                        && self.pipeline.manual_unblock
                    {
                        live.source.emit_by_name::<()>("unblock", &[]);
                    }
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
            State::Initial => match self.cue_time {
                Some(cue) => {
                    let preroll_at = cue - Duration::seconds(PREROLL_LEAD_TIME_SECONDS);
                    if now >= preroll_at {
                        Some(State::Starting)
                    } else {
                        None
                    }
                }
                None => Some(State::Started),
            },
            State::Starting => {
                if self.cue_time.map_or(true, |cue| now >= cue) {
                    Some(State::Started)
                } else {
                    None
                }
            }
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
        self.pipeline.stage = match self.state {
            State::Initial | State::Stopping | State::Stopped => SourcePipelineStage::Idle,
            State::Starting => SourcePipelineStage::Prerolling,
            State::Started => SourcePipelineStage::Playing,
        };
    }

    fn advance_schedule(&mut self, now: DateTime<Utc>) -> bool {
        let mut changed = false;
        while let Some(next_state) = self.schedule_transition_due(now) {
            if next_state == self.state {
                break;
            }
            self.state = next_state;
            changed = true;
        }

        let old_stage = self.pipeline.stage;
        self.apply_state_to_stage();
        changed || old_stage != self.pipeline.stage
    }

    pub fn add_consumer_link(&mut self, link_id: &str, audio: bool, video: bool) {
        if audio {
            self.audio_consumer_slot_ids.insert(link_id.to_string());
        }
        if video {
            self.video_consumer_slot_ids.insert(link_id.to_string());
        }
    }

    pub fn remove_consumer_link(&mut self, link_id: &str) {
        self.audio_consumer_slot_ids.remove(link_id);
        self.video_consumer_slot_ids.remove(link_id);
    }

    pub fn live_audio_appsink(&self) -> Option<AppSink> {
        self.live_pipeline
            .as_ref()
            .and_then(|live| live.audio_appsink.clone())
    }

    pub fn live_video_appsink(&self) -> Option<AppSink> {
        self.live_pipeline
            .as_ref()
            .and_then(|live| live.video_appsink.clone())
    }

    /// Mirrors old source scheduling semantics:
    /// - Initial until `cue - 10s`
    /// - Starting between `cue - 10s` and `cue` (preroll)
    /// - Started at/after `cue` (unblocked/playing)
    pub fn schedule(
        &mut self,
        cue_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<(), String> {
        self.cue_time = cue_time;
        self.end_time = end_time;
        self.last_error = None;
        if matches!(
            self.state,
            State::Starting | State::Stopping | State::Stopped
        ) {
            self.state = State::Initial;
        }
        self.advance_schedule(Utc::now());

        if let Err(err) = self.sync_live_pipeline() {
            self.last_error = Some(err.clone());
            self.pipeline.stage = SourcePipelineStage::Idle;
            self.state = State::Stopped;
            self.teardown_live_pipeline();
            return Err(err);
        }

        Ok(())
    }

    pub fn stop(&mut self) {
        self.teardown_live_pipeline();
        self.pipeline.stage = SourcePipelineStage::Idle;
        self.state = State::Stopped;
    }

    pub fn mark_error(&mut self, message: String) {
        self.last_error = Some(message);
    }

    pub fn as_info(&self) -> NodeInfo {
        NodeInfo::Source(SourceInfo {
            uri: self.uri.clone(),
            video_consumer_slot_ids: Some(self.video_consumer_slot_ids.iter().cloned().collect()),
            audio_consumer_slot_ids: Some(self.audio_consumer_slot_ids.iter().cloned().collect()),
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
        let mut node = SourceNode::new(
            "source-test".to_string(),
            "https://example.com/stream.mp4".to_string(),
            true,
            true,
        );

        if node.schedule(None, None).is_ok() {
            assert_eq!(node.state, State::Started);
            node.stop();
        } else {
            assert_eq!(node.state, State::Stopped);
            assert_eq!(node.pipeline.stage, SourcePipelineStage::Idle);
        }
        assert!(node.live_pipeline.is_none());

        let cue = Utc::now() + Duration::seconds(30);
        if node.schedule(Some(cue), None).is_ok() {
            assert_eq!(node.state, State::Initial);
            node.stop();
        } else {
            assert_eq!(node.state, State::Stopped);
            assert_eq!(node.pipeline.stage, SourcePipelineStage::Idle);
        }
        assert!(node.live_pipeline.is_none());
    }

    #[test]
    fn advance_schedule_transitions_to_stopped_after_end_time() {
        let mut node = SourceNode::new(
            "source-test".to_string(),
            "https://example.com/stream.mp4".to_string(),
            true,
            true,
        );

        let cue = Utc::now() - Duration::seconds(20);
        let end = Utc::now() - Duration::seconds(1);
        node.cue_time = Some(cue);
        node.end_time = Some(end);
        node.state = State::Initial;

        node.advance_schedule(Utc::now());

        assert_eq!(node.state, State::Stopped);
        assert_eq!(node.pipeline.stage, SourcePipelineStage::Idle);
    }

    #[test]
    fn consumer_link_bookkeeping_tracks_audio_and_video_links() {
        let mut node = SourceNode::new(
            "source-test".to_string(),
            "https://example.com/stream.mp4".to_string(),
            true,
            true,
        );

        node.add_consumer_link("slot-av", true, true);
        node.add_consumer_link("slot-a", true, false);
        node.add_consumer_link("slot-v", false, true);

        assert!(node.audio_consumer_slot_ids.contains("slot-av"));
        assert!(node.audio_consumer_slot_ids.contains("slot-a"));
        assert!(node.video_consumer_slot_ids.contains("slot-av"));
        assert!(node.video_consumer_slot_ids.contains("slot-v"));

        node.remove_consumer_link("slot-av");
        assert!(!node.audio_consumer_slot_ids.contains("slot-av"));
        assert!(!node.video_consumer_slot_ids.contains("slot-av"));
    }

    #[test]
    fn advance_schedule_enters_starting_during_preroll_window() {
        let mut node = SourceNode::new(
            "source-test".to_string(),
            "https://example.com/stream.mp4".to_string(),
            true,
            true,
        );

        let cue = Utc::now() + Duration::seconds(20);
        node.cue_time = Some(cue);
        node.state = State::Initial;

        node.advance_schedule(cue - Duration::seconds(15));
        assert_eq!(node.state, State::Initial);
        assert_eq!(node.pipeline.stage, SourcePipelineStage::Idle);

        node.advance_schedule(cue - Duration::seconds(5));
        assert_eq!(node.state, State::Starting);
        assert_eq!(node.pipeline.stage, SourcePipelineStage::Prerolling);
    }

    #[test]
    fn as_info_returns_current_state_and_slots() {
        let mut node = SourceNode::new(
            "source-test".to_string(),
            "https://example.com/stream.mp4".to_string(),
            true,
            true,
        );
        node.state = State::Started;
        node.add_consumer_link("slot-a", true, false);
        node.add_consumer_link("slot-v", false, true);

        let info = node.as_info();
        match info {
            NodeInfo::Source(source) => {
                assert_eq!(source.uri, "https://example.com/stream.mp4");
                assert_eq!(source.state, State::Started);
                assert!(source
                    .audio_consumer_slot_ids
                    .unwrap_or_default()
                    .contains(&"slot-a".to_string()));
                assert!(source
                    .video_consumer_slot_ids
                    .unwrap_or_default()
                    .contains(&"slot-v".to_string()));
            }
            other => panic!("expected source info, got {other:?}"),
        }
    }
}
