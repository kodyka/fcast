use crate::migration::{
    node_manager::NodeManager,
    protocol::{Command, CommandResult, ControllerMessage, ServerMessage},
};
use anyhow::{anyhow, Context, Result};
use parking_lot::Mutex;
use serde::Deserialize;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex as StdMutex,
    },
    thread::JoinHandle,
    time::Duration,
};
use tracing::{error, info, warn};

lazy_static::lazy_static! {
    static ref GRAPH_NODE_MANAGER: Mutex<NodeManager> = Mutex::new(NodeManager::default());
    static ref GRAPH_REFRESH_THREAD: StdMutex<Option<JoinHandle<()>>> = StdMutex::new(None);
    static ref GRAPH_REFRESH_RUNNING: AtomicBool = AtomicBool::new(false);
    static ref GRAPH_COMMAND_SERVER_THREAD: StdMutex<Option<JoinHandle<()>>> = StdMutex::new(None);
    static ref GRAPH_COMMAND_SERVER_RUNNING: AtomicBool = AtomicBool::new(false);
}

const GRAPH_REFRESH_INTERVAL: Duration = Duration::from_millis(100);
const GRAPH_COMMAND_POLL_INTERVAL: Duration = Duration::from_millis(50);
const GRAPH_COMMAND_MAX_REQUEST_SIZE: usize = 1024 * 1024;
const GRAPH_COMMAND_BIND_ENV: &str = "MIGRATION_COMMAND_BIND";

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum InboundCommand {
    Controller(ControllerMessage),
    Command(Command),
}

fn ensure_refresh_thread_running() -> Result<()> {
    let mut thread_slot = GRAPH_REFRESH_THREAD
        .lock()
        .map_err(|_| anyhow!("Graph refresh thread mutex is poisoned"))?;

    if thread_slot.is_some() {
        GRAPH_REFRESH_RUNNING.store(true, Ordering::SeqCst);
        return Ok(());
    }

    GRAPH_REFRESH_RUNNING.store(true, Ordering::SeqCst);
    let handle = std::thread::Builder::new()
        .name("graph-runtime-refresh".to_string())
        .spawn(|| {
            while GRAPH_REFRESH_RUNNING.load(Ordering::SeqCst) {
                GRAPH_NODE_MANAGER.lock().tick();
                std::thread::sleep(GRAPH_REFRESH_INTERVAL);
            }
        })
        .context("Failed to spawn graph runtime refresh thread")?;

    *thread_slot = Some(handle);
    Ok(())
}

fn stop_refresh_thread() {
    GRAPH_REFRESH_RUNNING.store(false, Ordering::SeqCst);

    if let Ok(mut thread_slot) = GRAPH_REFRESH_THREAD.lock() {
        if let Some(handle) = thread_slot.take() {
            let _ = handle.join();
        }
    }
}

fn command_endpoint_bind_from_env() -> Option<String> {
    std::env::var(GRAPH_COMMAND_BIND_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn build_http_response(status: &str, content_type: &str, body: &[u8]) -> Vec<u8> {
    let mut response = Vec::with_capacity(body.len() + 256);
    response.extend_from_slice(format!("HTTP/1.1 {status}\r\n").as_bytes());
    response.extend_from_slice(format!("Content-Type: {content_type}\r\n").as_bytes());
    response.extend_from_slice(format!("Content-Length: {}\r\n", body.len()).as_bytes());
    response.extend_from_slice(b"Connection: close\r\n\r\n");
    response.extend_from_slice(body);
    response
}

fn parse_http_request(stream: &mut TcpStream) -> Result<(String, String, Vec<u8>), String> {
    let mut buffer = Vec::new();
    let mut temp = [0u8; 4096];
    let mut header_end = None;

    while header_end.is_none() && buffer.len() < GRAPH_COMMAND_MAX_REQUEST_SIZE {
        let read = stream
            .read(&mut temp)
            .map_err(|err| format!("Failed to read HTTP request: {err}"))?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&temp[..read]);
        header_end = buffer.windows(4).position(|window| window == b"\r\n\r\n");
    }

    let Some(header_idx) = header_end else {
        return Err("Malformed HTTP request: missing header terminator".to_string());
    };

    let headers_end = header_idx + 4;
    let header_bytes = &buffer[..header_idx];
    let headers = std::str::from_utf8(header_bytes)
        .map_err(|_| "Malformed HTTP request: invalid UTF-8 headers".to_string())?;

    let mut lines = headers.lines();
    let request_line = lines
        .next()
        .ok_or_else(|| "Malformed HTTP request: missing request line".to_string())?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts
        .next()
        .ok_or_else(|| "Malformed HTTP request: missing method".to_string())?
        .to_string();
    let path = request_parts
        .next()
        .ok_or_else(|| "Malformed HTTP request: missing path".to_string())?
        .to_string();

    let content_length = lines
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.trim().eq_ignore_ascii_case("content-length") {
                value.trim().parse::<usize>().ok()
            } else {
                None
            }
        })
        .next()
        .unwrap_or(0);

    if content_length > GRAPH_COMMAND_MAX_REQUEST_SIZE {
        return Err("HTTP request body is too large".to_string());
    }

    let mut body = buffer[headers_end..].to_vec();
    while body.len() < content_length {
        let read = stream
            .read(&mut temp)
            .map_err(|err| format!("Failed to read HTTP body: {err}"))?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&temp[..read]);
        if body.len() > GRAPH_COMMAND_MAX_REQUEST_SIZE {
            return Err("HTTP request body is too large".to_string());
        }
    }

    if body.len() > content_length {
        body.truncate(content_length);
    }

    Ok((method, path, body))
}

fn handle_command_http_request(method: &str, path: &str, body: &[u8]) -> (String, String, Vec<u8>) {
    match (method, path) {
        ("POST", "/command") => {
            let payload = String::from_utf8_lossy(body);
            let response = try_handle_command_json(&payload);
            (
                "200 OK".to_string(),
                "application/json".to_string(),
                response.into_bytes(),
            )
        }
        ("GET", "/health") => (
            "200 OK".to_string(),
            "application/json".to_string(),
            br#"{"status":"ok"}"#.to_vec(),
        ),
        ("POST", _) | ("GET", _) => (
            "404 Not Found".to_string(),
            "application/json".to_string(),
            br#"{"error":"not found"}"#.to_vec(),
        ),
        _ => (
            "405 Method Not Allowed".to_string(),
            "application/json".to_string(),
            br#"{"error":"method not allowed"}"#.to_vec(),
        ),
    }
}

fn handle_command_http_connection(stream: &mut TcpStream) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));

    let response = match parse_http_request(stream) {
        Ok((method, path, body)) => {
            let (status, content_type, payload) =
                handle_command_http_request(&method, &path, &body);
            build_http_response(&status, &content_type, &payload)
        }
        Err(err) => build_http_response(
            "400 Bad Request",
            "application/json",
            format!(r#"{{"error":"{err}"}}"#).as_bytes(),
        ),
    };

    let _ = stream.write_all(&response);
    let _ = stream.flush();
}

fn ensure_command_server_running() -> Result<()> {
    let Some(bind_addr) = command_endpoint_bind_from_env() else {
        return Ok(());
    };

    let mut thread_slot = GRAPH_COMMAND_SERVER_THREAD
        .lock()
        .map_err(|_| anyhow!("Graph command server mutex is poisoned"))?;
    if thread_slot.is_some() {
        GRAPH_COMMAND_SERVER_RUNNING.store(true, Ordering::SeqCst);
        return Ok(());
    }

    GRAPH_COMMAND_SERVER_RUNNING.store(true, Ordering::SeqCst);
    let bind_addr_for_thread = bind_addr.clone();
    let handle = std::thread::Builder::new()
        .name("graph-command-endpoint".to_string())
        .spawn(move || {
            let listener = match TcpListener::bind(&bind_addr_for_thread) {
                Ok(listener) => listener,
                Err(err) => {
                    error!(
                        ?err,
                        bind_addr = %bind_addr_for_thread,
                        "Failed to bind graph command endpoint"
                    );
                    return;
                }
            };

            if let Err(err) = listener.set_nonblocking(true) {
                error!(
                    ?err,
                    bind_addr = %bind_addr_for_thread,
                    "Failed to set nonblocking mode for graph command endpoint"
                );
                return;
            }

            info!(
                bind_addr = %bind_addr_for_thread,
                "Graph command endpoint started"
            );

            while GRAPH_COMMAND_SERVER_RUNNING.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, peer_addr)) => {
                        handle_command_http_connection(&mut stream);
                        let _ = stream.shutdown(std::net::Shutdown::Both);
                        info!(peer = %peer_addr, "Processed graph command endpoint request");
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(GRAPH_COMMAND_POLL_INTERVAL);
                    }
                    Err(err) => {
                        warn!(?err, "Graph command endpoint accept failed");
                        std::thread::sleep(GRAPH_COMMAND_POLL_INTERVAL);
                    }
                }
            }

            info!(
                bind_addr = %bind_addr_for_thread,
                "Graph command endpoint stopped"
            );
        })
        .with_context(|| {
            format!("Failed to spawn graph command endpoint thread for {bind_addr}")
        })?;

    *thread_slot = Some(handle);
    Ok(())
}

fn stop_command_server_thread() {
    GRAPH_COMMAND_SERVER_RUNNING.store(false, Ordering::SeqCst);

    if let Ok(mut thread_slot) = GRAPH_COMMAND_SERVER_THREAD.lock() {
        if let Some(handle) = thread_slot.take() {
            let _ = handle.join();
        }
    }
}

pub fn start_graph_runtime() -> Result<()> {
    {
        let mut manager = GRAPH_NODE_MANAGER.lock();
        manager.start();
    }
    ensure_refresh_thread_running()?;
    ensure_command_server_running()?;
    Ok(())
}

pub fn shutdown_graph_runtime() -> Result<()> {
    stop_command_server_thread();
    stop_refresh_thread();
    {
        let mut manager = GRAPH_NODE_MANAGER.lock();
        manager.shutdown();
    }
    Ok(())
}

pub fn handle_command(command: Command) -> CommandResult {
    GRAPH_NODE_MANAGER.lock().dispatch(command)
}

pub fn handle_controller_message(message: ControllerMessage) -> ServerMessage {
    let result = handle_command(message.command);
    ServerMessage {
        id: Some(message.id),
        result,
    }
}

pub fn handle_command_json(payload: &str) -> Result<String> {
    let inbound: InboundCommand =
        serde_json::from_str(payload).context("Failed to parse command JSON payload")?;

    let response = match inbound {
        InboundCommand::Controller(msg) => handle_controller_message(msg),
        InboundCommand::Command(command) => ServerMessage {
            id: None,
            result: handle_command(command),
        },
    };

    serde_json::to_string(&response).context("Failed to serialize command response")
}

pub fn try_handle_command_json(payload: &str) -> String {
    match handle_command_json(payload) {
        Ok(response) => response,
        Err(err) => {
            error!(?err, "Failed to handle graph command JSON");
            serde_json::to_string(&ServerMessage {
                id: None,
                result: CommandResult::Error(format!("Invalid command payload: {err}")),
            })
            .unwrap_or_else(|ser_err| {
                format!(
                    "{{\"id\":null,\"result\":{{\"error\":\"Serialization failure: {}\"}}}}",
                    ser_err
                )
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration::protocol::{Command, CommandResult, DestinationFamily, ServerMessage};
    use serde_json::json;
    use std::io::Write;
    use std::net::Shutdown;
    use std::sync::{Mutex as StdMutex, OnceLock};
    use uuid::Uuid;

    fn test_lock() -> &'static StdMutex<()> {
        static LOCK: OnceLock<StdMutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| StdMutex::new(()))
    }

    fn test_guard() -> std::sync::MutexGuard<'static, ()> {
        test_lock().lock().unwrap_or_else(|err| err.into_inner())
    }

    fn parse_request_from_wire(payload: &[u8]) -> Result<(String, String, Vec<u8>), String> {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let payload = payload.to_vec();

        let writer = std::thread::spawn(move || {
            let mut client = TcpStream::connect(addr).unwrap();
            client.write_all(&payload).unwrap();
            client.shutdown(Shutdown::Write).unwrap();
        });

        let (mut stream, _) = listener.accept().unwrap();
        let parsed = parse_http_request(&mut stream);
        writer.join().unwrap();
        parsed
    }

    #[test]
    fn json_command_roundtrip() {
        let _guard = test_guard();
        let _ = shutdown_graph_runtime();
        std::env::remove_var(GRAPH_COMMAND_BIND_ENV);
        start_graph_runtime().unwrap();

        let payload = serde_json::to_string(&Command::CreateSource {
            id: "test-source".to_string(),
            uri: "https://example.com/video.mp4".to_string(),
            audio: true,
            video: true,
        })
        .unwrap();

        let response = handle_command_json(&payload).unwrap();
        assert!(response.contains("success"));

        let payload = serde_json::to_string(&Command::CreateDestination {
            id: "test-destination".to_string(),
            family: DestinationFamily::LocalPlayback,
            audio: true,
            video: true,
        })
        .unwrap();
        let response = handle_command_json(&payload).unwrap();
        assert!(response.contains("success"));

        let payload = serde_json::to_string(&Command::Connect {
            link_id: "test-link".to_string(),
            src_id: "test-source".to_string(),
            sink_id: "test-destination".to_string(),
            audio: true,
            video: true,
            config: None,
        })
        .unwrap();
        let response = handle_command_json(&payload).unwrap();
        assert!(response.contains("success"));

        shutdown_graph_runtime().unwrap();
    }

    #[test]
    fn handle_command_json_accepts_controller_payloads() {
        let _guard = test_guard();
        let _ = shutdown_graph_runtime();
        std::env::remove_var(GRAPH_COMMAND_BIND_ENV);
        start_graph_runtime().unwrap();

        let request_id = Uuid::new_v4();
        let payload = json!({
            "id": request_id,
            "command": {
                "getinfo": {}
            }
        })
        .to_string();

        let response = handle_command_json(&payload).unwrap();
        let server_message: ServerMessage = serde_json::from_str(&response).unwrap();
        assert_eq!(server_message.id, Some(request_id));
        assert!(matches!(server_message.result, CommandResult::Info(_)));

        shutdown_graph_runtime().unwrap();
    }

    #[test]
    fn try_handle_command_json_wraps_invalid_payload_errors() {
        let _guard = test_guard();
        let _ = shutdown_graph_runtime();

        let response = try_handle_command_json("{invalid");
        let parsed: ServerMessage = serde_json::from_str(&response).unwrap();

        match parsed.result {
            CommandResult::Error(err) => {
                assert!(err.contains("Invalid command payload"));
            }
            other => panic!("expected command error, got {other:?}"),
        }
    }

    #[test]
    fn command_http_request_routes_to_command_handler() {
        let _guard = test_guard();
        let payload = serde_json::to_string(&Command::GetInfo { id: None }).unwrap();
        let (status, content_type, body) =
            handle_command_http_request("POST", "/command", payload.as_bytes());
        let body = String::from_utf8(body).unwrap();

        assert_eq!(status, "200 OK");
        assert_eq!(content_type, "application/json");
        assert!(body.contains("\"result\""));
    }

    #[test]
    fn command_http_request_reports_404_for_unknown_path() {
        let _guard = test_guard();
        let (status, _, _) = handle_command_http_request("POST", "/unknown", b"{}");
        assert_eq!(status, "404 Not Found");
    }

    #[test]
    fn command_http_request_supports_health_and_method_checks() {
        let _guard = test_guard();

        let (status, content_type, body) = handle_command_http_request("GET", "/health", b"");
        assert_eq!(status, "200 OK");
        assert_eq!(content_type, "application/json");
        assert_eq!(body, br#"{"status":"ok"}"#.to_vec());

        let (status, _, body) = handle_command_http_request("PUT", "/command", b"{}");
        assert_eq!(status, "405 Method Not Allowed");
        assert_eq!(body, br#"{"error":"method not allowed"}"#.to_vec());
    }

    #[test]
    fn command_endpoint_bind_env_trims_and_filters_empty_values() {
        let _guard = test_guard();

        std::env::remove_var(GRAPH_COMMAND_BIND_ENV);
        assert_eq!(command_endpoint_bind_from_env(), None);

        std::env::set_var(GRAPH_COMMAND_BIND_ENV, "   ");
        assert_eq!(command_endpoint_bind_from_env(), None);

        std::env::set_var(GRAPH_COMMAND_BIND_ENV, " 127.0.0.1:8999 ");
        assert_eq!(
            command_endpoint_bind_from_env(),
            Some("127.0.0.1:8999".to_string())
        );
        std::env::remove_var(GRAPH_COMMAND_BIND_ENV);
    }

    #[test]
    fn parse_http_request_parses_method_path_and_body() {
        let _guard = test_guard();

        let request =
            b"POST /command HTTP/1.1\r\nHost: localhost\r\nContent-Length: 7\r\n\r\n{\"a\":1}";
        let (method, path, body) = parse_request_from_wire(request).unwrap();

        assert_eq!(method, "POST");
        assert_eq!(path, "/command");
        assert_eq!(body, b"{\"a\":1}");
    }

    #[test]
    fn parse_http_request_rejects_missing_header_terminator() {
        let _guard = test_guard();

        let request = b"POST /command HTTP/1.1\r\nHost: localhost\r\nContent-Length: 2\r\n{\"";
        let err = parse_request_from_wire(request).unwrap_err();
        assert!(err.contains("missing header terminator"));
    }

    #[test]
    fn parse_http_request_rejects_oversized_content_length() {
        let _guard = test_guard();

        let request = format!(
            "POST /command HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n",
            GRAPH_COMMAND_MAX_REQUEST_SIZE + 1
        );
        let err = parse_request_from_wire(request.as_bytes()).unwrap_err();
        assert!(err.contains("body is too large"));
    }

    #[test]
    fn legacy_curl_script_payloads_are_compatible() {
        let _guard = test_guard();
        let _ = shutdown_graph_runtime();
        std::env::remove_var(GRAPH_COMMAND_BIND_ENV);
        start_graph_runtime().unwrap();

        let commands = [
            r#"{"createmixer":{"id":"channel-1"}}"#,
            r#"{"createdestination":{"id":"centricular-output","family":"LocalPlayback"}}"#,
            r#"{"connect":{"link_id":"channel-1","src_id":"channel-1","sink_id":"centricular-output"}}"#,
            r#"{"getinfo":{}}"#,
        ];

        for payload in commands {
            let (status, content_type, body) =
                handle_command_http_request("POST", "/command", payload.as_bytes());
            let body = String::from_utf8(body).unwrap();
            assert_eq!(status, "200 OK");
            assert_eq!(content_type, "application/json");
            assert!(body.contains("\"result\""));
        }

        shutdown_graph_runtime().unwrap();
    }
}
