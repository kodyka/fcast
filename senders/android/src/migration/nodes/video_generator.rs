use crate::migration::protocol::{NodeInfo, SourceInfo, State};
use chrono::{DateTime, Duration, Utc};
use gst::prelude::*;
use gst_app::AppSink;
use std::collections::BTreeSet;

const PREROLL_LEAD_TIME_SECONDS: i64 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoGeneratorStage {
    Idle,
    Prerolling,
    Playing,
}

#[derive(Debug, Clone)]
pub struct VideoGeneratorPipelineProfile {
    pub elements: Vec<String>,
    pub pattern: String,
    pub is_live: bool,
    pub flip: bool,
    pub stage: VideoGeneratorStage,
}

#[derive(Debug, Clone)]
pub struct LiveVideoGeneratorPipeline {
    pub pipeline: gst::Pipeline,
    pub appsink: AppSink,
}

impl VideoGeneratorPipelineProfile {
    fn new() -> Self {
        Self {
            elements: vec![
                "videotestsrc".to_string(),
                "deinterlace".to_string(),
                "appsink".to_string(),
            ],
            pattern: "ball".to_string(),
            is_live: true,
            flip: true,
            stage: VideoGeneratorStage::Idle,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VideoGeneratorNode {
    pub id: String,
    pub audio_enabled: bool,
    pub video_enabled: bool,
    pub audio_consumer_slot_ids: BTreeSet<String>,
    pub video_consumer_slot_ids: BTreeSet<String>,
    pub cue_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub state: State,
    pub pipeline: VideoGeneratorPipelineProfile,
    pub live_pipeline: Option<LiveVideoGeneratorPipeline>,
    pub last_error: Option<String>,
}

impl VideoGeneratorNode {
    fn gst_initialized() -> bool {
        unsafe { gst::ffi::gst_is_initialized() != 0 }
    }

    pub fn new(id: String) -> Self {
        Self {
            id,
            audio_enabled: false,
            video_enabled: true,
            audio_consumer_slot_ids: BTreeSet::new(),
            video_consumer_slot_ids: BTreeSet::new(),
            cue_time: None,
            end_time: None,
            state: State::Initial,
            pipeline: VideoGeneratorPipelineProfile::new(),
            live_pipeline: None,
            last_error: None,
        }
    }

    fn make_element(element: &str, name: &str) -> Result<gst::Element, String> {
        gst::ElementFactory::make(element)
            .name(name)
            .build()
            .map_err(|err| format!("Failed to create element `{element}`: {}", &*err.message))
    }

    fn build_live_pipeline(
        id: &str,
        profile: &VideoGeneratorPipelineProfile,
    ) -> Result<LiveVideoGeneratorPipeline, String> {
        let pipeline = gst::Pipeline::with_name(&format!("migration-videogen-{id}"));

        let src = Self::make_element("videotestsrc", &format!("videogen-src-{id}"))?;
        src.set_property("flip", profile.flip);
        src.set_property("is-live", profile.is_live);
        src.set_property_from_str("pattern", &profile.pattern);

        let deinterlace = Self::make_element("deinterlace", &format!("videogen-deinterlace-{id}"))?;
        let appsink = Self::make_element("appsink", &format!("videogen-appsink-{id}"))?
            .downcast::<AppSink>()
            .map_err(|_| format!("Failed to downcast video generator appsink for `{id}`"))?;

        pipeline.add(&src).map_err(|err| {
            format!("Failed to add videotestsrc to video generator pipeline: {err:?}")
        })?;
        pipeline.add(&deinterlace).map_err(|err| {
            format!("Failed to add deinterlace to video generator pipeline: {err:?}")
        })?;
        pipeline
            .add(appsink.upcast_ref::<gst::Element>())
            .map_err(|err| format!("Failed to add appsink to video generator pipeline: {err:?}"))?;

        src.link(&deinterlace)
            .map_err(|err| format!("Failed to link videotestsrc->deinterlace: {err:?}"))?;
        deinterlace
            .link(appsink.upcast_ref::<gst::Element>())
            .map_err(|err| format!("Failed to link deinterlace->appsink: {err:?}"))?;

        Ok(LiveVideoGeneratorPipeline { pipeline, appsink })
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

        self.live_pipeline = Some(Self::build_live_pipeline(&self.id, &self.pipeline)?);
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
                        "Video generator {} pipeline error from {:?}: {} ({:?})",
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
            self.pipeline.stage = VideoGeneratorStage::Idle;
            self.state = State::Stopped;
            self.teardown_live_pipeline();
            return Err(err);
        }

        if saw_eos {
            self.pipeline.stage = VideoGeneratorStage::Idle;
            self.state = State::Stopped;
            self.teardown_live_pipeline();
        }

        Ok(())
    }

    fn sync_live_pipeline(&mut self) -> Result<(), String> {
        // Unit tests and host-only flows may call migration code before GStreamer init.
        if !Self::gst_initialized() {
            return Ok(());
        }

        self.poll_bus_messages()?;

        match self.pipeline.stage {
            VideoGeneratorStage::Idle => {
                self.teardown_live_pipeline();
                Ok(())
            }
            VideoGeneratorStage::Prerolling | VideoGeneratorStage::Playing => {
                self.ensure_live_pipeline()?;
                let target_state = if self.pipeline.stage == VideoGeneratorStage::Prerolling {
                    gst::State::Paused
                } else {
                    gst::State::Playing
                };

                if let Some(live) = self.live_pipeline.as_ref() {
                    live.pipeline
                        .set_state(target_state)
                        .map_err(|err| format!("Failed to set video generator pipeline state to {target_state:?}: {err:?}"))?;
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
            State::Initial | State::Stopping | State::Stopped => VideoGeneratorStage::Idle,
            State::Starting => VideoGeneratorStage::Prerolling,
            State::Started => VideoGeneratorStage::Playing,
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

    pub fn live_video_appsink(&self) -> Option<AppSink> {
        self.live_pipeline.as_ref().map(|live| live.appsink.clone())
    }

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
            self.pipeline.stage = VideoGeneratorStage::Idle;
            self.state = State::Stopped;
            self.teardown_live_pipeline();
            return Err(err);
        }

        Ok(())
    }

    pub fn stop(&mut self) {
        self.teardown_live_pipeline();
        self.pipeline.stage = VideoGeneratorStage::Idle;
        self.state = State::Stopped;
    }

    pub fn mark_error(&mut self, message: String) {
        self.last_error = Some(message);
    }

    // Old protocol has no dedicated VideoGenerator info variant.
    // We encode it as a synthetic SourceInfo for compatibility.
    pub fn as_compatible_source_info(&self) -> NodeInfo {
        NodeInfo::Source(SourceInfo {
            uri: format!("videogenerator://{}", self.id),
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
        let mut node = VideoGeneratorNode::new("video-gen-test".to_string());

        if node.schedule(None, None).is_ok() {
            assert_eq!(node.state, State::Started);
            assert_eq!(node.pipeline.stage, VideoGeneratorStage::Playing);
            node.stop();
        } else {
            assert_eq!(node.state, State::Stopped);
            assert_eq!(node.pipeline.stage, VideoGeneratorStage::Idle);
        }
        assert!(node.live_pipeline.is_none());

        let cue = Utc::now() + Duration::seconds(30);
        if node.schedule(Some(cue), None).is_ok() {
            assert_eq!(node.state, State::Initial);
            assert_eq!(node.pipeline.stage, VideoGeneratorStage::Idle);
            node.stop();
        } else {
            assert_eq!(node.state, State::Stopped);
            assert_eq!(node.pipeline.stage, VideoGeneratorStage::Idle);
        }
        assert!(node.live_pipeline.is_none());
    }

    #[test]
    fn advance_schedule_transitions_through_preroll_play_and_stop() {
        let mut node = VideoGeneratorNode::new("video-gen-test".to_string());
        let cue = Utc::now() + Duration::seconds(20);
        let end = cue + Duration::seconds(5);
        node.cue_time = Some(cue);
        node.end_time = Some(end);
        node.state = State::Initial;

        node.advance_schedule(cue - Duration::seconds(11));
        assert_eq!(node.state, State::Initial);
        assert_eq!(node.pipeline.stage, VideoGeneratorStage::Idle);

        node.advance_schedule(cue - Duration::seconds(9));
        assert_eq!(node.state, State::Starting);
        assert_eq!(node.pipeline.stage, VideoGeneratorStage::Prerolling);

        node.advance_schedule(cue + Duration::seconds(1));
        assert_eq!(node.state, State::Started);
        assert_eq!(node.pipeline.stage, VideoGeneratorStage::Playing);

        node.advance_schedule(end + Duration::seconds(1));
        assert_eq!(node.state, State::Stopped);
        assert_eq!(node.pipeline.stage, VideoGeneratorStage::Idle);
    }

    #[test]
    fn consumer_link_bookkeeping_tracks_audio_and_video() {
        let mut node = VideoGeneratorNode::new("video-gen-test".to_string());
        node.add_consumer_link("slot-audio-video", true, true);
        node.add_consumer_link("slot-video", false, true);

        assert!(node.audio_consumer_slot_ids.contains("slot-audio-video"));
        assert!(node.video_consumer_slot_ids.contains("slot-audio-video"));
        assert!(node.video_consumer_slot_ids.contains("slot-video"));

        node.remove_consumer_link("slot-audio-video");
        assert!(!node.audio_consumer_slot_ids.contains("slot-audio-video"));
        assert!(!node.video_consumer_slot_ids.contains("slot-audio-video"));
        assert!(node.video_consumer_slot_ids.contains("slot-video"));
    }

    #[test]
    fn compatible_info_uses_source_shape_and_uri_scheme() {
        let mut node = VideoGeneratorNode::new("video-gen-test".to_string());
        node.add_consumer_link("slot-1", false, true);
        node.state = State::Started;

        let info = node.as_compatible_source_info();
        match info {
            NodeInfo::Source(source) => {
                assert_eq!(source.uri, "videogenerator://video-gen-test");
                assert_eq!(source.state, State::Started);
                assert_eq!(
                    source.video_consumer_slot_ids.unwrap_or_default(),
                    vec!["slot-1".to_string()]
                );
            }
            other => panic!("expected source node info, got {other:?}"),
        }
    }
}
