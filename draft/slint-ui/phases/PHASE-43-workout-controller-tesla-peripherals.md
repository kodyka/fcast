# Phase 43 — Workout / Game-controller / Tesla / External-camera Peripherals Placeholder

> **UI-only.** Heart-rate / cycling / power-meter workout devices, game
> controller mappings, Tesla vehicle integration, external USB camera, etc.
> **No real BLE / OBD / Tesla API / USB scanning.**

**Status:** `[ ] Not started`
**Depends on:** Phases 1, 2, 3, 7, 42 (peripherals root)
**Functional integration:** Permanently deferred.

**Moblin source analogues** (~9 files):
- `View/Settings/WorkoutDevices/{WorkoutDevicesSettingsView,WorkoutDeviceSettingsView,WorkoutDeviceScannerSettingsView}.swift`
- `View/Settings/GameControllers/{GameControllersSettingsView,GameControllerSettingsView,GameControllerInputSettingsView,GameControllerOnboardingView}.swift`
- `View/Settings/Tesla/TeslaSettingsView.swift`
- `View/Settings/Camera/ExternalCameraSettingsView.swift`

**Files to add:**
- `senders/android/ui/pages/peripheral_workout_devices_page.slint`
- `senders/android/ui/pages/peripheral_workout_device_page.slint`
- `senders/android/ui/pages/peripheral_workout_device_scanner_page.slint`
- `senders/android/ui/pages/peripheral_game_controllers_page.slint`
- `senders/android/ui/pages/peripheral_game_controller_page.slint`
- `senders/android/ui/pages/peripheral_game_controller_input_page.slint`
- `senders/android/ui/pages/peripheral_game_controller_onboarding_page.slint`
- `senders/android/ui/pages/peripheral_tesla_settings_page.slint`
- `senders/android/ui/pages/peripheral_external_camera_page.slint`

---

## Goal

Surface workout and game-controller integration UI as placeholders —
extending the peripherals root from Phase 42.

---

## Moblin source pattern

```swift
// WorkoutDevicesSettingsView.swift (excerpt)
List {
    ForEach(database.workoutDevices) { device in
        NavigationLink { WorkoutDeviceSettingsView(device: device) } label: {
            HStack {
                Image(systemName: device.icon)
                Text(device.name)
                Spacer()
                Text(device.lastValue)
            }
        }
    }
}
```

Matches the peripheral list shape from Phase 42.

---

## Tasks

### 43-A — Add Panel variants

- [ ] Extend `peripherals` root from Phase 42 to include workout
  devices / game controllers / Tesla / external camera. Add Panel
  variants: `peripheral-workout-devices`, `-workout-device`,
  `-workout-scanner`, `peripheral-game-controllers`, `-game-controller`,
  `-game-controller-input`, `-game-controller-onboarding`,
  `peripheral-tesla`, `peripheral-external-camera`.

- [ ] Update `PeripheralsSettingsPage` (Phase 42-B) with rows for
  Workout / Game controllers / Tesla / External camera.

### 43-B — `PeripheralWorkoutDevicesPage`

```slint
export struct WorkoutDevice {
    id: int,
    name: string,
    kind: string,                 // "heart-rate" / "cycling-cadence" / "power" / "speed"
    last-value: string,           // "67 bpm" / "92 rpm" / "243 W"
    connected: bool,
}

in-out property <[WorkoutDevice]> mock-devices: [
    { id: 1, name: "Polar H10",      kind: "heart-rate",      last-value: "—",     connected: false },
    { id: 2, name: "Wahoo Tickr",    kind: "heart-rate",      last-value: "67 bpm",connected: true  },
    { id: 3, name: "Garmin cadence", kind: "cycling-cadence", last-value: "—",     connected: false },
];
```

- [ ] List rendering + "Add device" → opens scanner.

### 43-C — `PeripheralWorkoutDevicePage`

- [ ] Per-device detail: kind label, last value, "Disconnect" button,
  data-broadcast toggle (whether to overlay on stream).

### 43-D — `PeripheralWorkoutDeviceScannerPage`

- [ ] Scanner template from Phase 42-I, scanning for BLE workout
  services (HRP, CSCS, CPS).

### 43-E — `PeripheralGameControllersPage`

- [ ] List of paired controllers (`mock-controllers: [{ name, vendor,
  battery-pct }]`), "Onboarding" button.

### 43-F — `PeripheralGameControllerPage`

- [ ] Per-controller settings: name (rename), input-mappings list →
  `peripheral-game-controller-input`.

### 43-G — `PeripheralGameControllerInputPage`

- [ ] Per-button mapping editor:

  ```slint
  export struct ButtonMapping {
      id: string,            // "A" / "B" / "X" / "Y" / "L1" / "R1" / "L2" / "R2" / "Up" / etc.
      label: string,
      action: string,        // resolves to QuickAction.id from Phase 17
  }

  in-out property <[ButtonMapping]> mock-mappings: [
      { id: "A", label: "A button", action: "toggle-cast" },
      { id: "B", label: "B button", action: "scan-qr" },
      // ...
  ];

  for m[i] in root.mock-mappings: SettingsValueRow {
      label: m.label; value: m.action;
      clicked => {
          // cycle through known action IDs
          let actions = ["toggle-cast", "scan-qr", "show-debug", "open-settings"];
          let idx = ...;       // pseudo: find in actions, +1 mod len
          root.mock-mappings[i].action = actions[mod(idx + 1, actions.length)];
      }
  }
  ```

### 43-H — `PeripheralGameControllerOnboardingPage`

- [ ] Multi-step "Press the A button now" detector — placeholder cycles
  through 4 fake "detected" states.

### 43-I — `PeripheralTeslaSettingsPage`

- [ ] Tesla account toggle, vehicle picker (`mock-vehicles: [string]`),
  metric overlay toggles (battery, range, gear, climate).

### 43-J — `PeripheralExternalCameraPage`

- [ ] USB device picker (`mock-devices: [{ vendor, product, format }]`),
  resolution/framerate cyclers.

---

## Exit criteria

1. Peripherals root updated to surface 4 new categories.
2. Each new category renders its list / per-device sub-page / scanner.
3. Game-controller input mapping cycles through available actions.
4. `cargo build -p android-sender` passes.

---

## What's NOT in this phase

- Real BLE workout protocol handling (HRP, CSCS, CPS).
- Real GameControllerKit / Android `InputDevice` integration.
- Real Tesla Vehicle Command API.
- Real USB camera enumeration / V4L2 / UVC.

---

## Slint best practices applied here

- **Indexed array mutation `mock-mappings[i].action = ...`** — Slint
  1.15 supports element mutation through the index syntax inside `for`
  loops.
- **Reuse of scanner template** from Phase 42-I avoids per-peripheral
  scanner boilerplate.
