import sys

def fix_android_sender(filename):
    with open(filename, 'r') as f:
        content = f.read()

    # Allow large enum variant
    content = content.replace("pub enum MessageResult {", "#[allow(clippy::large_enum_variant)]\npub enum MessageResult {")
    content = content.replace("enum NodeRecord {", "#[allow(clippy::large_enum_variant)]\nenum NodeRecord {")
    content = content.replace("pub enum NodeInfo {", "#[allow(clippy::large_enum_variant)]\npub enum NodeInfo {")

    with open(filename, 'w') as f:
        f.write(content)

fix_android_sender("senders/android/src/migration/messages.rs")
fix_android_sender("senders/android/src/migration/node_manager.rs")
fix_android_sender("senders/android/src/migration/protocol.rs")

def fix_android_sender2(filename):
    with open(filename, 'r') as f:
        content = f.read()

    # question mark
    content = content.replace("    let Some(current) = before.or(after) else {\n        return None;\n    };", "    let current = before.or(after)?;")

    with open(filename, 'w') as f:
        f.write(content)

fix_android_sender2("senders/android/src/migration/nodes/control.rs")

def fix_android_sender3(filename):
    with open(filename, 'r') as f:
        content = f.read()

    # match
    content = content.replace("Event::StateChanged(device_connection_state) => {\n                            if let device::DeviceConnectionState::Connected { local_addr, .. } = device_connection_state {", "Event::StateChanged(device::DeviceConnectionState::Connected { local_addr, .. }) => {")
    content = content.replace("                            } else {\n                                // Handle other state changes here if needed\n                            }\n                        }", "                        }\n                        Event::StateChanged(_) => {\n                                // Handle other state changes here if needed\n                        }")

    # Allow dead code
    content = content.replace("struct RecordingTickerState", "#[allow(dead_code)]\nstruct RecordingTickerState")
    content = content.replace("fn spawn_recording_ticker", "#[allow(dead_code)]\nfn spawn_recording_ticker")
    content = content.replace("struct PlatformApp", "#[allow(dead_code)]\nstruct PlatformApp")
    content = content.replace("fn ensure_gstreamer_initialized", "#[allow(dead_code)]\nfn ensure_gstreamer_initialized")
    content = content.replace("enum JavaMethod", "#[allow(dead_code)]\nenum JavaMethod")
    content = content.replace("fn call_java_method_no_args", "#[allow(dead_code)]\nfn call_java_method_no_args")
    content = content.replace("struct Application", "#[allow(dead_code)]\nstruct Application")
    content = content.replace("impl Application", "#[allow(dead_code)]\nimpl Application")
    content = content.replace("fn default_presets", "#[allow(dead_code)]\nfn default_presets")
    content = content.replace("fn default_quick_actions", "#[allow(dead_code)]\nfn default_quick_actions")

    with open(filename, 'w') as f:
        f.write(content)

fix_android_sender3("senders/android/src/lib.rs")

def fix_android_sender4(filename):
    with open(filename, 'r') as f:
        content = f.read()
    content = content.replace("struct LinkRecord {\n    source: String,\n    destination: String,\n    config: Option<HashMap<String, Value>>,\n}", "#[allow(dead_code)]\nstruct LinkRecord {\n    source: String,\n    destination: String,\n    config: Option<HashMap<String, Value>>,\n}")
    with open(filename, 'w') as f:
        f.write(content)

fix_android_sender4("senders/android/src/migration/node_manager.rs")
