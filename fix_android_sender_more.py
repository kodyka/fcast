import sys

def fix_android_sender_lib(filename):
    with open(filename, 'r') as f:
        content = f.read()

    # The previous attempt didn't replace because there's a DeviceEvent rather than Event. Let's fix.
    content = content.replace("                    DeviceEvent::StateChanged(device_connection_state) => {\n                        if let device::DeviceConnectionState::Connected { local_addr, .. } = device_connection_state {\n                            self.local_address = Some(local_addr);\n\n                            self.ui_weak.upgrade_in_event_loop(|ui| {\n                                ui.global::<Bridge>().set_banner_visible(false);\n                                ui.global::<Bridge>().set_banner_message(\"\".into());\n                            })?;\n                        }\n                    }", "                    DeviceEvent::StateChanged(device::DeviceConnectionState::Connected { local_addr, .. }) => {\n                        self.local_address = Some(local_addr);\n\n                        self.ui_weak.upgrade_in_event_loop(|ui| {\n                            ui.global::<Bridge>().set_banner_visible(false);\n                            ui.global::<Bridge>().set_banner_message(\"\".into());\n                        })?;\n                    }\n                    DeviceEvent::StateChanged(_) => {}")

    with open(filename, 'w') as f:
        f.write(content)

fix_android_sender_lib("senders/android/src/lib.rs")

def fix_android_sender_protocol(filename):
    with open(filename, 'r') as f:
        content = f.read()

    content = content.replace("        let mut points = vec![\n            point_at(20, json!(1.0)),\n            point_at(-10, json!(2.0)),\n            point_at(5, json!(3.0)),\n        ];", "        let mut points = [\n            point_at(20, json!(1.0)),\n            point_at(-10, json!(2.0)),\n            point_at(5, json!(3.0)),\n        ];")

    with open(filename, 'w') as f:
        f.write(content)

fix_android_sender_protocol("senders/android/src/migration/protocol.rs")

def fix_android_sender_node_manager(filename):
    with open(filename, 'r') as f:
        content = f.read()
    content = content.replace("    #[allow(dead_code)]\n    config: Option<HashMap<String, Value>>,", "    config: Option<HashMap<String, Value>>,") # we might have missed adding allow to the struct.

    content = content.replace("struct LinkRecord {\n    source: String,\n    destination: String,\n    config: Option<HashMap<String, Value>>,\n}", "#[allow(dead_code)]\nstruct LinkRecord {\n    source: String,\n    destination: String,\n    config: Option<HashMap<String, Value>>,\n}")

    content = content.replace("struct LinkRecord {\n    source: String,\n    destination: String,\n    #[allow(dead_code)]\n    config: Option<HashMap<String, Value>>,\n}", "#[allow(dead_code)]\nstruct LinkRecord {\n    source: String,\n    destination: String,\n    #[allow(dead_code)]\n    config: Option<HashMap<String, Value>>,\n}")

    # Adding allow directly to the field
    content = content.replace("config: Option<HashMap<String, Value>>,", "#[allow(dead_code)]\n    config: Option<HashMap<String, Value>>,")

    with open(filename, 'w') as f:
        f.write(content)

fix_android_sender_node_manager("senders/android/src/migration/node_manager.rs")
