use crate::migration::nodes::control::evaluate_control_points;
use crate::migration::protocol::{ControlPoint, MixerInfo, MixerSlotInfo, NodeInfo, State};
use chrono::{DateTime, Duration, Utc};
use gst::glib::types::Type;
use gst::prelude::*;
use gst_app::{AppSink, AppSrc};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use tracing::warn;

const PREROLL_LEAD_TIME_SECONDS: i64 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixerPipelineStage {
    Idle,
    Starting,
    Playing,
}

#[derive(Debug, Clone)]
pub struct MixerPipelineProfile {
    pub video_branch_elements: Vec<String>,
    pub audio_branch_elements: Vec<String>,
    pub width: i64,
    pub height: i64,
    pub sample_rate: i64,
    pub fallback_image: String,
    pub fallback_timeout_ms: i64,
    pub stage: MixerPipelineStage,
}

#[derive(Debug, Clone)]
pub struct LiveMixerSlot {
    pub video_appsrc: Option<AppSrc>,
    pub video_pad: Option<gst::Pad>,
    pub audio_appsrc: Option<AppSrc>,
    pub audio_pad: Option<gst::Pad>,
}

#[derive(Debug, Clone)]
pub struct LiveMixerPipeline {
    pub pipeline: gst::Pipeline,
    pub video_mixer: Option<gst::Element>,
    pub audio_mixer: Option<gst::Element>,
    pub video_capsfilter: Option<gst::Element>,
    pub audio_capsfilter: Option<gst::Element>,
    pub video_output_appsink: Option<AppSink>,
    pub audio_output_appsink: Option<AppSink>,
    pub slots: HashMap<String, LiveMixerSlot>,
}

impl MixerPipelineProfile {
    fn from_settings(
        settings: &HashMap<String, Value>,
        audio_enabled: bool,
        video_enabled: bool,
        stage: MixerPipelineStage,
    ) -> Self {
        let width = settings
            .get("width")
            .and_then(Value::as_i64)
            .or_else(|| {
                settings
                    .get("width")
                    .and_then(Value::as_f64)
                    .map(|v| v as i64)
            })
            .unwrap_or(1920);
        let height = settings
            .get("height")
            .and_then(Value::as_i64)
            .or_else(|| {
                settings
                    .get("height")
                    .and_then(Value::as_f64)
                    .map(|v| v as i64)
            })
            .unwrap_or(1080);
        let sample_rate = settings
            .get("sample-rate")
            .and_then(Value::as_i64)
            .or_else(|| {
                settings
                    .get("sample-rate")
                    .and_then(Value::as_f64)
                    .map(|v| v as i64)
            })
            .unwrap_or(48000);
        let fallback_timeout_ms = settings
            .get("fallback-timeout")
            .and_then(Value::as_i64)
            .or_else(|| {
                settings
                    .get("fallback-timeout")
                    .and_then(Value::as_f64)
                    .map(|v| v as i64)
            })
            .unwrap_or(500);
        let fallback_image = settings
            .get("fallback-image")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        let video_branch_elements = if video_enabled {
            vec![
                "compositor".to_string(),
                "capsfilter".to_string(),
                "queue".to_string(),
                "appsink".to_string(),
            ]
        } else {
            Vec::new()
        };

        let audio_branch_elements = if audio_enabled {
            vec![
                "audiomixer".to_string(),
                "audioconvert".to_string(),
                "audioresample".to_string(),
                "appsink".to_string(),
            ]
        } else {
            Vec::new()
        };

        Self {
            video_branch_elements,
            audio_branch_elements,
            width,
            height,
            sample_rate,
            fallback_image,
            fallback_timeout_ms,
            stage,
        }
    }
}

fn parse_slot_config_key(property: &str) -> Result<(bool, &str), String> {
    let split: Vec<&str> = property.splitn(2, "::").collect();
    match split.len() {
        2 => match split[0] {
            "video" => Ok((true, split[1])),
            "audio" => Ok((false, split[1])),
            _ => Err("Slot property media type must be one of [audio, video]".to_string()),
        },
        _ => Err("Slot property name must be in form media-type::property-name".to_string()),
    }
}

fn validate_setting_value(name: &str, value: &Value) -> Result<(), String> {
    match name {
        "width" | "height" | "sample-rate" | "fallback-timeout" => {
            if value.is_number() {
                Ok(())
            } else {
                Err(format!("Setting `{name}` expects a numeric value"))
            }
        }
        "fallback-image" => {
            if value.is_string() {
                Ok(())
            } else {
                Err("Setting `fallback-image` expects a string value".to_string())
            }
        }
        _ => Err(format!("No setting with name {name} on mixers")),
    }
}

fn validate_slot_value(is_video: bool, property: &str, value: &Value) -> Result<(), String> {
    let numeric = value.is_number();

    if is_video {
        match property {
            "x" | "y" | "width" | "height" | "zorder" | "alpha" => {
                if numeric {
                    Ok(())
                } else {
                    Err(format!("video::{property} expects a numeric value"))
                }
            }
            _ => Ok(()),
        }
    } else {
        match property {
            "volume" => {
                if numeric {
                    Ok(())
                } else {
                    Err("audio::volume expects a numeric value".to_string())
                }
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MixerNode {
    pub id: String,
    pub audio_enabled: bool,
    pub video_enabled: bool,
    pub slots: HashMap<String, MixerSlotInfo>,
    pub video_consumer_slot_ids: BTreeSet<String>,
    pub audio_consumer_slot_ids: BTreeSet<String>,
    pub cue_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub state: State,
    pub settings: HashMap<String, Value>,
    pub control_points: HashMap<String, Vec<ControlPoint>>,
    pub slot_settings: HashMap<String, HashMap<String, Value>>,
    pub slot_control_points: HashMap<String, HashMap<String, Vec<ControlPoint>>>,
    pub pipeline: MixerPipelineProfile,
    pub live_pipeline: Option<LiveMixerPipeline>,
    pub last_error: Option<String>,
}

impl MixerNode {
    fn gst_initialized() -> bool {
        unsafe { gst::ffi::gst_is_initialized() != 0 }
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

    fn make_appsrc(id: &str, slot_id: &str, media: &str) -> Result<AppSrc, String> {
        let element = Self::make_element(
            "appsrc",
            Some(&format!("mixer-{media}-appsrc-{id}-{slot_id}")),
        )?;
        let appsrc = element
            .downcast::<AppSrc>()
            .map_err(|_| format!("Failed to downcast appsrc for mixer `{id}` slot `{slot_id}`"))?;
        appsrc.set_property("is-live", true);
        appsrc.set_property("do-timestamp", true);
        appsrc.set_property_from_str("format", "time");
        appsrc.set_property("block", false);
        Ok(appsrc)
    }

    fn parse_i32(value: Option<&Value>) -> Option<i32> {
        value
            .and_then(Value::as_i64)
            .or_else(|| value.and_then(Value::as_f64).map(|v| v as i64))
            .map(|v| v as i32)
    }

    fn parse_f64(value: Option<&Value>) -> Option<f64> {
        value
            .and_then(Value::as_f64)
            .or_else(|| value.and_then(Value::as_i64).map(|v| v as f64))
    }

    fn parse_i64(value: Option<&Value>) -> Option<i64> {
        value
            .and_then(Value::as_i64)
            .or_else(|| value.and_then(Value::as_f64).map(|v| v as i64))
    }

    fn parse_u64(value: Option<&Value>) -> Option<u64> {
        value
            .and_then(Value::as_u64)
            .or_else(|| {
                value
                    .and_then(Value::as_i64)
                    .and_then(|v| u64::try_from(v).ok())
            })
            .or_else(|| {
                value
                    .and_then(Value::as_f64)
                    .and_then(|v| if v >= 0.0 { Some(v as u64) } else { None })
            })
    }

    fn slot_has_media(slot: &HashMap<String, Value>, media: &str) -> bool {
        slot.keys()
            .any(|key| key.starts_with(&format!("{media}::")))
    }

    fn current_width(&self) -> i32 {
        Self::parse_i32(self.settings.get("width")).unwrap_or(1920)
    }

    fn current_height(&self) -> i32 {
        Self::parse_i32(self.settings.get("height")).unwrap_or(1080)
    }

    fn current_sample_rate(&self) -> i32 {
        Self::parse_i32(self.settings.get("sample-rate")).unwrap_or(48000)
    }

    fn set_video_caps(capsfilter: &gst::Element, width: i32, height: i32) {
        let caps = gst::Caps::builder("video/x-raw")
            .field("width", &width)
            .field("height", &height)
            .field("framerate", &gst::Fraction::new(30, 1))
            .build();
        if capsfilter.has_property("caps") {
            capsfilter.set_property("caps", &caps);
        }
    }

    fn set_audio_caps(capsfilter: &gst::Element, sample_rate: i32) {
        let caps = gst::Caps::builder("audio/x-raw")
            .field("channels", &2i32)
            .field("rate", &sample_rate)
            .build();
        if capsfilter.has_property("caps") {
            capsfilter.set_property("caps", &caps);
        }
    }

    fn set_dynamic_pad_property(
        pad: &gst::Pad,
        property: &str,
        value: &Value,
    ) -> Result<(), String> {
        let Some(pspec) = pad.find_property(property) else {
            return Err(format!("Pad `{}` has no property `{property}`", pad.name()));
        };

        let expected_type = pspec.value_type();

        match expected_type {
            Type::BOOL => {
                let v = value
                    .as_bool()
                    .ok_or_else(|| format!("{property} expects a boolean value"))?;
                Self::set_pad_property_safe(pad, property, v, expected_type)
            }
            Type::STRING => {
                let v = value
                    .as_str()
                    .ok_or_else(|| format!("{property} expects a string value"))?;
                Self::set_pad_property_safe(pad, property, v, expected_type)
            }
            Type::I32 => {
                let raw = Self::parse_i64(Some(value))
                    .ok_or_else(|| format!("{property} expects a numeric value"))?;
                let v =
                    i32::try_from(raw).map_err(|_| format!("{property} is out of i32 range"))?;
                Self::set_pad_property_safe(pad, property, v, expected_type)
            }
            Type::U32 => {
                let raw = Self::parse_u64(Some(value))
                    .ok_or_else(|| format!("{property} expects an unsigned numeric value"))?;
                let v =
                    u32::try_from(raw).map_err(|_| format!("{property} is out of u32 range"))?;
                Self::set_pad_property_safe(pad, property, v, expected_type)
            }
            Type::I64 => {
                let v = Self::parse_i64(Some(value))
                    .ok_or_else(|| format!("{property} expects a numeric value"))?;
                Self::set_pad_property_safe(pad, property, v, expected_type)
            }
            Type::U64 => {
                let v = Self::parse_u64(Some(value))
                    .ok_or_else(|| format!("{property} expects an unsigned numeric value"))?;
                Self::set_pad_property_safe(pad, property, v, expected_type)
            }
            Type::F32 => {
                let v = Self::parse_f64(Some(value))
                    .ok_or_else(|| format!("{property} expects a numeric value"))?
                    as f32;
                Self::set_pad_property_safe(pad, property, v, expected_type)
            }
            Type::F64 => {
                let v = Self::parse_f64(Some(value))
                    .ok_or_else(|| format!("{property} expects a numeric value"))?;
                Self::set_pad_property_safe(pad, property, v, expected_type)
            }
            value_type if value_type.is_a(Type::ENUM) || value_type.is_a(Type::FLAGS) => {
                let v = value
                    .as_str()
                    .ok_or_else(|| format!("{property} expects a string enum/flag value"))?;
                Self::set_pad_property_from_str_safe(pad, property, v, expected_type)
            }
            _ => Err(format!(
                "Unsupported value type `{expected_type:?}` for pad property `{property}` on `{}`",
                pad.name()
            )),
        }
    }

    fn resolve_video_pad_property_name(pad: &gst::Pad, property: &str) -> Option<String> {
        if pad.find_property(property).is_some() {
            return Some(property.to_string());
        }

        let alias = match property {
            // Legacy compositor payloads use x/y while modern compositor pads often expose xpos/ypos.
            "x" => Some("xpos"),
            "y" => Some("ypos"),
            "xpos" => Some("x"),
            "ypos" => Some("y"),
            _ => None,
        };

        if let Some(alias_property) = alias {
            if pad.find_property(alias_property).is_some() {
                return Some(alias_property.to_string());
            }
        }

        None
    }

    fn set_pad_property_safe<T: Into<gst::glib::Value>>(
        pad: &gst::Pad,
        property: &str,
        value: T,
        expected_type: Type,
    ) -> Result<(), String> {
        let gvalue: gst::glib::Value = value.into();
        let actual_type = gvalue.type_();
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            pad.set_property(property, &gvalue);
        }))
        .map_err(|_| {
            format!(
                "Failed to set `{property}` on pad `{}`: value type `{actual_type:?}`, expected `{expected_type:?}`",
                pad.name()
            )
        })
    }

    fn set_pad_property_from_str_safe(
        pad: &gst::Pad,
        property: &str,
        value: &str,
        expected_type: Type,
    ) -> Result<(), String> {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            pad.set_property_from_str(property, value);
        }))
        .map_err(|_| {
            format!(
                "Failed to set `{property}` on pad `{}` from string value `{value}` (expected `{expected_type:?}`)",
                pad.name()
            )
        })
    }

    fn apply_video_slot_properties(
        pad: &gst::Pad,
        slot_settings: &HashMap<String, Value>,
    ) -> Result<(), String> {
        for (key, value) in slot_settings {
            if !key.starts_with("video::") {
                continue;
            }
            let prop = key.trim_start_matches("video::");
            if prop == "sizing-policy" {
                // Kept for legacy payload compatibility even when compositor pad does not expose this property.
                continue;
            }
            if let Some(resolved_prop) = Self::resolve_video_pad_property_name(pad, prop) {
                Self::set_dynamic_pad_property(pad, &resolved_prop, value)?;
            } else {
                warn!(
                    pad = %pad.name(),
                    property = %prop,
                    "Ignoring unsupported video pad property"
                );
            }
        }
        Ok(())
    }

    fn apply_audio_slot_properties(
        pad: &gst::Pad,
        slot_settings: &HashMap<String, Value>,
    ) -> Result<(), String> {
        for (key, value) in slot_settings {
            if !key.starts_with("audio::") {
                continue;
            }
            let prop = key.trim_start_matches("audio::");
            if pad.find_property(prop).is_some() {
                Self::set_dynamic_pad_property(pad, prop, value)?;
            } else {
                warn!(
                    pad = %pad.name(),
                    property = %prop,
                    "Ignoring unsupported audio pad property"
                );
            }
        }
        Ok(())
    }

    fn build_live_pipeline(&self) -> Result<LiveMixerPipeline, String> {
        let pipeline = gst::Pipeline::with_name(&format!("migration-mixer-{}", self.id));

        let mut live = LiveMixerPipeline {
            pipeline: pipeline.clone(),
            video_mixer: None,
            audio_mixer: None,
            video_capsfilter: None,
            audio_capsfilter: None,
            video_output_appsink: None,
            audio_output_appsink: None,
            slots: HashMap::new(),
        };

        if self.video_enabled {
            let mixer = Self::make_element("compositor", Some("compositor"))?;
            let capsfilter = Self::make_element("capsfilter", Some("mixer-video-capsfilter"))?;
            let sink = Self::make_element("appsink", Some("mixer-video-appsink"))?
                .downcast::<AppSink>()
                .map_err(|_| "Failed to downcast mixer video appsink".to_string())?;
            let base_src = Self::make_element("videotestsrc", Some("mixer-video-base-src"))?;
            let base_queue = Self::make_element("queue", Some("mixer-video-base-queue"))?;

            base_src.set_property("is-live", true);
            base_src.set_property_from_str("pattern", "black");
            if mixer.has_property("background") {
                mixer.set_property_from_str("background", "black");
            }

            pipeline
                .add(&mixer)
                .map_err(|err| format!("Failed to add compositor to mixer pipeline: {err:?}"))?;
            pipeline.add(&capsfilter).map_err(|err| {
                format!("Failed to add video capsfilter to mixer pipeline: {err:?}")
            })?;
            pipeline
                .add(sink.upcast_ref::<gst::Element>())
                .map_err(|err| format!("Failed to add video appsink to mixer pipeline: {err:?}"))?;
            pipeline.add(&base_src).map_err(|err| {
                format!("Failed to add video base source to mixer pipeline: {err:?}")
            })?;
            pipeline.add(&base_queue).map_err(|err| {
                format!("Failed to add video base queue to mixer pipeline: {err:?}")
            })?;

            Self::set_video_caps(&capsfilter, self.current_width(), self.current_height());

            base_src
                .link(&base_queue)
                .map_err(|err| format!("Failed to link video base source to queue: {err:?}"))?;
            let base_pad = mixer
                .request_pad_simple("sink_%u")
                .ok_or_else(|| "Failed to request base video pad on compositor".to_string())?;
            base_queue
                .static_pad("src")
                .ok_or_else(|| "Video base queue is missing src pad".to_string())?
                .link(&base_pad)
                .map_err(|err| format!("Failed to link video base queue to compositor: {err:?}"))?;

            gst::Element::link_many(
                [&mixer, &capsfilter, sink.upcast_ref::<gst::Element>()].as_slice(),
            )
            .map_err(|err| format!("Failed to link mixer video output chain: {err:?}"))?;

            live.video_mixer = Some(mixer);
            live.video_capsfilter = Some(capsfilter);
            live.video_output_appsink = Some(sink);
        }

        if self.audio_enabled {
            let mixer = Self::make_element("audiomixer", Some("audiomixer"))?;
            let aconv = Self::make_element("audioconvert", Some("mixer-audio-convert"))?;
            let aresample = Self::make_element("audioresample", Some("mixer-audio-resample"))?;
            let capsfilter = Self::make_element("capsfilter", Some("mixer-audio-capsfilter"))?;
            let sink = Self::make_element("appsink", Some("mixer-audio-appsink"))?
                .downcast::<AppSink>()
                .map_err(|_| "Failed to downcast mixer audio appsink".to_string())?;
            let base_src = Self::make_element("audiotestsrc", Some("mixer-audio-base-src"))?;
            let base_queue = Self::make_element("queue", Some("mixer-audio-base-queue"))?;

            base_src.set_property("is-live", true);
            base_src.set_property("volume", 0.0f64);

            pipeline
                .add(&mixer)
                .map_err(|err| format!("Failed to add audiomixer to mixer pipeline: {err:?}"))?;
            pipeline
                .add(&aconv)
                .map_err(|err| format!("Failed to add audio convert to mixer pipeline: {err:?}"))?;
            pipeline.add(&aresample).map_err(|err| {
                format!("Failed to add audio resample to mixer pipeline: {err:?}")
            })?;
            pipeline.add(&capsfilter).map_err(|err| {
                format!("Failed to add audio capsfilter to mixer pipeline: {err:?}")
            })?;
            pipeline
                .add(sink.upcast_ref::<gst::Element>())
                .map_err(|err| format!("Failed to add audio appsink to mixer pipeline: {err:?}"))?;
            pipeline.add(&base_src).map_err(|err| {
                format!("Failed to add audio base source to mixer pipeline: {err:?}")
            })?;
            pipeline.add(&base_queue).map_err(|err| {
                format!("Failed to add audio base queue to mixer pipeline: {err:?}")
            })?;

            Self::set_audio_caps(&capsfilter, self.current_sample_rate());

            base_src
                .link(&base_queue)
                .map_err(|err| format!("Failed to link audio base source to queue: {err:?}"))?;
            let base_pad = mixer
                .request_pad_simple("sink_%u")
                .ok_or_else(|| "Failed to request base audio pad on audiomixer".to_string())?;
            if base_pad.has_property("volume") {
                base_pad.set_property("volume", 0.0f64);
            }
            base_queue
                .static_pad("src")
                .ok_or_else(|| "Audio base queue is missing src pad".to_string())?
                .link(&base_pad)
                .map_err(|err| format!("Failed to link audio base queue to audiomixer: {err:?}"))?;

            gst::Element::link_many(
                [
                    &mixer,
                    &aconv,
                    &aresample,
                    &capsfilter,
                    sink.upcast_ref::<gst::Element>(),
                ]
                .as_slice(),
            )
            .map_err(|err| format!("Failed to link mixer audio output chain: {err:?}"))?;

            live.audio_mixer = Some(mixer);
            live.audio_capsfilter = Some(capsfilter);
            live.audio_output_appsink = Some(sink);
        }

        for (slot_id, slot_settings) in &self.slot_settings {
            let has_video = self.video_enabled && Self::slot_has_media(slot_settings, "video");
            let has_audio = self.audio_enabled && Self::slot_has_media(slot_settings, "audio");

            if !has_video && !has_audio {
                continue;
            }

            let mut live_slot = LiveMixerSlot {
                video_appsrc: None,
                video_pad: None,
                audio_appsrc: None,
                audio_pad: None,
            };

            if has_video {
                let appsrc = Self::make_appsrc(&self.id, slot_id, "video")?;
                let queue = Self::make_element(
                    "queue",
                    Some(&format!("mixer-slot-video-queue-{slot_id}")),
                )?;
                pipeline
                    .add(appsrc.upcast_ref::<gst::Element>())
                    .map_err(|err| {
                        format!("Failed to add video slot appsrc `{slot_id}`: {err:?}")
                    })?;
                pipeline.add(&queue).map_err(|err| {
                    format!("Failed to add video slot queue `{slot_id}`: {err:?}")
                })?;
                appsrc
                    .upcast_ref::<gst::Element>()
                    .link(&queue)
                    .map_err(|err| {
                        format!("Failed to link video slot appsrc `{slot_id}`: {err:?}")
                    })?;

                if let Some(video_mixer) = live.video_mixer.as_ref() {
                    let pad = video_mixer.request_pad_simple("sink_%u").ok_or_else(|| {
                        format!("Failed to request video mixer pad for slot `{slot_id}`")
                    })?;
                    queue
                        .static_pad("src")
                        .ok_or_else(|| format!("Video queue for slot `{slot_id}` has no src pad"))?
                        .link(&pad)
                        .map_err(|err| {
                            format!("Failed to link video slot `{slot_id}` to mixer: {err:?}")
                        })?;
                    Self::apply_video_slot_properties(&pad, slot_settings)?;
                    live_slot.video_pad = Some(pad);
                }
                live_slot.video_appsrc = Some(appsrc);
            }

            if has_audio {
                let appsrc = Self::make_appsrc(&self.id, slot_id, "audio")?;
                let queue = Self::make_element(
                    "queue",
                    Some(&format!("mixer-slot-audio-queue-{slot_id}")),
                )?;
                let aconv = Self::make_element(
                    "audioconvert",
                    Some(&format!("mixer-slot-audio-conv-{slot_id}")),
                )?;
                let aresample = Self::make_element(
                    "audioresample",
                    Some(&format!("mixer-slot-audio-resample-{slot_id}")),
                )?;
                let capsfilter = Self::make_element(
                    "capsfilter",
                    Some(&format!("mixer-slot-audio-caps-{slot_id}")),
                )?;
                Self::set_audio_caps(&capsfilter, self.current_sample_rate());

                pipeline
                    .add(appsrc.upcast_ref::<gst::Element>())
                    .map_err(|err| {
                        format!("Failed to add audio slot appsrc `{slot_id}`: {err:?}")
                    })?;
                pipeline.add(&queue).map_err(|err| {
                    format!("Failed to add audio slot queue `{slot_id}`: {err:?}")
                })?;
                pipeline.add(&aconv).map_err(|err| {
                    format!("Failed to add audio slot convert `{slot_id}`: {err:?}")
                })?;
                pipeline.add(&aresample).map_err(|err| {
                    format!("Failed to add audio slot resample `{slot_id}`: {err:?}")
                })?;
                pipeline.add(&capsfilter).map_err(|err| {
                    format!("Failed to add audio slot capsfilter `{slot_id}`: {err:?}")
                })?;

                gst::Element::link_many(
                    [
                        appsrc.upcast_ref::<gst::Element>(),
                        &aconv,
                        &aresample,
                        &capsfilter,
                        &queue,
                    ]
                    .as_slice(),
                )
                .map_err(|err| {
                    format!("Failed to link audio slot `{slot_id}` processing chain: {err:?}")
                })?;

                if let Some(audio_mixer) = live.audio_mixer.as_ref() {
                    let pad = audio_mixer.request_pad_simple("sink_%u").ok_or_else(|| {
                        format!("Failed to request audio mixer pad for slot `{slot_id}`")
                    })?;
                    queue
                        .static_pad("src")
                        .ok_or_else(|| format!("Audio queue for slot `{slot_id}` has no src pad"))?
                        .link(&pad)
                        .map_err(|err| {
                            format!("Failed to link audio slot `{slot_id}` to mixer: {err:?}")
                        })?;
                    Self::apply_audio_slot_properties(&pad, slot_settings)?;
                    live_slot.audio_pad = Some(pad);
                }
                live_slot.audio_appsrc = Some(appsrc);
            }

            live.slots.insert(slot_id.clone(), live_slot);
        }

        Ok(live)
    }

    fn apply_live_settings(&mut self) -> Result<(), String> {
        let width = self.current_width();
        let height = self.current_height();
        let sample_rate = self.current_sample_rate();

        if let Some(live) = self.live_pipeline.as_mut() {
            if let Some(video_caps) = live.video_capsfilter.as_ref() {
                Self::set_video_caps(video_caps, width, height);
            }
            if let Some(audio_caps) = live.audio_capsfilter.as_ref() {
                Self::set_audio_caps(audio_caps, sample_rate);
            }

            for (slot_id, live_slot) in &mut live.slots {
                if let Some(slot_settings) = self.slot_settings.get(slot_id) {
                    if let Some(pad) = live_slot.video_pad.as_ref() {
                        Self::apply_video_slot_properties(pad, slot_settings)?;
                    }
                    if let Some(pad) = live_slot.audio_pad.as_ref() {
                        Self::apply_audio_slot_properties(pad, slot_settings)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn teardown_live_pipeline(&mut self) {
        if let Some(live) = self.live_pipeline.take() {
            let _ = live.pipeline.set_state(gst::State::Null);
        }
    }

    fn live_slots_match_model(&self) -> bool {
        let Some(live) = self.live_pipeline.as_ref() else {
            return false;
        };
        live.slots.len() == self.slot_settings.len()
            && self
                .slot_settings
                .keys()
                .all(|slot_id| live.slots.contains_key(slot_id))
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
                        "Mixer {} pipeline error from {:?}: {} ({:?})",
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
            self.pipeline.stage = MixerPipelineStage::Idle;
            self.teardown_live_pipeline();
            return Err(err);
        }

        if saw_eos {
            self.state = State::Stopped;
            self.pipeline.stage = MixerPipelineStage::Idle;
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
            MixerPipelineStage::Idle => {
                self.teardown_live_pipeline();
                Ok(())
            }
            MixerPipelineStage::Starting | MixerPipelineStage::Playing => {
                if self.live_pipeline.is_none() || !self.live_slots_match_model() {
                    self.teardown_live_pipeline();
                    self.live_pipeline = Some(self.build_live_pipeline()?);
                }

                self.apply_live_settings()?;

                let target_state = if self.pipeline.stage == MixerPipelineStage::Starting {
                    gst::State::Paused
                } else {
                    gst::State::Playing
                };

                if let Some(live) = self.live_pipeline.as_ref() {
                    live.pipeline.set_state(target_state).map_err(|err| {
                        format!("Failed to set mixer pipeline state to {target_state:?}: {err:?}")
                    })?;
                }
                self.poll_bus_messages()
            }
        }
    }

    pub fn refresh(&mut self) -> Result<(), String> {
        let now = Utc::now();
        self.apply_control_points(now);
        self.advance_schedule(now);
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
            State::Initial | State::Stopping | State::Stopped => MixerPipelineStage::Idle,
            State::Starting => MixerPipelineStage::Starting,
            State::Started => MixerPipelineStage::Playing,
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

    pub fn new(
        id: String,
        config: Option<HashMap<String, Value>>,
        audio_enabled: bool,
        video_enabled: bool,
    ) -> Result<Self, String> {
        let mut settings = HashMap::from([
            ("width".to_string(), Value::from(1920)),
            ("height".to_string(), Value::from(1080)),
            ("sample-rate".to_string(), Value::from(48000)),
            ("fallback-image".to_string(), Value::from("")),
            ("fallback-timeout".to_string(), Value::from(500)),
        ]);

        if let Some(cfg) = config {
            for (key, value) in cfg {
                validate_setting_value(&key, &value)?;
                settings.insert(key, value);
            }
        }

        let pipeline = MixerPipelineProfile::from_settings(
            &settings,
            audio_enabled,
            video_enabled,
            MixerPipelineStage::Idle,
        );

        Ok(Self {
            id,
            audio_enabled,
            video_enabled,
            slots: HashMap::new(),
            video_consumer_slot_ids: BTreeSet::new(),
            audio_consumer_slot_ids: BTreeSet::new(),
            cue_time: None,
            end_time: None,
            state: State::Initial,
            settings,
            control_points: HashMap::new(),
            slot_settings: HashMap::new(),
            slot_control_points: HashMap::new(),
            pipeline,
            live_pipeline: None,
            last_error: None,
        })
    }

    fn default_slot_settings(&self, audio: bool, video: bool) -> HashMap<String, Value> {
        let mut defaults = HashMap::new();
        if video {
            let width = self
                .settings
                .get("width")
                .and_then(Value::as_i64)
                .or_else(|| {
                    self.settings
                        .get("width")
                        .and_then(Value::as_f64)
                        .map(|v| v as i64)
                })
                .unwrap_or(1920);
            let height = self
                .settings
                .get("height")
                .and_then(Value::as_i64)
                .or_else(|| {
                    self.settings
                        .get("height")
                        .and_then(Value::as_f64)
                        .map(|v| v as i64)
                })
                .unwrap_or(1080);

            defaults.insert("video::x".to_string(), Value::from(0));
            defaults.insert("video::y".to_string(), Value::from(0));
            defaults.insert("video::width".to_string(), Value::from(width));
            defaults.insert("video::height".to_string(), Value::from(height));
            defaults.insert("video::alpha".to_string(), Value::from(1.0));
            defaults.insert("video::zorder".to_string(), Value::from(0));
        }
        if audio {
            defaults.insert("audio::volume".to_string(), Value::from(1.0));
        }
        defaults
    }

    pub fn connect_output_consumer(&mut self, link_id: &str, audio: bool, video: bool) {
        if audio {
            self.audio_consumer_slot_ids.insert(link_id.to_string());
        }
        if video {
            self.video_consumer_slot_ids.insert(link_id.to_string());
        }
    }

    pub fn disconnect_output_consumer(&mut self, link_id: &str) {
        self.audio_consumer_slot_ids.remove(link_id);
        self.video_consumer_slot_ids.remove(link_id);
    }

    pub fn live_audio_output_appsink(&self) -> Option<AppSink> {
        self.live_pipeline
            .as_ref()
            .and_then(|live| live.audio_output_appsink.clone())
    }

    pub fn live_video_output_appsink(&self) -> Option<AppSink> {
        self.live_pipeline
            .as_ref()
            .and_then(|live| live.video_output_appsink.clone())
    }

    pub fn live_slot_audio_appsrc(&self, slot_id: &str) -> Option<AppSrc> {
        self.live_pipeline
            .as_ref()
            .and_then(|live| live.slots.get(slot_id))
            .and_then(|slot| slot.audio_appsrc.clone())
    }

    pub fn live_slot_video_appsrc(&self, slot_id: &str) -> Option<AppSrc> {
        self.live_pipeline
            .as_ref()
            .and_then(|live| live.slots.get(slot_id))
            .and_then(|slot| slot.video_appsrc.clone())
    }

    pub fn connect_input_slot(
        &mut self,
        link_id: &str,
        audio: bool,
        video: bool,
        slot_config: Option<HashMap<String, Value>>,
    ) -> Result<(), String> {
        let mut merged = self.default_slot_settings(audio, video);

        if let Some(cfg) = slot_config {
            for (key, value) in cfg {
                let (is_video, property) = parse_slot_config_key(&key)?;
                if is_video && !video {
                    return Err(format!(
                        "Cannot set {key} on link {link_id}; video is not enabled for this link"
                    ));
                }
                if !is_video && !audio {
                    return Err(format!(
                        "Cannot set {key} on link {link_id}; audio is not enabled for this link"
                    ));
                }
                validate_slot_value(is_video, property, &value)?;
                merged.insert(key, value);
            }
        }

        let volume = merged
            .get("audio::volume")
            .and_then(Value::as_f64)
            .unwrap_or(1.0);

        if audio || video {
            self.slots
                .entry(link_id.to_string())
                .or_insert(MixerSlotInfo { volume });
        }
        if let Some(slot) = self.slots.get_mut(link_id) {
            slot.volume = volume;
        }

        self.slot_settings.insert(link_id.to_string(), merged);
        self.slot_control_points
            .entry(link_id.to_string())
            .or_default();
        self.sync_live_pipeline()?;
        Ok(())
    }

    pub fn disconnect_input_slot(&mut self, link_id: &str) {
        self.slots.remove(link_id);
        self.slot_settings.remove(link_id);
        self.slot_control_points.remove(link_id);
        if let Err(err) = self.sync_live_pipeline() {
            self.last_error = Some(err);
        }
    }

    pub fn add_control_point(&mut self, property: &str, cp: ControlPoint) -> Result<(), String> {
        if !self.settings.contains_key(property) {
            return Err(format!(
                "Mixer {} has no setting with name {property}",
                self.id
            ));
        }
        validate_setting_value(property, &cp.value)?;
        let points = self.control_points.entry(property.to_string()).or_default();
        points.push(cp);
        points.sort();
        self.apply_control_points(Utc::now());
        self.sync_live_pipeline()?;
        Ok(())
    }

    pub fn remove_control_point(&mut self, controller_id: &str, property: &str) {
        if let Some(points) = self.control_points.get_mut(property) {
            points.retain(|point| point.id != controller_id);
        }
        self.apply_control_points(Utc::now());
        if let Err(err) = self.sync_live_pipeline() {
            self.last_error = Some(err);
        }
    }

    pub fn add_slot_control_point(
        &mut self,
        slot_id: &str,
        property: &str,
        cp: ControlPoint,
    ) -> Result<(), String> {
        if !self.slot_settings.contains_key(slot_id) {
            return Err(format!("Mixer {} has no slot with id {slot_id}", self.id));
        }
        let (is_video, prop_name) = parse_slot_config_key(property)?;
        validate_slot_value(is_video, prop_name, &cp.value)?;

        let slot = self
            .slot_control_points
            .entry(slot_id.to_string())
            .or_default();
        let points = slot.entry(property.to_string()).or_default();
        points.push(cp);
        points.sort();
        self.apply_control_points(Utc::now());
        self.sync_live_pipeline()?;
        Ok(())
    }

    pub fn remove_slot_control_point(
        &mut self,
        controller_id: &str,
        slot_id: &str,
        property: &str,
    ) {
        if let Some(slot) = self.slot_control_points.get_mut(slot_id) {
            if let Some(points) = slot.get_mut(property) {
                points.retain(|point| point.id != controller_id);
            }
        }
        self.apply_control_points(Utc::now());
        if let Err(err) = self.sync_live_pipeline() {
            self.last_error = Some(err);
        }
    }

    pub fn apply_control_points(&mut self, at: DateTime<Utc>) {
        let mixer_updates: Vec<(String, Value)> = self
            .control_points
            .iter()
            .filter_map(|(property, points)| {
                evaluate_control_points(points, at).map(|value| (property.clone(), value))
            })
            .collect();

        for (property, value) in mixer_updates {
            self.settings.insert(property, value);
        }

        let slot_updates: Vec<(String, String, Value)> = self
            .slot_control_points
            .iter()
            .flat_map(|(slot_id, properties)| {
                properties.iter().filter_map(|(property, points)| {
                    evaluate_control_points(points, at)
                        .map(|value| (slot_id.clone(), property.clone(), value))
                })
            })
            .collect();

        for (slot_id, property, value) in slot_updates {
            self.slot_settings
                .entry(slot_id.clone())
                .or_default()
                .insert(property.clone(), value.clone());

            if property == "audio::volume" {
                if let Some(slot) = self.slots.get_mut(&slot_id) {
                    slot.volume = value.as_f64().unwrap_or(slot.volume);
                }
            }
        }
    }

    pub fn schedule(
        &mut self,
        cue_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<(), String> {
        self.cue_time = cue_time;
        self.end_time = end_time;
        self.last_error = None;
        let now = Utc::now();
        self.apply_control_points(now);
        if matches!(
            self.state,
            State::Starting | State::Stopping | State::Stopped
        ) {
            self.state = State::Initial;
        }
        self.advance_schedule(now);

        self.pipeline = MixerPipelineProfile::from_settings(
            &self.settings,
            self.audio_enabled,
            self.video_enabled,
            self.pipeline.stage,
        );

        if let Err(err) = self.sync_live_pipeline() {
            self.last_error = Some(err.clone());
            self.state = State::Stopped;
            self.pipeline.stage = MixerPipelineStage::Idle;
            self.teardown_live_pipeline();
            return Err(err);
        }

        Ok(())
    }

    pub fn stop(&mut self) {
        self.teardown_live_pipeline();
        self.state = State::Stopped;
        self.pipeline.stage = MixerPipelineStage::Idle;
    }

    pub fn mark_error(&mut self, message: String) {
        self.last_error = Some(message);
    }

    pub fn as_info(&self) -> NodeInfo {
        NodeInfo::Mixer(MixerInfo {
            slots: self.slots.clone(),
            video_consumer_slot_ids: Some(self.video_consumer_slot_ids.iter().cloned().collect()),
            audio_consumer_slot_ids: Some(self.audio_consumer_slot_ids.iter().cloned().collect()),
            cue_time: self.cue_time,
            end_time: self.end_time,
            state: self.state,
            settings: self.settings.clone(),
            control_points: self.control_points.clone(),
            slot_settings: self.slot_settings.clone(),
            slot_control_points: self.slot_control_points.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn schedule_without_gstreamer_init_or_runtime_available_is_handled() {
        let mut node = MixerNode::new("mixer-test".to_string(), None, false, true).unwrap();
        node.connect_input_slot("slot-video", false, true, None)
            .unwrap();

        if node.schedule(None, None).is_ok() {
            assert_eq!(node.state, State::Started);
            node.stop();
        } else {
            assert_eq!(node.state, State::Stopped);
            assert_eq!(node.pipeline.stage, MixerPipelineStage::Idle);
        }
        assert!(node.live_pipeline.is_none());

        let cue = Utc::now() + Duration::seconds(20);
        if node.schedule(Some(cue), None).is_ok() {
            assert_eq!(node.state, State::Initial);
            node.stop();
        } else {
            assert_eq!(node.state, State::Stopped);
            assert_eq!(node.pipeline.stage, MixerPipelineStage::Idle);
        }
        assert!(node.live_pipeline.is_none());
    }

    #[test]
    fn new_rejects_invalid_setting_types() {
        let config = HashMap::from([("width".to_string(), json!("wide"))]);
        let err = MixerNode::new("mixer-test".to_string(), Some(config), true, true).unwrap_err();
        assert!(err.contains("expects a numeric value"));
    }

    #[test]
    fn connect_input_slot_applies_defaults_and_custom_values() {
        let mut node = MixerNode::new("mixer-test".to_string(), None, true, true).unwrap();
        let slot_config = HashMap::from([
            ("video::x".to_string(), json!(20)),
            ("audio::volume".to_string(), json!(0.4)),
        ]);

        node.connect_input_slot("slot-1", true, true, Some(slot_config))
            .unwrap();

        let slot = node.slots.get("slot-1").unwrap();
        assert!((slot.volume - 0.4).abs() < 0.0001);

        let settings = node.slot_settings.get("slot-1").unwrap();
        assert_eq!(settings.get("video::x"), Some(&json!(20)));
        assert_eq!(settings.get("audio::volume"), Some(&json!(0.4)));
        assert!(settings.contains_key("video::width"));
        assert!(settings.contains_key("video::height"));
    }

    #[test]
    fn connect_input_slot_validates_slot_config_keys_and_media() {
        let mut node = MixerNode::new("mixer-test".to_string(), None, true, true).unwrap();

        let bad_format = HashMap::from([("x".to_string(), json!(10))]);
        let err = node
            .connect_input_slot("slot-1", true, true, Some(bad_format))
            .unwrap_err();
        assert!(err.contains("must be in form media-type::property-name"));

        let bad_media = HashMap::from([("video::x".to_string(), json!(10))]);
        let err = node
            .connect_input_slot("slot-2", true, false, Some(bad_media))
            .unwrap_err();
        assert!(err.contains("video is not enabled"));

        let bad_value = HashMap::from([("audio::volume".to_string(), json!("loud"))]);
        let err = node
            .connect_input_slot("slot-3", true, false, Some(bad_value))
            .unwrap_err();
        assert!(err.contains("expects a numeric value"));
    }

    #[test]
    fn add_and_remove_control_point_updates_mixer_setting() {
        let mut node = MixerNode::new("mixer-test".to_string(), None, true, true).unwrap();
        let cp = ControlPoint {
            id: "cp-width".to_string(),
            time: Utc::now() - Duration::seconds(1),
            value: json!(1280),
            mode: crate::migration::protocol::ControlMode::Set,
        };

        node.add_control_point("width", cp).unwrap();
        assert_eq!(node.settings.get("width"), Some(&json!(1280)));
        assert_eq!(node.control_points.get("width").map(Vec::len), Some(1));

        node.remove_control_point("cp-width", "width");
        assert_eq!(node.control_points.get("width").map(Vec::len), Some(0));
    }

    #[test]
    fn add_control_point_rejects_unknown_property() {
        let mut node = MixerNode::new("mixer-test".to_string(), None, true, true).unwrap();
        let cp = ControlPoint {
            id: "cp-unknown".to_string(),
            time: Utc::now(),
            value: json!(1),
            mode: crate::migration::protocol::ControlMode::Set,
        };

        let err = node.add_control_point("not-a-setting", cp).unwrap_err();
        assert!(err.contains("has no setting"));
    }

    #[test]
    fn slot_control_points_update_slot_volume_and_can_be_removed() {
        let mut node = MixerNode::new("mixer-test".to_string(), None, true, false).unwrap();
        node.connect_input_slot("slot-1", true, false, None)
            .unwrap();

        let cp = ControlPoint {
            id: "cp-vol".to_string(),
            time: Utc::now() - Duration::seconds(1),
            value: json!(0.2),
            mode: crate::migration::protocol::ControlMode::Set,
        };
        node.add_slot_control_point("slot-1", "audio::volume", cp)
            .unwrap();

        let slot = node.slots.get("slot-1").unwrap();
        assert!((slot.volume - 0.2).abs() < 0.0001);
        assert_eq!(
            node.slot_control_points
                .get("slot-1")
                .and_then(|entry| entry.get("audio::volume"))
                .map(Vec::len),
            Some(1)
        );

        node.remove_slot_control_point("cp-vol", "slot-1", "audio::volume");
        assert_eq!(
            node.slot_control_points
                .get("slot-1")
                .and_then(|entry| entry.get("audio::volume"))
                .map(Vec::len),
            Some(0)
        );
    }

    #[test]
    fn output_consumer_bookkeeping_tracks_links() {
        let mut node = MixerNode::new("mixer-test".to_string(), None, true, true).unwrap();
        node.connect_output_consumer("out-av", true, true);
        node.connect_output_consumer("out-a", true, false);

        assert!(node.audio_consumer_slot_ids.contains("out-av"));
        assert!(node.audio_consumer_slot_ids.contains("out-a"));
        assert!(node.video_consumer_slot_ids.contains("out-av"));

        node.disconnect_output_consumer("out-av");
        assert!(!node.audio_consumer_slot_ids.contains("out-av"));
        assert!(!node.video_consumer_slot_ids.contains("out-av"));
    }

    #[test]
    fn disconnect_input_slot_removes_slot_models() {
        let mut node = MixerNode::new("mixer-test".to_string(), None, true, true).unwrap();
        node.connect_input_slot("slot-1", true, true, None).unwrap();
        assert!(node.slots.contains_key("slot-1"));
        assert!(node.slot_settings.contains_key("slot-1"));
        assert!(node.slot_control_points.contains_key("slot-1"));

        node.disconnect_input_slot("slot-1");
        assert!(!node.slots.contains_key("slot-1"));
        assert!(!node.slot_settings.contains_key("slot-1"));
        assert!(!node.slot_control_points.contains_key("slot-1"));
    }

    #[test]
    fn as_info_contains_mixer_slots_and_control_maps() {
        let mut node = MixerNode::new("mixer-test".to_string(), None, true, true).unwrap();
        node.connect_input_slot("slot-1", true, true, None).unwrap();
        node.connect_output_consumer("out-1", true, true);
        node.state = State::Started;

        let info = node.as_info();
        match info {
            NodeInfo::Mixer(mixer) => {
                assert_eq!(mixer.state, State::Started);
                assert!(mixer.slots.contains_key("slot-1"));
                assert!(mixer.slot_settings.contains_key("slot-1"));
                assert!(mixer
                    .audio_consumer_slot_ids
                    .unwrap_or_default()
                    .contains(&"out-1".to_string()));
            }
            other => panic!("expected mixer info, got {other:?}"),
        }
    }
}
