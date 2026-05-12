use crate::migration::protocol::{Command, CommandResult, ControlPoint, NodeInfo, State};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct StartMessage {
    pub cue_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy)]
pub struct StopMessage;

#[derive(Debug, Clone)]
pub struct ScheduleMessage {
    pub cue_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct AddControlPointMessage {
    pub property: String,
    pub control_point: ControlPoint,
}

#[derive(Debug, Clone)]
pub struct RemoveControlPointMessage {
    pub controller_id: String,
    pub property: String,
}

#[derive(Debug, Clone, Copy)]
pub struct GetNodeInfoMessage;

#[derive(Debug, Clone)]
pub struct StoppedMessage {
    pub id: String,
    pub has_video_producer: bool,
    pub has_audio_producer: bool,
}

#[derive(Debug, Clone)]
pub enum NodeStatusMessage {
    State { id: String, state: State },
    Error { id: String, message: String },
}

#[derive(Debug, Clone)]
pub enum ConsumerMessage {
    Connect {
        link_id: String,
        has_video: bool,
        has_audio: bool,
        config: Option<std::collections::HashMap<String, serde_json::Value>>,
    },
    Disconnect {
        slot_id: String,
    },
    AddControlPoint {
        slot_id: String,
        property: String,
        control_point: ControlPoint,
    },
    RemoveControlPoint {
        controller_id: String,
        slot_id: String,
        property: String,
    },
}

#[derive(Debug, Clone)]
pub struct CommandMessage {
    pub command: Command,
}

#[derive(Debug, Clone)]
pub struct RegisterListenerMessage {
    pub id: String,
}

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum MessageResult {
    Command(CommandResult),
    NodeInfo(NodeInfo),
    Empty,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration::protocol::{Command, ControlMode, MixerInfo, MixerSlotInfo};
    use serde_json::json;
    use std::collections::HashMap;

    fn sample_control_point() -> ControlPoint {
        ControlPoint {
            id: "cp-1".to_string(),
            time: Utc::now(),
            value: json!(0.75),
            mode: ControlMode::Set,
        }
    }

    #[test]
    fn start_and_schedule_messages_store_times() {
        let cue = Some(Utc::now());
        let end = cue.map(|t| t + chrono::Duration::seconds(10));

        let start = StartMessage {
            cue_time: cue,
            end_time: end,
        };
        let schedule = ScheduleMessage {
            cue_time: cue,
            end_time: end,
        };

        assert_eq!(start.cue_time, schedule.cue_time);
        assert_eq!(start.end_time, schedule.end_time);
    }

    #[test]
    fn node_status_message_variants_preserve_payload() {
        let state_message = NodeStatusMessage::State {
            id: "node-1".to_string(),
            state: State::Started,
        };
        match state_message {
            NodeStatusMessage::State { id, state } => {
                assert_eq!(id, "node-1");
                assert_eq!(state, State::Started);
            }
            other => panic!("unexpected variant: {other:?}"),
        }

        let error_message = NodeStatusMessage::Error {
            id: "node-2".to_string(),
            message: "boom".to_string(),
        };
        match error_message {
            NodeStatusMessage::Error { id, message } => {
                assert_eq!(id, "node-2");
                assert_eq!(message, "boom");
            }
            other => panic!("unexpected variant: {other:?}"),
        }
    }

    #[test]
    fn consumer_message_variants_preserve_payload() {
        let mut config = HashMap::new();
        config.insert("video::alpha".to_string(), json!(0.5));

        let connect = ConsumerMessage::Connect {
            link_id: "slot-1".to_string(),
            has_video: true,
            has_audio: false,
            config: Some(config.clone()),
        };
        match connect {
            ConsumerMessage::Connect {
                link_id,
                has_video,
                has_audio,
                config: Some(cfg),
            } => {
                assert_eq!(link_id, "slot-1");
                assert!(has_video);
                assert!(!has_audio);
                assert_eq!(cfg, config);
            }
            other => panic!("unexpected variant: {other:?}"),
        }

        let add = ConsumerMessage::AddControlPoint {
            slot_id: "slot-1".to_string(),
            property: "audio::volume".to_string(),
            control_point: sample_control_point(),
        };
        match add {
            ConsumerMessage::AddControlPoint {
                slot_id,
                property,
                control_point,
            } => {
                assert_eq!(slot_id, "slot-1");
                assert_eq!(property, "audio::volume");
                assert_eq!(control_point.id, "cp-1");
            }
            other => panic!("unexpected variant: {other:?}"),
        }

        let remove = ConsumerMessage::RemoveControlPoint {
            controller_id: "cp-1".to_string(),
            slot_id: "slot-1".to_string(),
            property: "audio::volume".to_string(),
        };
        match remove {
            ConsumerMessage::RemoveControlPoint {
                controller_id,
                slot_id,
                property,
            } => {
                assert_eq!(controller_id, "cp-1");
                assert_eq!(slot_id, "slot-1");
                assert_eq!(property, "audio::volume");
            }
            other => panic!("unexpected variant: {other:?}"),
        }
    }

    #[test]
    fn message_result_variants_store_command_and_node_info() {
        let command_message = MessageResult::Command(CommandResult::Success);
        assert!(matches!(
            command_message,
            MessageResult::Command(CommandResult::Success)
        ));

        let node_info = MessageResult::NodeInfo(NodeInfo::Mixer(MixerInfo {
            slots: HashMap::from([("slot-1".to_string(), MixerSlotInfo { volume: 1.0 })]),
            video_consumer_slot_ids: Some(vec!["link-video".to_string()]),
            audio_consumer_slot_ids: Some(vec!["link-audio".to_string()]),
            cue_time: None,
            end_time: None,
            state: State::Initial,
            settings: HashMap::new(),
            control_points: HashMap::new(),
            slot_settings: HashMap::new(),
            slot_control_points: HashMap::new(),
        }));

        match node_info {
            MessageResult::NodeInfo(NodeInfo::Mixer(info)) => {
                assert!(info.slots.contains_key("slot-1"));
            }
            other => panic!("unexpected variant: {other:?}"),
        }

        let command_payload = CommandMessage {
            command: Command::GetInfo { id: None },
        };
        assert!(matches!(
            command_payload.command,
            Command::GetInfo { id: None }
        ));
    }
}
