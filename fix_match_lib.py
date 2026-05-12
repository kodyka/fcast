import sys

def fix(filename):
    with open(filename, 'r') as f:
        content = f.read()

    find_str = """                        DeviceEvent::StateChanged(device_connection_state) => {
                            if let device::DeviceConnectionState::Connected { local_addr, .. } = device_connection_state {
                                self.local_address = Some(local_addr);

                                self.ui_weak.upgrade_in_event_loop(|ui| {
                                    ui.global::<Bridge>()
                                        .invoke_change_state(AppState::SelectingSettings);
                                })?;
                            }
                        }"""

    replace_str = """                        DeviceEvent::StateChanged(device::DeviceConnectionState::Connected { local_addr, .. }) => {
                            self.local_address = Some(local_addr);

                            self.ui_weak.upgrade_in_event_loop(|ui| {
                                ui.global::<Bridge>()
                                    .invoke_change_state(AppState::SelectingSettings);
                            })?;
                        }
                        DeviceEvent::StateChanged(_) => {}"""

    content = content.replace(find_str, replace_str)

    with open(filename, 'w') as f:
        f.write(content)

fix("senders/android/src/lib.rs")
