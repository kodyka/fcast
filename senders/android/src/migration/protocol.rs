use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::HashMap};

fn default_as_true() -> bool {
    true
}

fn default_capture_width() -> u32 {
    1280
}

fn default_capture_height() -> u32 {
    720
}

fn default_capture_fps() -> u32 {
    30
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ControlMode {
    Set,
    Interpolate,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub struct ControlPoint {
    pub id: String,
    pub time: DateTime<Utc>,
    pub value: serde_json::Value,
    pub mode: ControlMode,
}

impl Ord for ControlPoint {
    fn cmp(&self, other: &Self) -> Ordering {
        self.time.cmp(&other.time)
    }
}

impl PartialOrd for ControlPoint {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Command {
    CreateVideoGenerator {
        id: String,
    },
    CreateSource {
        id: String,
        uri: String,
        #[serde(default = "default_as_true")]
        audio: bool,
        #[serde(default = "default_as_true")]
        video: bool,
    },
    CreateScreenCaptureSource {
        id: String,
        #[serde(default = "default_capture_width")]
        width: u32,
        #[serde(default = "default_capture_height")]
        height: u32,
        #[serde(default = "default_capture_fps")]
        fps: u32,
    },
    CreateDestination {
        id: String,
        family: DestinationFamily,
        #[serde(default = "default_as_true")]
        audio: bool,
        #[serde(default = "default_as_true")]
        video: bool,
    },
    CreateMixer {
        id: String,
        config: Option<HashMap<String, serde_json::Value>>,
        #[serde(default = "default_as_true")]
        audio: bool,
        #[serde(default = "default_as_true")]
        video: bool,
    },
    Connect {
        link_id: String,
        src_id: String,
        sink_id: String,
        #[serde(default = "default_as_true")]
        audio: bool,
        #[serde(default = "default_as_true")]
        video: bool,
        config: Option<HashMap<String, serde_json::Value>>,
    },
    Start {
        id: String,
        cue_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    },
    Reschedule {
        id: String,
        cue_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    },
    Remove {
        id: String,
    },
    Disconnect {
        link_id: String,
    },
    GetInfo {
        id: Option<String>,
    },
    AddControlPoint {
        controllee_id: String,
        property: String,
        control_point: ControlPoint,
    },
    RemoveControlPoint {
        id: String,
        controllee_id: String,
        property: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct ControllerMessage {
    pub id: uuid::Uuid,
    pub command: Command,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Initial,
    Starting,
    Started,
    Stopping,
    Stopped,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DestinationFamily {
    Rtmp {
        uri: String,
    },
    Udp {
        host: String,
    },
    LocalFile {
        base_name: String,
        max_size_time: Option<u32>,
    },
    LocalPlayback,
    Whep {
        #[serde(default)]
        server_port: u16,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct SourceInfo {
    pub uri: String,
    pub video_consumer_slot_ids: Option<Vec<String>>,
    pub audio_consumer_slot_ids: Option<Vec<String>>,
    pub cue_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub state: State,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct DestinationInfo {
    pub family: DestinationFamily,
    pub audio_slot_id: Option<String>,
    pub video_slot_id: Option<String>,
    pub cue_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub state: State,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bound_port_v4: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bound_port_v6: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct MixerSlotInfo {
    pub volume: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct MixerInfo {
    pub slots: HashMap<String, MixerSlotInfo>,
    pub video_consumer_slot_ids: Option<Vec<String>>,
    pub audio_consumer_slot_ids: Option<Vec<String>>,
    pub cue_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub state: State,
    pub settings: HashMap<String, serde_json::Value>,
    pub control_points: HashMap<String, Vec<ControlPoint>>,
    pub slot_settings: HashMap<String, HashMap<String, serde_json::Value>>,
    pub slot_control_points: HashMap<String, HashMap<String, Vec<ControlPoint>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeInfo {
    Source(SourceInfo),
    Destination(DestinationInfo),
    Mixer(MixerInfo),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct Info {
    pub nodes: HashMap<String, NodeInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommandResult {
    Error(String),
    Success,
    Info(Info),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct ServerMessage {
    pub id: Option<uuid::Uuid>,
    pub result: CommandResult,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use serde_json::json;
    use uuid::Uuid;

    fn point_at(offset_secs: i64, value: serde_json::Value) -> ControlPoint {
        ControlPoint {
            id: format!("cp-{offset_secs}"),
            time: Utc::now() + Duration::seconds(offset_secs),
            value,
            mode: ControlMode::Set,
        }
    }

    #[test]
    fn command_defaults_audio_and_video_to_true_when_omitted() {
        let source: Command =
            serde_json::from_str(r#"{"createsource":{"id":"source-1","uri":"file:///a.mp4"}}"#)
                .unwrap();
        assert!(matches!(
            source,
            Command::CreateSource {
                audio: true,
                video: true,
                ..
            }
        ));

        let destination: Command = serde_json::from_str(
            r#"{"createdestination":{"id":"dest-1","family":"LocalPlayback"}}"#,
        )
        .unwrap();
        assert!(matches!(
            destination,
            Command::CreateDestination {
                audio: true,
                video: true,
                ..
            }
        ));

        let mixer: Command = serde_json::from_str(r#"{"createmixer":{"id":"mixer-1"}}"#).unwrap();
        assert!(matches!(
            mixer,
            Command::CreateMixer {
                audio: true,
                video: true,
                ..
            }
        ));

        let connect: Command = serde_json::from_str(
            r#"{"connect":{"link_id":"link-1","src_id":"source-1","sink_id":"dest-1"}}"#,
        )
        .unwrap();
        assert!(matches!(
            connect,
            Command::Connect {
                audio: true,
                video: true,
                ..
            }
        ));
    }

    #[test]
    fn whep_destination_serdes_roundtrip() {
        let original = DestinationFamily::Whep { server_port: 0 };
        let json = serde_json::to_string(&original).unwrap();
        let parsed: DestinationFamily = serde_json::from_str(&json).unwrap();
        assert_eq!(original, parsed);
        assert!(json.starts_with(r#"{"Whep":{"server_port":0"#));
    }

    #[test]
    fn whep_destination_default_server_port_when_omitted() {
        let minimal: DestinationFamily = serde_json::from_str(r#"{"Whep":{}}"#).unwrap();
        match minimal {
            DestinationFamily::Whep { server_port } => assert_eq!(server_port, 0),
            other => panic!("expected Whep variant, got {other:?}"),
        }
    }

    #[test]
    fn whep_destination_info_bound_ports_skipped_when_none() {
        let info = DestinationInfo {
            family: DestinationFamily::Whep { server_port: 0 },
            audio_slot_id: None,
            video_slot_id: None,
            cue_time: None,
            end_time: None,
            state: State::Initial,
            bound_port_v4: None,
            bound_port_v6: None,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(!json.contains("bound_port"));
    }

    #[test]
    fn whep_destination_info_bound_ports_emit_when_some() {
        let info = DestinationInfo {
            family: DestinationFamily::Whep { server_port: 0 },
            audio_slot_id: None,
            video_slot_id: None,
            cue_time: None,
            end_time: None,
            state: State::Initial,
            bound_port_v4: Some(54321),
            bound_port_v6: Some(54322),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains(r#""bound_port_v4":54321"#));
        assert!(json.contains(r#""bound_port_v6":54322"#));
    }

    #[test]
    fn destination_family_roundtrip_includes_all_variants() {
        let families = [
            DestinationFamily::Rtmp {
                uri: "rtmp://localhost/live".to_string(),
            },
            DestinationFamily::Udp {
                host: "127.0.0.1".to_string(),
            },
            DestinationFamily::LocalFile {
                base_name: "capture".to_string(),
                max_size_time: Some(5_000),
            },
            DestinationFamily::LocalPlayback,
        ];

        for family in families {
            let encoded = serde_json::to_string(&family).unwrap();
            let decoded: DestinationFamily = serde_json::from_str(&encoded).unwrap();
            assert_eq!(decoded, family);
        }
    }

    #[test]
    fn screen_capture_command_deserialises() {
        let cmd: Command = serde_json::from_str(
            r#"{"createscreencapturesource":{"id":"cap-1","width":1280,"height":720,"fps":30}}"#,
        )
        .unwrap();

        match cmd {
            Command::CreateScreenCaptureSource {
                id,
                width,
                height,
                fps,
            } => {
                assert_eq!(id, "cap-1");
                assert_eq!(width, 1280);
                assert_eq!(height, 720);
                assert_eq!(fps, 30);
            }
            other => panic!("expected CreateScreenCaptureSource, got {other:?}"),
        }
    }

    #[test]
    fn screen_capture_command_uses_defaults_when_omitted() {
        let cmd: Command =
            serde_json::from_str(r#"{"createscreencapturesource":{"id":"cap-1"}}"#).unwrap();

        match cmd {
            Command::CreateScreenCaptureSource {
                id,
                width,
                height,
                fps,
            } => {
                assert_eq!(id, "cap-1");
                assert_eq!(width, 1280);
                assert_eq!(height, 720);
                assert_eq!(fps, 30);
            }
            other => panic!("expected CreateScreenCaptureSource, got {other:?}"),
        }
    }

    #[test]
    fn screen_capture_command_roundtrips() {
        let cmd = Command::CreateScreenCaptureSource {
            id: "cap-1".into(),
            width: 1920,
            height: 1080,
            fps: 60,
        };

        let json = serde_json::to_string(&cmd).unwrap();
        let parsed: Command = serde_json::from_str(&json).unwrap();
        assert!(matches!(
            parsed,
            Command::CreateScreenCaptureSource {
                ref id,
                width: 1920,
                height: 1080,
                fps: 60,
            } if id == "cap-1"
        ));
    }

    #[test]
    fn control_points_sort_by_timestamp() {
        let mut points = vec![
            point_at(20, json!(1.0)),
            point_at(-10, json!(2.0)),
            point_at(5, json!(3.0)),
        ];

        points.sort();
        assert_eq!(points[0].id, "cp--10");
        assert_eq!(points[1].id, "cp-5");
        assert_eq!(points[2].id, "cp-20");
    }

    #[test]
    fn controller_and_server_messages_preserve_wire_shape() {
        let request_id = Uuid::new_v4();
        let controller = ControllerMessage {
            id: request_id,
            command: Command::GetInfo { id: None },
        };

        let controller_json = serde_json::to_value(&controller).unwrap();
        let expected_id = request_id.as_hyphenated().to_string();
        assert_eq!(
            controller_json
                .get("id")
                .and_then(serde_json::Value::as_str),
            Some(expected_id.as_str())
        );
        assert!(controller_json
            .get("command")
            .and_then(|v| v.get("getinfo"))
            .is_some());

        let response = ServerMessage {
            id: Some(request_id),
            result: CommandResult::Success,
        };
        let response_json = serde_json::to_value(&response).unwrap();
        assert!(response_json.get("result").is_some());

        let decoded: ServerMessage = serde_json::from_value(response_json).unwrap();
        assert_eq!(decoded.id, Some(request_id));
        assert!(matches!(decoded.result, CommandResult::Success));
    }

    #[test]
    fn node_info_and_command_result_roundtrip() {
        let mut nodes = HashMap::new();
        nodes.insert(
            "source-1".to_string(),
            NodeInfo::Source(SourceInfo {
                uri: "videogenerator://source-1".to_string(),
                video_consumer_slot_ids: Some(vec!["slot-video".to_string()]),
                audio_consumer_slot_ids: Some(vec!["slot-audio".to_string()]),
                cue_time: None,
                end_time: None,
                state: State::Started,
            }),
        );

        let result = CommandResult::Info(Info { nodes });
        let encoded = serde_json::to_string(&result).unwrap();
        let decoded: CommandResult = serde_json::from_str(&encoded).unwrap();
        match decoded {
            CommandResult::Info(info) => {
                assert!(info.nodes.contains_key("source-1"));
            }
            other => panic!("expected info, got {other:?}"),
        }
    }
}
