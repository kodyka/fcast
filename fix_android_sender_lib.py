import sys

def fix_android_sender3(filename):
    with open(filename, 'r') as f:
        content = f.read()

    # match
    content = content.replace("                    Event::StateChanged(device_connection_state) => {\n                        if let device::DeviceConnectionState::Connected { local_addr, .. } = device_connection_state {\n                            self.local_address = Some(local_addr);\n\n                            self.ui_weak.upgrade_in_event_loop(|ui| {\n                                ui.global::<Bridge>().set_banner_visible(false);\n                                ui.global::<Bridge>().set_banner_message(\"\".into());\n                            })?;\n                        }\n                    }", "                    Event::StateChanged(device::DeviceConnectionState::Connected { local_addr, .. }) => {\n                        self.local_address = Some(local_addr);\n\n                        self.ui_weak.upgrade_in_event_loop(|ui| {\n                            ui.global::<Bridge>().set_banner_visible(false);\n                            ui.global::<Bridge>().set_banner_message(\"\".into());\n                        })?;\n                    }\n                    Event::StateChanged(_) => {}")

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
