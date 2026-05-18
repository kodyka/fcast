use crate::migration::{
    media_bridge::StreamBridge,
    nodes::{DestinationNode, MixerNode, ScreenCaptureNode, SourceNode, VideoGeneratorNode},
    protocol::{Command, CommandResult, ControlPoint, Info, NodeInfo},
};
use chrono::{DateTime, Utc};
use gst_app::{AppSink, AppSrc};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct LinkRecord {
    src_id: String,
    sink_id: String,
    audio: bool,
    video: bool,
    config: Option<HashMap<String, Value>>,
}

#[derive(Debug, Clone)]
enum NodeRecord {
    Source(SourceNode),
    ScreenCapture(ScreenCaptureNode),
    Destination(DestinationNode),
    Mixer(MixerNode),
    VideoGenerator(VideoGeneratorNode),
}

impl NodeRecord {
    fn can_output_audio(&self) -> bool {
        match self {
            Self::Source(node) => node.audio_enabled,
            Self::ScreenCapture(_) => false,
            Self::Mixer(node) => node.audio_enabled,
            Self::VideoGenerator(node) => node.audio_enabled,
            Self::Destination(_) => false,
        }
    }

    fn can_output_video(&self) -> bool {
        match self {
            Self::Source(node) => node.video_enabled,
            Self::ScreenCapture(_) => true,
            Self::Mixer(node) => node.video_enabled,
            Self::VideoGenerator(node) => node.video_enabled,
            Self::Destination(_) => false,
        }
    }

    fn can_input_audio(&self) -> bool {
        match self {
            Self::Destination(node) => node.audio_enabled,
            Self::Mixer(node) => node.audio_enabled,
            Self::Source(_) | Self::ScreenCapture(_) | Self::VideoGenerator(_) => false,
        }
    }

    fn can_input_video(&self) -> bool {
        match self {
            Self::Destination(node) => node.video_enabled,
            Self::Mixer(node) => node.video_enabled,
            Self::Source(_) | Self::ScreenCapture(_) | Self::VideoGenerator(_) => false,
        }
    }

    fn to_info(&self) -> NodeInfo {
        match self {
            Self::Source(node) => node.as_info(),
            Self::ScreenCapture(node) => node.as_info(),
            Self::Destination(node) => node.as_info(),
            Self::Mixer(node) => node.as_info(),
            Self::VideoGenerator(node) => node.as_compatible_source_info(),
        }
    }

    fn set_schedule(
        &mut self,
        cue_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<(), String> {
        match self {
            Self::Source(node) => node.schedule(cue_time, end_time),
            Self::ScreenCapture(node) => node.schedule(cue_time, end_time),
            Self::Destination(node) => node.schedule(cue_time, end_time),
            Self::Mixer(node) => node.schedule(cue_time, end_time),
            Self::VideoGenerator(node) => node.schedule(cue_time, end_time),
        }
    }

    fn stop(&mut self) {
        match self {
            Self::Source(node) => node.stop(),
            Self::ScreenCapture(node) => node.stop(),
            Self::Destination(node) => node.stop(),
            Self::Mixer(node) => node.stop(),
            Self::VideoGenerator(node) => node.stop(),
        }
    }

    fn mark_error(&mut self, message: String) {
        match self {
            Self::Source(node) => node.mark_error(message),
            Self::ScreenCapture(node) => node.mark_error(message),
            Self::Destination(node) => node.mark_error(message),
            Self::Mixer(node) => node.mark_error(message),
            Self::VideoGenerator(node) => node.mark_error(message),
        }
    }

    fn add_consumer_link(&mut self, link_id: &str, audio: bool, video: bool) {
        match self {
            Self::Source(node) => node.add_consumer_link(link_id, audio, video),
            Self::ScreenCapture(node) => node.add_consumer_link(link_id, audio, video),
            Self::Mixer(node) => node.connect_output_consumer(link_id, audio, video),
            Self::VideoGenerator(node) => node.add_consumer_link(link_id, audio, video),
            Self::Destination(_) => {}
        }
    }

    fn remove_consumer_link(&mut self, link_id: &str) {
        match self {
            Self::Source(node) => node.remove_consumer_link(link_id),
            Self::ScreenCapture(node) => node.remove_consumer_link(link_id),
            Self::Mixer(node) => node.disconnect_output_consumer(link_id),
            Self::VideoGenerator(node) => node.remove_consumer_link(link_id),
            Self::Destination(_) => {}
        }
    }

    fn refresh_runtime(&mut self) {
        let result = match self {
            Self::Source(node) => node.refresh(),
            Self::ScreenCapture(node) => node.refresh(),
            Self::Destination(node) => node.refresh(),
            Self::Mixer(node) => node.refresh(),
            Self::VideoGenerator(node) => node.refresh(),
        };

        if let Err(err) = result {
            self.mark_error(err);
        }
    }

    fn output_audio_appsink(&self) -> Option<AppSink> {
        match self {
            Self::Source(node) => node.live_audio_appsink(),
            Self::ScreenCapture(_) => None,
            Self::Mixer(node) => node.live_audio_output_appsink(),
            Self::VideoGenerator(_) | Self::Destination(_) => None,
        }
    }

    fn output_video_appsink(&self) -> Option<AppSink> {
        match self {
            Self::Source(node) => node.live_video_appsink(),
            Self::ScreenCapture(node) => node.live_video_appsink(),
            Self::Mixer(node) => node.live_video_output_appsink(),
            Self::VideoGenerator(node) => node.live_video_appsink(),
            Self::Destination(_) => None,
        }
    }

    fn input_audio_appsrc(&self, link_id: &str) -> Option<AppSrc> {
        match self {
            Self::Destination(node) => node.live_audio_appsrc(),
            Self::Mixer(node) => node.live_slot_audio_appsrc(link_id),
            Self::Source(_) | Self::ScreenCapture(_) | Self::VideoGenerator(_) => None,
        }
    }

    fn input_video_appsrc(&self, link_id: &str) -> Option<AppSrc> {
        match self {
            Self::Destination(node) => node.live_video_appsrc(),
            Self::Mixer(node) => node.live_slot_video_appsrc(link_id),
            Self::Source(_) | Self::ScreenCapture(_) | Self::VideoGenerator(_) => None,
        }
    }
}

#[derive(Debug, Default)]
pub struct NodeManager {
    started: bool,
    nodes: HashMap<String, NodeRecord>,
    links: HashMap<String, LinkRecord>,
    media_bridges: HashMap<String, StreamBridge>,
}

impl NodeManager {
    fn gst_initialized() -> bool {
        unsafe { gst::ffi::gst_is_initialized() != 0 }
    }

    fn refresh_nodes(&mut self) {
        for node in self.nodes.values_mut() {
            node.refresh_runtime();
        }
    }

    fn bridge_key(src_id: &str, media: &str) -> String {
        format!("{src_id}:{media}")
    }

    fn remove_media_bridge_link(&mut self, src_id: &str, link_id: &str, audio: bool, video: bool) {
        if audio {
            let key = Self::bridge_key(src_id, "audio");
            let mut remove_bridge = false;
            if let Some(bridge) = self.media_bridges.get_mut(&key) {
                bridge.remove_consumer(link_id);
                if !bridge.has_consumers() {
                    bridge.clear();
                    remove_bridge = true;
                }
            }
            if remove_bridge {
                self.media_bridges.remove(&key);
            }
        }
        if video {
            let key = Self::bridge_key(src_id, "video");
            let mut remove_bridge = false;
            if let Some(bridge) = self.media_bridges.get_mut(&key) {
                bridge.remove_consumer(link_id);
                if !bridge.has_consumers() {
                    bridge.clear();
                    remove_bridge = true;
                }
            }
            if remove_bridge {
                self.media_bridges.remove(&key);
            }
        }
    }

    fn sync_media_links(&mut self) {
        if !Self::gst_initialized() {
            return;
        }

        let links = self
            .links
            .iter()
            .map(|(id, link)| (id.clone(), link.clone()))
            .collect::<Vec<_>>();

        for (link_id, link) in links {
            let Some(src_node) = self.nodes.get(&link.src_id) else {
                self.remove_media_bridge_link(&link.src_id, &link_id, link.audio, link.video);
                continue;
            };
            let Some(sink_node) = self.nodes.get(&link.sink_id) else {
                self.remove_media_bridge_link(&link.src_id, &link_id, link.audio, link.video);
                continue;
            };

            if link.audio {
                let key = Self::bridge_key(&link.src_id, "audio");
                let bridge = self.media_bridges.entry(key).or_default();
                if let Some(producer_sink) = src_node.output_audio_appsink() {
                    bridge.attach_sink(&producer_sink);
                }
                if let Some(consumer_src) = sink_node.input_audio_appsrc(&link_id) {
                    bridge.add_consumer(&link_id, &consumer_src);
                } else {
                    bridge.remove_consumer(&link_id);
                }
            }

            if link.video {
                let key = Self::bridge_key(&link.src_id, "video");
                let bridge = self.media_bridges.entry(key).or_default();
                if let Some(producer_sink) = src_node.output_video_appsink() {
                    bridge.attach_sink(&producer_sink);
                }
                if let Some(consumer_src) = sink_node.input_video_appsrc(&link_id) {
                    bridge.add_consumer(&link_id, &consumer_src);
                } else {
                    bridge.remove_consumer(&link_id);
                }
            }
        }

        let empty_bridge_keys = self
            .media_bridges
            .iter()
            .filter_map(|(key, bridge)| {
                if bridge.has_consumers() {
                    None
                } else {
                    Some(key.clone())
                }
            })
            .collect::<Vec<_>>();

        for key in empty_bridge_keys {
            if let Some(mut bridge) = self.media_bridges.remove(&key) {
                bridge.clear();
            }
        }
    }

    pub fn start(&mut self) {
        self.started = true;
        self.refresh_nodes();
        self.sync_media_links();
    }

    pub fn tick(&mut self) {
        if !self.started {
            return;
        }
        self.refresh_nodes();
        self.sync_media_links();
    }

    pub fn shutdown(&mut self) {
        self.started = false;
        for node in self.nodes.values_mut() {
            node.stop();
        }
        self.nodes.clear();
        self.links.clear();
        for bridge in self.media_bridges.values_mut() {
            bridge.clear();
        }
        self.media_bridges.clear();
    }

    pub fn dispatch(&mut self, command: Command) -> CommandResult {
        if !self.started {
            self.started = true;
        }

        self.refresh_nodes();

        let (result, should_sync) = match command {
            Command::CreateVideoGenerator { id } => (self.create_video_generator(id), true),
            Command::CreateSource {
                id,
                uri,
                audio,
                video,
            } => (self.create_source(id, uri, audio, video), true),
            Command::CreateScreenCaptureSource {
                id,
                width,
                height,
                fps,
            } => (
                self.create_screen_capture_source(id, width, height, fps),
                true,
            ),
            Command::CreateDestination {
                id,
                family,
                audio,
                video,
            } => (self.create_destination(id, family, audio, video), true),
            Command::CreateMixer {
                id,
                config,
                audio,
                video,
            } => (self.create_mixer(id, config, audio, video), true),
            Command::Connect {
                link_id,
                src_id,
                sink_id,
                audio,
                video,
                config,
            } => (
                self.connect(link_id, src_id, sink_id, audio, video, config),
                true,
            ),
            Command::Disconnect { link_id } => (self.disconnect(&link_id), true),
            Command::Start {
                id,
                cue_time,
                end_time,
            } => (self.schedule_node(&id, cue_time, end_time), true),
            Command::Reschedule {
                id,
                cue_time,
                end_time,
            } => (self.schedule_node(&id, cue_time, end_time), true),
            Command::Remove { id } => (self.remove_node(&id), true),
            Command::GetInfo { id } => (self.get_info(id.as_ref()), false),
            Command::AddControlPoint {
                controllee_id,
                property,
                control_point,
            } => (
                self.add_control_point(&controllee_id, &property, control_point),
                true,
            ),
            Command::RemoveControlPoint {
                id,
                controllee_id,
                property,
            } => (
                self.remove_control_point(&id, &controllee_id, &property),
                true,
            ),
        };

        if should_sync {
            self.sync_media_links();
        }

        self.refresh_nodes();

        result
    }

    fn ensure_unique_id(&self, id: &str) -> Result<(), String> {
        if self.nodes.contains_key(id) {
            return Err(format!("A node already exists with id {id}"));
        }
        Ok(())
    }

    fn create_video_generator(&mut self, id: String) -> CommandResult {
        if let Err(err) = self.ensure_unique_id(&id) {
            return CommandResult::Error(err);
        }

        self.nodes.insert(
            id.clone(),
            NodeRecord::VideoGenerator(VideoGeneratorNode::new(id)),
        );
        CommandResult::Success
    }

    fn create_source(
        &mut self,
        id: String,
        uri: String,
        audio: bool,
        video: bool,
    ) -> CommandResult {
        if let Err(err) = self.ensure_unique_id(&id) {
            return CommandResult::Error(err);
        }
        if !audio && !video {
            return CommandResult::Error(format!(
                "Source with id {id} must have either audio or video enabled"
            ));
        }

        self.nodes.insert(
            id.clone(),
            NodeRecord::Source(SourceNode::new(id, uri, audio, video)),
        );
        CommandResult::Success
    }

    fn create_screen_capture_source(
        &mut self,
        id: String,
        width: u32,
        height: u32,
        fps: u32,
    ) -> CommandResult {
        if let Err(err) = self.ensure_unique_id(&id) {
            return CommandResult::Error(err);
        }
        if id.is_empty() {
            return CommandResult::Error("ScreenCaptureSource requires a non-empty id".to_string());
        }
        if width == 0 || height == 0 || fps == 0 {
            return CommandResult::Error(format!(
                "ScreenCaptureSource {id} requires non-zero width/height/fps"
            ));
        }

        self.nodes.insert(
            id.clone(),
            NodeRecord::ScreenCapture(ScreenCaptureNode::new(id, width, height, fps)),
        );
        CommandResult::Success
    }

    fn create_destination(
        &mut self,
        id: String,
        family: crate::migration::protocol::DestinationFamily,
        audio: bool,
        video: bool,
    ) -> CommandResult {
        if let Err(err) = self.ensure_unique_id(&id) {
            return CommandResult::Error(err);
        }
        if !audio && !video {
            return CommandResult::Error(format!(
                "Destination with id {id} must have either audio or video enabled"
            ));
        }

        self.nodes.insert(
            id.clone(),
            NodeRecord::Destination(DestinationNode::new(id, family, audio, video)),
        );
        CommandResult::Success
    }

    fn create_mixer(
        &mut self,
        id: String,
        config: Option<HashMap<String, Value>>,
        audio: bool,
        video: bool,
    ) -> CommandResult {
        if let Err(err) = self.ensure_unique_id(&id) {
            return CommandResult::Error(err);
        }
        if !audio && !video {
            return CommandResult::Error(format!(
                "Mixer with id {id} must have either audio or video enabled"
            ));
        }

        let node = match MixerNode::new(id.clone(), config, audio, video) {
            Ok(node) => node,
            Err(err) => return CommandResult::Error(err),
        };
        self.nodes.insert(id, NodeRecord::Mixer(node));
        CommandResult::Success
    }

    fn connect(
        &mut self,
        link_id: String,
        src_id: String,
        sink_id: String,
        audio: bool,
        video: bool,
        config: Option<HashMap<String, Value>>,
    ) -> CommandResult {
        if !audio && !video {
            return CommandResult::Error(format!(
                "Link with id {link_id} must have either audio or video enabled"
            ));
        }
        if self.links.contains_key(&link_id) {
            return CommandResult::Error(format!("A link already exists with id {link_id}"));
        }

        let Some(src_node) = self.nodes.get(&src_id) else {
            return CommandResult::Error(format!("No producer with id {src_id}"));
        };
        let Some(sink_node) = self.nodes.get(&sink_id) else {
            return CommandResult::Error(format!("No consumer with id {sink_id}"));
        };

        if audio && (!src_node.can_output_audio() || !sink_node.can_input_audio()) {
            return CommandResult::Error(format!(
                "Link {link_id} requested audio, but source/sink capabilities do not match"
            ));
        }
        if video && (!src_node.can_output_video() || !sink_node.can_input_video()) {
            return CommandResult::Error(format!(
                "Link {link_id} requested video, but source/sink capabilities do not match"
            ));
        }

        let sink_update = match self.nodes.get_mut(&sink_id) {
            Some(NodeRecord::Destination(dest)) => dest.connect_input(&link_id, audio, video),
            Some(NodeRecord::Mixer(mixer)) => {
                mixer.connect_input_slot(&link_id, audio, video, config.clone())
            }
            Some(
                NodeRecord::Source(_)
                | NodeRecord::ScreenCapture(_)
                | NodeRecord::VideoGenerator(_),
            ) => Err(format!("Node {sink_id} is not a consumer")),
            None => Err(format!("No consumer with id {sink_id}")),
        };

        if let Err(err) = sink_update {
            return CommandResult::Error(err);
        }

        if let Some(src) = self.nodes.get_mut(&src_id) {
            src.add_consumer_link(&link_id, audio, video);
        }

        self.links.insert(
            link_id,
            LinkRecord {
                src_id,
                sink_id,
                audio,
                video,
                config,
            },
        );

        CommandResult::Success
    }

    fn disconnect(&mut self, link_id: &str) -> CommandResult {
        let Some(link) = self.links.remove(link_id) else {
            return CommandResult::Error(format!("No link with id {link_id}"));
        };

        self.remove_media_bridge_link(&link.src_id, link_id, link.audio, link.video);

        if let Some(src) = self.nodes.get_mut(&link.src_id) {
            src.remove_consumer_link(link_id);
        }
        if let Some(sink) = self.nodes.get_mut(&link.sink_id) {
            match sink {
                NodeRecord::Destination(dest) => dest.disconnect_input(link_id),
                NodeRecord::Mixer(mixer) => mixer.disconnect_input_slot(link_id),
                NodeRecord::Source(_)
                | NodeRecord::ScreenCapture(_)
                | NodeRecord::VideoGenerator(_) => {}
            }
        }

        CommandResult::Success
    }

    fn schedule_node(
        &mut self,
        id: &str,
        cue_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> CommandResult {
        let Some(node) = self.nodes.get_mut(id) else {
            return CommandResult::Error(format!("No node with id {id}"));
        };

        if let Err(err) = node.set_schedule(cue_time, end_time) {
            node.mark_error(err.clone());
            return CommandResult::Error(err);
        }

        CommandResult::Success
    }

    fn remove_node(&mut self, id: &str) -> CommandResult {
        if !self.nodes.contains_key(id) {
            return CommandResult::Error(format!("No node with id {id}"));
        }

        if let Some(node) = self.nodes.get_mut(id) {
            node.stop();
        }

        let link_ids = self
            .links
            .iter()
            .filter_map(|(link_id, link)| {
                if link.src_id == id || link.sink_id == id {
                    Some(link_id.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        for link_id in link_ids {
            let _ = self.disconnect(&link_id);
        }

        self.nodes.remove(id);
        CommandResult::Success
    }

    fn get_info(&self, id: Option<&String>) -> CommandResult {
        let mut nodes = HashMap::new();
        match id {
            Some(id) => {
                let Some(node) = self.nodes.get(id) else {
                    return CommandResult::Error(format!("No node with id {id}"));
                };
                nodes.insert(id.clone(), node.to_info());
            }
            None => {
                for (id, node) in &self.nodes {
                    nodes.insert(id.clone(), node.to_info());
                }
            }
        }

        CommandResult::Info(Info { nodes })
    }

    fn add_control_point(
        &mut self,
        controllee_id: &str,
        property: &str,
        control_point: ControlPoint,
    ) -> CommandResult {
        if let Some(link) = self.links.get(controllee_id).cloned() {
            let Some(node) = self.nodes.get_mut(&link.sink_id) else {
                return CommandResult::Error(format!(
                    "No sink node with id {} for link {}",
                    link.sink_id, controllee_id
                ));
            };

            if let NodeRecord::Mixer(mixer) = node {
                return match mixer.add_slot_control_point(controllee_id, property, control_point) {
                    Ok(()) => CommandResult::Success,
                    Err(err) => CommandResult::Error(err),
                };
            }

            return CommandResult::Error(format!(
                "Slot control points are only supported for mixer links; {} is not a mixer",
                link.sink_id
            ));
        }

        let Some(node) = self.nodes.get_mut(controllee_id) else {
            return CommandResult::Error(format!("No node or slot with id {controllee_id}"));
        };

        if let NodeRecord::Mixer(mixer) = node {
            return match mixer.add_control_point(property, control_point) {
                Ok(()) => CommandResult::Success,
                Err(err) => CommandResult::Error(err),
            };
        }

        CommandResult::Error(format!(
            "Node control points are currently supported only for mixers; {controllee_id} is not a mixer"
        ))
    }

    fn remove_control_point(
        &mut self,
        controller_id: &str,
        controllee_id: &str,
        property: &str,
    ) -> CommandResult {
        if let Some(link) = self.links.get(controllee_id).cloned() {
            let Some(node) = self.nodes.get_mut(&link.sink_id) else {
                return CommandResult::Error(format!(
                    "No sink node with id {} for link {}",
                    link.sink_id, controllee_id
                ));
            };

            if let NodeRecord::Mixer(mixer) = node {
                mixer.remove_slot_control_point(controller_id, controllee_id, property);
                return CommandResult::Success;
            }

            return CommandResult::Error(format!(
                "Slot control points are only supported for mixer links; {} is not a mixer",
                link.sink_id
            ));
        }

        let Some(node) = self.nodes.get_mut(controllee_id) else {
            return CommandResult::Error(format!("No node or slot with id {controllee_id}"));
        };

        if let NodeRecord::Mixer(mixer) = node {
            mixer.remove_control_point(controller_id, property);
            return CommandResult::Success;
        }

        CommandResult::Error(format!(
            "Node control points are currently supported only for mixers; {controllee_id} is not a mixer"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration::protocol::{Command, ControlMode, DestinationFamily, State};
    use chrono::Duration;
    use serde_json::json;

    fn started_manager() -> NodeManager {
        let mut manager = NodeManager::default();
        manager.start();
        manager
    }

    fn expect_error(result: CommandResult, needle: &str) {
        match result {
            CommandResult::Error(err) => assert!(
                err.contains(needle),
                "expected error containing `{needle}`, got `{err}`"
            ),
            other => panic!("expected error containing `{needle}`, got {other:?}"),
        }
    }

    #[test]
    fn create_connect_and_get_info() {
        let mut manager = started_manager();

        assert!(matches!(
            manager.dispatch(Command::CreateSource {
                id: "source-1".to_string(),
                uri: "https://example.com/video.mp4".to_string(),
                audio: true,
                video: true,
            }),
            CommandResult::Success
        ));
        assert!(matches!(
            manager.dispatch(Command::CreateDestination {
                id: "dest-1".to_string(),
                family: DestinationFamily::LocalPlayback,
                audio: true,
                video: true,
            }),
            CommandResult::Success
        ));
        assert!(matches!(
            manager.dispatch(Command::Connect {
                link_id: "link-1".to_string(),
                src_id: "source-1".to_string(),
                sink_id: "dest-1".to_string(),
                audio: true,
                video: true,
                config: None,
            }),
            CommandResult::Success
        ));

        let result = manager.dispatch(Command::GetInfo { id: None });
        match result {
            CommandResult::Info(info) => {
                assert!(info.nodes.contains_key("source-1"));
                assert!(info.nodes.contains_key("dest-1"));
            }
            other => panic!("Expected info result, got {other:?}"),
        }
    }

    #[test]
    fn create_video_generator_maps_to_compatible_source_info() {
        let mut manager = started_manager();
        assert!(matches!(
            manager.dispatch(Command::CreateVideoGenerator {
                id: "gen-1".to_string()
            }),
            CommandResult::Success
        ));

        let result = manager.dispatch(Command::GetInfo {
            id: Some("gen-1".to_string()),
        });
        match result {
            CommandResult::Info(info) => match info.nodes.get("gen-1") {
                Some(NodeInfo::Source(source)) => {
                    assert_eq!(source.uri, "videogenerator://gen-1");
                }
                other => panic!("expected compatible source info, got {other:?}"),
            },
            other => panic!("expected info result, got {other:?}"),
        }
    }

    #[test]
    fn create_commands_validate_flags_ids_and_config() {
        let mut manager = started_manager();

        expect_error(
            manager.dispatch(Command::CreateSource {
                id: "source-disabled".to_string(),
                uri: "https://example.com/video.mp4".to_string(),
                audio: false,
                video: false,
            }),
            "must have either audio or video enabled",
        );
        expect_error(
            manager.dispatch(Command::CreateDestination {
                id: "dest-disabled".to_string(),
                family: DestinationFamily::LocalPlayback,
                audio: false,
                video: false,
            }),
            "must have either audio or video enabled",
        );
        expect_error(
            manager.dispatch(Command::CreateMixer {
                id: "mixer-disabled".to_string(),
                config: None,
                audio: false,
                video: false,
            }),
            "must have either audio or video enabled",
        );

        assert!(matches!(
            manager.dispatch(Command::CreateSource {
                id: "dup-id".to_string(),
                uri: "https://example.com/video.mp4".to_string(),
                audio: true,
                video: true,
            }),
            CommandResult::Success
        ));
        expect_error(
            manager.dispatch(Command::CreateDestination {
                id: "dup-id".to_string(),
                family: DestinationFamily::LocalPlayback,
                audio: true,
                video: true,
            }),
            "already exists with id dup-id",
        );

        expect_error(
            manager.dispatch(Command::CreateMixer {
                id: "mixer-invalid-config".to_string(),
                config: Some(HashMap::from([("bad".to_string(), json!(1))])),
                audio: true,
                video: true,
            }),
            "No setting with name bad",
        );
    }

    #[test]
    fn create_screen_capture_source_succeeds() {
        let mut manager = started_manager();

        assert!(matches!(
            manager.dispatch(Command::CreateScreenCaptureSource {
                id: "cap-1".to_string(),
                width: 1280,
                height: 720,
                fps: 30,
            }),
            CommandResult::Success
        ));

        match manager.nodes.get("cap-1") {
            Some(NodeRecord::ScreenCapture(node)) => {
                assert_eq!(node.id, "cap-1");
                assert_eq!(node.width, 1280);
                assert_eq!(node.height, 720);
                assert_eq!(node.fps, 30);
                assert_eq!(node.state, State::Started);
            }
            other => panic!("expected NodeRecord::ScreenCapture, got {other:?}"),
        }
    }

    #[test]
    fn screen_capture_source_validates_dimensions() {
        let mut manager = started_manager();

        for (width, height, fps, label) in [
            (0u32, 720u32, 30u32, "zero width"),
            (1280u32, 0u32, 30u32, "zero height"),
            (1280u32, 720u32, 0u32, "zero fps"),
        ] {
            expect_error(
                manager.dispatch(Command::CreateScreenCaptureSource {
                    id: format!("cap-{label}"),
                    width,
                    height,
                    fps,
                }),
                "requires non-zero width/height/fps",
            );
        }
    }

    #[test]
    fn screen_capture_source_rejects_duplicate_id() {
        let mut manager = started_manager();
        let make_command = || Command::CreateScreenCaptureSource {
            id: "cap-1".to_string(),
            width: 1280,
            height: 720,
            fps: 30,
        };

        assert!(matches!(
            manager.dispatch(make_command()),
            CommandResult::Success
        ));
        expect_error(
            manager.dispatch(make_command()),
            "already exists with id cap-1",
        );
    }

    #[test]
    fn screen_capture_source_appears_in_get_info() {
        let mut manager = started_manager();
        assert!(matches!(
            manager.dispatch(Command::CreateScreenCaptureSource {
                id: "cap-1".to_string(),
                width: 1280,
                height: 720,
                fps: 30,
            }),
            CommandResult::Success
        ));

        let result = manager.dispatch(Command::GetInfo { id: None });
        match result {
            CommandResult::Info(info) => match info.nodes.get("cap-1") {
                Some(NodeInfo::Source(source)) => {
                    assert_eq!(source.uri, "screen://1280x720@30fps");
                    assert_eq!(source.state, State::Started);
                    assert_eq!(source.audio_consumer_slot_ids, None);
                }
                other => panic!("expected NodeInfo::Source, got {other:?}"),
            },
            other => panic!("expected CommandResult::Info, got {other:?}"),
        }
    }

    #[test]
    fn screen_capture_source_uses_serde_defaults_via_dispatch() {
        let cmd: Command =
            serde_json::from_str(r#"{"createscreencapturesource":{"id":"cap-default"}}"#).unwrap();

        let mut manager = started_manager();
        assert!(matches!(manager.dispatch(cmd), CommandResult::Success));

        match manager.nodes.get("cap-default") {
            Some(NodeRecord::ScreenCapture(node)) => {
                assert_eq!(node.width, 1280);
                assert_eq!(node.height, 720);
                assert_eq!(node.fps, 30);
            }
            other => panic!("expected NodeRecord::ScreenCapture, got {other:?}"),
        }
    }

    #[test]
    fn connect_disconnect_validate_capabilities_and_link_ids() {
        let mut manager = started_manager();

        assert!(matches!(
            manager.dispatch(Command::CreateSource {
                id: "source-audio".to_string(),
                uri: "https://example.com/audio.mp3".to_string(),
                audio: true,
                video: false,
            }),
            CommandResult::Success
        ));
        assert!(matches!(
            manager.dispatch(Command::CreateDestination {
                id: "dest-video".to_string(),
                family: DestinationFamily::LocalPlayback,
                audio: false,
                video: true,
            }),
            CommandResult::Success
        ));
        assert!(matches!(
            manager.dispatch(Command::CreateDestination {
                id: "dest-audio".to_string(),
                family: DestinationFamily::LocalPlayback,
                audio: true,
                video: false,
            }),
            CommandResult::Success
        ));

        expect_error(
            manager.dispatch(Command::Connect {
                link_id: "bad-link-media".to_string(),
                src_id: "source-audio".to_string(),
                sink_id: "dest-video".to_string(),
                audio: true,
                video: false,
                config: None,
            }),
            "capabilities do not match",
        );
        expect_error(
            manager.dispatch(Command::Connect {
                link_id: "missing-src".to_string(),
                src_id: "missing".to_string(),
                sink_id: "dest-audio".to_string(),
                audio: true,
                video: false,
                config: None,
            }),
            "No producer with id missing",
        );
        expect_error(
            manager.dispatch(Command::Connect {
                link_id: "missing-sink".to_string(),
                src_id: "source-audio".to_string(),
                sink_id: "missing".to_string(),
                audio: true,
                video: false,
                config: None,
            }),
            "No consumer with id missing",
        );

        let link_config = Some(HashMap::from([("audio::volume".to_string(), json!(0.5))]));
        assert!(matches!(
            manager.dispatch(Command::Connect {
                link_id: "link-1".to_string(),
                src_id: "source-audio".to_string(),
                sink_id: "dest-audio".to_string(),
                audio: true,
                video: false,
                config: link_config.clone(),
            }),
            CommandResult::Success
        ));
        assert_eq!(
            manager
                .links
                .get("link-1")
                .and_then(|link| link.config.clone()),
            link_config
        );

        expect_error(
            manager.dispatch(Command::Connect {
                link_id: "link-1".to_string(),
                src_id: "source-audio".to_string(),
                sink_id: "dest-audio".to_string(),
                audio: true,
                video: false,
                config: None,
            }),
            "already exists with id link-1",
        );

        assert!(matches!(
            manager.dispatch(Command::Disconnect {
                link_id: "link-1".to_string()
            }),
            CommandResult::Success
        ));
        expect_error(
            manager.dispatch(Command::Disconnect {
                link_id: "link-1".to_string(),
            }),
            "No link with id link-1",
        );
    }

    #[test]
    fn start_reschedule_and_remove_commands_work_for_nodes() {
        let mut manager = started_manager();
        assert!(matches!(
            manager.dispatch(Command::CreateSource {
                id: "source-1".to_string(),
                uri: "https://example.com/video.mp4".to_string(),
                audio: true,
                video: true,
            }),
            CommandResult::Success
        ));

        assert!(matches!(
            manager.dispatch(Command::Start {
                id: "source-1".to_string(),
                cue_time: None,
                end_time: None,
            }),
            CommandResult::Success
        ));

        let cue = Utc::now() + Duration::seconds(20);
        let end = cue + Duration::seconds(5);
        assert!(matches!(
            manager.dispatch(Command::Reschedule {
                id: "source-1".to_string(),
                cue_time: Some(cue),
                end_time: Some(end),
            }),
            CommandResult::Success
        ));

        assert!(matches!(
            manager.dispatch(Command::Remove {
                id: "source-1".to_string()
            }),
            CommandResult::Success
        ));
        expect_error(
            manager.dispatch(Command::Remove {
                id: "source-1".to_string(),
            }),
            "No node with id source-1",
        );
    }

    #[test]
    fn get_info_reports_missing_node_for_specific_id() {
        let mut manager = started_manager();
        expect_error(
            manager.dispatch(Command::GetInfo {
                id: Some("missing".to_string()),
            }),
            "No node with id missing",
        );
    }

    #[test]
    fn add_and_remove_control_point_supports_mixer_and_mixer_slots() {
        let mut manager = started_manager();

        assert!(matches!(
            manager.dispatch(Command::CreateSource {
                id: "source-1".to_string(),
                uri: "https://example.com/audio.mp3".to_string(),
                audio: true,
                video: false,
            }),
            CommandResult::Success
        ));
        assert!(matches!(
            manager.dispatch(Command::CreateMixer {
                id: "mixer-1".to_string(),
                config: None,
                audio: true,
                video: false,
            }),
            CommandResult::Success
        ));
        assert!(matches!(
            manager.dispatch(Command::Connect {
                link_id: "slot-1".to_string(),
                src_id: "source-1".to_string(),
                sink_id: "mixer-1".to_string(),
                audio: true,
                video: false,
                config: None,
            }),
            CommandResult::Success
        ));

        let mixer_cp = ControlPoint {
            id: "cp-width".to_string(),
            time: Utc::now(),
            value: json!(1280),
            mode: ControlMode::Set,
        };
        assert!(matches!(
            manager.dispatch(Command::AddControlPoint {
                controllee_id: "mixer-1".to_string(),
                property: "width".to_string(),
                control_point: mixer_cp,
            }),
            CommandResult::Success
        ));
        assert!(matches!(
            manager.dispatch(Command::RemoveControlPoint {
                id: "cp-width".to_string(),
                controllee_id: "mixer-1".to_string(),
                property: "width".to_string(),
            }),
            CommandResult::Success
        ));

        let slot_cp = ControlPoint {
            id: "cp-slot".to_string(),
            time: Utc::now(),
            value: json!(0.5),
            mode: ControlMode::Set,
        };
        assert!(matches!(
            manager.dispatch(Command::AddControlPoint {
                controllee_id: "slot-1".to_string(),
                property: "audio::volume".to_string(),
                control_point: slot_cp,
            }),
            CommandResult::Success
        ));
        assert!(matches!(
            manager.dispatch(Command::RemoveControlPoint {
                id: "cp-slot".to_string(),
                controllee_id: "slot-1".to_string(),
                property: "audio::volume".to_string(),
            }),
            CommandResult::Success
        ));
    }

    #[test]
    fn control_point_commands_return_errors_for_invalid_targets() {
        let mut manager = started_manager();

        assert!(matches!(
            manager.dispatch(Command::CreateSource {
                id: "source-1".to_string(),
                uri: "https://example.com/audio.mp3".to_string(),
                audio: true,
                video: false,
            }),
            CommandResult::Success
        ));
        assert!(matches!(
            manager.dispatch(Command::CreateDestination {
                id: "dest-1".to_string(),
                family: DestinationFamily::LocalPlayback,
                audio: true,
                video: false,
            }),
            CommandResult::Success
        ));
        assert!(matches!(
            manager.dispatch(Command::Connect {
                link_id: "dest-link".to_string(),
                src_id: "source-1".to_string(),
                sink_id: "dest-1".to_string(),
                audio: true,
                video: false,
                config: None,
            }),
            CommandResult::Success
        ));

        let cp = ControlPoint {
            id: "cp-1".to_string(),
            time: Utc::now(),
            value: json!(0.5),
            mode: ControlMode::Set,
        };
        expect_error(
            manager.dispatch(Command::AddControlPoint {
                controllee_id: "source-1".to_string(),
                property: "width".to_string(),
                control_point: cp.clone(),
            }),
            "supported only for mixers",
        );
        expect_error(
            manager.dispatch(Command::AddControlPoint {
                controllee_id: "dest-link".to_string(),
                property: "audio::volume".to_string(),
                control_point: cp,
            }),
            "only supported for mixer links",
        );
        expect_error(
            manager.dispatch(Command::RemoveControlPoint {
                id: "cp-1".to_string(),
                controllee_id: "source-1".to_string(),
                property: "width".to_string(),
            }),
            "supported only for mixers",
        );
        expect_error(
            manager.dispatch(Command::RemoveControlPoint {
                id: "cp-1".to_string(),
                controllee_id: "missing".to_string(),
                property: "width".to_string(),
            }),
            "No node or slot with id missing",
        );
    }

    #[test]
    fn shutdown_clears_runtime_state() {
        let mut manager = started_manager();
        assert!(matches!(
            manager.dispatch(Command::CreateSource {
                id: "source-1".to_string(),
                uri: "https://example.com/video.mp4".to_string(),
                audio: true,
                video: true,
            }),
            CommandResult::Success
        ));
        assert!(matches!(
            manager.dispatch(Command::CreateDestination {
                id: "dest-1".to_string(),
                family: DestinationFamily::LocalPlayback,
                audio: true,
                video: true,
            }),
            CommandResult::Success
        ));
        assert!(matches!(
            manager.dispatch(Command::Connect {
                link_id: "link-1".to_string(),
                src_id: "source-1".to_string(),
                sink_id: "dest-1".to_string(),
                audio: true,
                video: true,
                config: None,
            }),
            CommandResult::Success
        ));

        manager.shutdown();
        assert!(!manager.started);
        assert!(manager.nodes.is_empty());
        assert!(manager.links.is_empty());
        assert!(manager.media_bridges.is_empty());
    }

    #[test]
    fn create_whep_destination_succeeds() {
        let mut manager = started_manager();
        let result = manager.dispatch(Command::CreateDestination {
            id: "tv-1".into(),
            family: DestinationFamily::Whep { server_port: 0 },
            audio: false,
            video: true,
        });
        assert!(matches!(result, CommandResult::Success), "{result:?}");
        assert!(manager.nodes.contains_key("tv-1"));
    }

    #[test]
    fn create_whep_destination_with_explicit_port_succeeds() {
        let mut manager = started_manager();
        let result = manager.dispatch(Command::CreateDestination {
            id: "tv-2".into(),
            family: DestinationFamily::Whep { server_port: 54321 },
            audio: false,
            video: true,
        });
        assert!(matches!(result, CommandResult::Success), "{result:?}");
        assert!(manager.nodes.contains_key("tv-2"));
    }

    #[test]
    fn whep_destination_info_carries_optional_bound_ports() {
        let mut manager = started_manager();
        assert!(matches!(
            manager.dispatch(Command::CreateDestination {
                id: "tv-1".into(),
                family: DestinationFamily::Whep { server_port: 0 },
                audio: false,
                video: true,
            }),
            CommandResult::Success
        ));

        let info = manager.dispatch(Command::GetInfo {
            id: Some("tv-1".into()),
        });
        match info {
            CommandResult::Info(snapshot) => {
                let dest = snapshot.nodes.get("tv-1").unwrap();
                match dest {
                    NodeInfo::Destination(d) => {
                        assert!(matches!(&d.family, DestinationFamily::Whep { .. }));
                        assert!(d.bound_port_v4.is_none());
                        assert!(d.bound_port_v6.is_none());
                    }
                    other => panic!("expected destination info, got {other:?}"),
                }
            }
            other => panic!("expected Info, got {other:?}"),
        }
    }

    #[test]
    fn local_playback_destination_info_omits_bound_ports() {
        let mut manager = started_manager();
        assert!(matches!(
            manager.dispatch(Command::CreateDestination {
                id: "out".into(),
                family: DestinationFamily::LocalPlayback,
                audio: true,
                video: true,
            }),
            CommandResult::Success
        ));

        let info = manager.dispatch(Command::GetInfo {
            id: Some("out".into()),
        });
        match info {
            CommandResult::Info(snapshot) => {
                let dest = snapshot.nodes.get("out").unwrap();
                let json = serde_json::to_string(dest).unwrap();
                assert!(!json.contains("bound_port"));
            }
            other => panic!("expected Info, got {other:?}"),
        }
    }
}
