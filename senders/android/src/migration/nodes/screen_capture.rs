use crate::{
    migration::protocol::{NodeInfo, SourceInfo, State},
    FRAME_PAIR,
};
use chrono::{DateTime, Duration, Utc};
use gst::prelude::*;
use gst_app::{AppSink, AppSrc};
use std::collections::BTreeSet;

const PREROLL_LEAD_TIME_SECONDS: i64 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenCapturePipelineStage {
    Idle,
    Prerolling,
    Playing,
}

#[derive(Debug, Clone)]
pub struct LiveScreenCapturePipeline {
    pub pipeline: gst::Pipeline,
    pub video_appsink: AppSink,
}

#[derive(Debug, Clone)]
pub struct ScreenCaptureNode {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub video_consumer_slot_ids: BTreeSet<String>,
    pub cue_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub state: State,
    pub stage: ScreenCapturePipelineStage,
    pub live_pipeline: Option<LiveScreenCapturePipeline>,
    pub last_error: Option<String>,
}

impl ScreenCaptureNode {
    fn gst_initialized() -> bool {
        unsafe { gst::ffi::gst_is_initialized() != 0 }
    }

    pub fn new(id: String, width: u32, height: u32, fps: u32) -> Self {
        Self {
            id,
            width,
            height,
            fps,
            video_consumer_slot_ids: BTreeSet::new(),
            cue_time: None,
            end_time: None,
            state: State::Initial,
            stage: ScreenCapturePipelineStage::Idle,
            live_pipeline: None,
            last_error: None,
        }
    }

    pub fn as_info(&self) -> NodeInfo {
        NodeInfo::Source(SourceInfo {
            uri: format!("screen://{}x{}@{}fps", self.width, self.height, self.fps),
            video_consumer_slot_ids: Some(self.video_consumer_slot_ids.iter().cloned().collect()),
            audio_consumer_slot_ids: None,
            cue_time: self.cue_time,
            end_time: self.end_time,
            state: self.state,
        })
    }

    pub fn schedule(
        &mut self,
        cue_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<(), String> {
        self.cue_time = cue_time;
        self.end_time = end_time;
        Ok(())
    }

    pub fn add_consumer_link(&mut self, link_id: &str, _audio: bool, video: bool) {
        if video {
            self.video_consumer_slot_ids.insert(link_id.to_string());
        }
    }

    pub fn remove_consumer_link(&mut self, link_id: &str) {
        self.video_consumer_slot_ids.remove(link_id);
    }

    pub fn live_video_appsink(&self) -> Option<AppSink> {
        self.live_pipeline
            .as_ref()
            .map(|live| live.video_appsink.clone())
    }

    pub fn stop(&mut self) {
        self.state = State::Stopped;
        self.stage = ScreenCapturePipelineStage::Idle;
        self.teardown_live_pipeline();
    }

    pub fn mark_error(&mut self, message: String) {
        self.last_error = Some(message);
        self.stop();
    }

    pub fn refresh(&mut self) -> Result<(), String> {
        self.advance_schedule(Utc::now());
        self.sync_live_pipeline()
    }

    fn teardown_live_pipeline(&mut self) {
        if let Some(live) = self.live_pipeline.take() {
            let _ = live.pipeline.set_state(gst::State::Null);
        }
    }

    fn build_live_pipeline(&self) -> Result<LiveScreenCapturePipeline, String> {
        let pipeline = gst::Pipeline::with_name(&format!("migration-screen-capture-{}", self.id));
        let appsrc = AppSrc::builder()
            .name(format!("screen-capture-appsrc-{}", self.id))
            .format(gst::Format::Time)
            .is_live(true)
            .do_timestamp(true)
            .stream_type(gst_app::AppStreamType::Stream)
            .caps(
                &gst::Caps::builder("video/x-raw")
                    .field("format", "I420")
                    .field("width", self.width as i32)
                    .field("height", self.height as i32)
                    .field("framerate", gst::Fraction::new(self.fps as i32, 1))
                    .build(),
            )
            .build();

        let videoconvert = gst::ElementFactory::make("videoconvert")
            .name(format!("screen-capture-videoconvert-{}", self.id))
            .build()
            .map_err(|err| format!("Failed to create videoconvert: {}", err.message))?;

        let appsink = gst::ElementFactory::make("appsink")
            .name(format!("screen-capture-video-appsink-{}", self.id))
            .property("sync", false)
            .build()
            .map_err(|err| format!("Failed to create appsink: {}", err.message))?
            .downcast::<AppSink>()
            .map_err(|_| "Failed to downcast screen capture appsink".to_string())?;

        pipeline
            .add_many([
                appsrc.upcast_ref::<gst::Element>(),
                &videoconvert,
                appsink.upcast_ref::<gst::Element>(),
            ])
            .map_err(|err| format!("Failed to add screen capture pipeline elements: {err:?}"))?;
        gst::Element::link_many([
            appsrc.upcast_ref::<gst::Element>(),
            &videoconvert,
            appsink.upcast_ref::<gst::Element>(),
        ])
        .map_err(|err| format!("Failed to link screen capture pipeline elements: {err:?}"))?;

        Self::wire_need_data(&appsrc);

        Ok(LiveScreenCapturePipeline {
            pipeline,
            video_appsink: appsink,
        })
    }

    fn ensure_live_pipeline(&mut self) -> Result<(), String> {
        if self.live_pipeline.is_none() {
            self.live_pipeline = Some(self.build_live_pipeline()?);
        }
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
                        "ScreenCapture {} pipeline error from {:?}: {} ({:?})",
                        self.id,
                        err.src().map(|src| src.path_string()),
                        err.error(),
                        err.debug()
                    ));
                }
                _ => {}
            }
        }

        if let Some(err) = last_error {
            self.last_error = Some(err.clone());
            self.stage = ScreenCapturePipelineStage::Idle;
            self.state = State::Stopped;
            self.teardown_live_pipeline();
            return Err(err);
        }

        if saw_eos {
            self.stage = ScreenCapturePipelineStage::Idle;
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

        match self.stage {
            ScreenCapturePipelineStage::Idle => {
                self.teardown_live_pipeline();
                Ok(())
            }
            ScreenCapturePipelineStage::Prerolling | ScreenCapturePipelineStage::Playing => {
                self.ensure_live_pipeline()?;

                let target_state = if self.stage == ScreenCapturePipelineStage::Prerolling {
                    gst::State::Paused
                } else {
                    gst::State::Playing
                };

                if let Some(live) = self.live_pipeline.as_ref() {
                    live.pipeline.set_state(target_state).map_err(|err| {
                        format!(
                            "Failed to set screen capture pipeline state to {target_state:?}: {err:?}"
                        )
                    })?;
                }

                self.poll_bus_messages()
            }
        }
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
                if self.cue_time.is_none_or(|cue| now >= cue) {
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
        self.stage = match self.state {
            State::Initial | State::Stopping | State::Stopped => ScreenCapturePipelineStage::Idle,
            State::Starting => ScreenCapturePipelineStage::Prerolling,
            State::Started => ScreenCapturePipelineStage::Playing,
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

        let old_stage = self.stage;
        self.apply_state_to_stage();
        changed || old_stage != self.stage
    }

    fn wire_need_data(appsrc: &AppSrc) {
        let mut caps = None::<gst::Caps>;
        appsrc.set_callbacks(
            gst_app::AppSrcCallbacks::builder()
                .need_data(move |appsrc, _| {
                    let frame = {
                        let (lock, cvar) = &*FRAME_PAIR;
                        let mut frame = lock.lock();
                        while (*frame).is_none() {
                            cvar.wait_for(&mut frame, std::time::Duration::from_millis(100));
                        }
                        (*frame).take()
                    };

                    let Some(frame) = frame else {
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
                        Some(old_caps) if *old_caps == now_caps => {}
                        _ => {
                            appsrc.set_caps(Some(&now_caps));
                            caps = Some(now_caps);
                        }
                    }

                    let _ = appsrc.push_buffer(frame.into_buffer());
                })
                .build(),
        );
    }
}
