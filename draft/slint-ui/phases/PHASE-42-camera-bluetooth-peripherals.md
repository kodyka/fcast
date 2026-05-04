# Phase 42 — Camera / Bluetooth Peripherals Placeholder

> **UI-only.** DJI / GoPro / BlackShark cooler / cat-printer / gimbal /
> selfie-stick configuration pages. **No real Bluetooth, no real RTMP-on-DJI,
> no real GoPro WiFi protocol.**

**Status:** `[ ] Not started`
**Depends on:** Phases 1, 2, 3, 7
**Functional integration:** Permanently deferred — FCast Android sender does not pair with these peripherals.

**Moblin source analogues** (~12 files):
- `View/Settings/DjiDevices/{DjiDevicesSettingsView,DjiDeviceSettingsView}.swift` + `View/ControlBar/QuickButton/Dji/QuickButtonDjiDevicesView.swift`
- `View/Settings/GoPro/GoProSettingsView.swift` + `View/ControlBar/QuickButton/GoPro/QuickButtonGoProView.swift`
- `View/Settings/PhoneCoolers/PhoneCoolersSettingsView.swift` + `View/Settings/PhoneCoolers/BlackSharks/{BlackSharkPhoneCoolerSettingsView,BlackSharkPhoneCoolerScannerSettingsView}.swift`
- `View/Settings/CatPrinters/{CatPrintersSettingsView,CatPrinterSettingsView,CatPrinterScannerSettingsView}.swift`
- `View/Settings/Gimbal/GimbalSettingsView.swift`
- `View/Settings/SelfieStick/SelfieStickSettingsView.swift`

**Files to add:**
- `senders/android/ui/pages/peripherals_settings_page.slint` — root
- `senders/android/ui/pages/peripheral_dji_settings_page.slint`
- `senders/android/ui/pages/peripheral_gopro_settings_page.slint`
- `senders/android/ui/pages/peripheral_phone_coolers_page.slint`
- `senders/android/ui/pages/peripheral_blackshark_cooler_page.slint`
- `senders/android/ui/pages/peripheral_blackshark_cooler_scanner_page.slint`
- `senders/android/ui/pages/peripheral_cat_printers_page.slint`
- `senders/android/ui/pages/peripheral_cat_printer_settings_page.slint`
- `senders/android/ui/pages/peripheral_cat_printer_scanner_page.slint`
- `senders/android/ui/pages/peripheral_gimbal_settings_page.slint`
- `senders/android/ui/pages/peripheral_selfie_stick_settings_page.slint`

---

## Goal

A unified peripherals panel listing all paired / available accessories with
per-device sub-pages.

---

## Moblin source pattern

```swift
// DjiDevicesSettingsView.swift (excerpt)
List {
    ForEach(database.djiDevices) { device in
        NavigationLink { DjiDeviceSettingsView(device: device) } label: {
            HStack { Image(...); Text(device.name); Spacer(); ConnectionStatusView(...) }
        }
    }
    Button("Pair new device") { ... }
}
```

Common shape: a list per peripheral type + a "scan" / "pair" entry.

---

## Tasks

### 42-A — Add Panel variants

- [ ] `Panel.peripherals`, `Panel.peripheral-dji`, `Panel.peripheral-gopro`,
  `Panel.peripheral-phone-coolers`, `Panel.peripheral-blackshark`,
  `Panel.peripheral-blackshark-scan`, `Panel.peripheral-cat-printers`,
  `Panel.peripheral-cat-printer`, `Panel.peripheral-cat-printer-scan`,
  `Panel.peripheral-gimbal`, `Panel.peripheral-selfie-stick`.

### 42-B — `PeripheralsSettingsPage`

```slint
export component PeripheralsSettingsPage inherits Rectangle {
    VerticalLayout {
        SettingsSection { title: "VIDEO";
            SettingsValueRow { label: "DJI devices";   value: "0 paired";
                clicked => { Bridge.active-panel = Panel.peripheral-dji; } }
            SettingsValueRow { label: "GoPro";          value: "Not paired";
                clicked => { Bridge.active-panel = Panel.peripheral-gopro; } }
        }
        SettingsSection { title: "AUDIO / INPUT";
            SettingsValueRow { label: "Gimbal";         value: "Not paired";
                clicked => { Bridge.active-panel = Panel.peripheral-gimbal; } }
            SettingsValueRow { label: "Selfie stick";  value: "Not paired";
                clicked => { Bridge.active-panel = Panel.peripheral-selfie-stick; } }
        }
        SettingsSection { title: "OTHER";
            SettingsValueRow { label: "Phone coolers";  value: "Not paired";
                clicked => { Bridge.active-panel = Panel.peripheral-phone-coolers; } }
            SettingsValueRow { label: "Cat printers";   value: "Not paired";
                clicked => { Bridge.active-panel = Panel.peripheral-cat-printers; } }
        }
    }
}
```

### 42-C — `PeripheralDjiSettingsPage`

```slint
export struct DjiDevice {
    id: int,
    name: string,
    model: string,         // "Osmo Action 4" / "RS 4" / etc.
    connected: bool,
    rtmp-url: string,
}

in-out property <[DjiDevice]> mock-devices: [
    { id: 1, name: "Osmo (kitchen)", model: "Osmo Action 4", connected: false, rtmp-url: "rtmp://..." },
    { id: 2, name: "RS 4 gimbal",    model: "RS 4",          connected: false, rtmp-url: "" },
];

VerticalLayout {
    for d in root.mock-devices: SettingsValueRow {
        label: d.name + " (" + d.model + ")";
        value: d.connected ? "Connected" : "Disconnected";
        clicked => { /* open per-device settings — model picker, RTMP URL, mode */ }
    }
    PrimaryButton { label: "Pair new DJI device";
        clicked => { /* open Bluetooth scanner placeholder */ } }
}
```

### 42-D — `PeripheralGoProSettingsPage`

- [ ] Single GoPro device pairing surface (model picker, WiFi config
  fields, "Set up GoPro" wizard placeholder).

### 42-E — `PeripheralPhoneCoolersPage` + `PeripheralBlackSharkCoolerPage` + scanner

- [ ] Page lists supported cooler brands; tapping a brand opens its
  detail page (BlackShark gets full editor; others = "Not yet
  supported" stub).

### 42-F — `PeripheralCatPrintersPage` + per-device + scanner

- [ ] Same pattern: list, per-device detail (paper width picker,
  print-test button), Bluetooth scanner placeholder.

### 42-G — `PeripheralGimbalSettingsPage`

- [ ] Brand picker, follow-mode picker, calibration "tap to recalibrate"
  button.

### 42-H — `PeripheralSelfieStickSettingsPage`

- [ ] Action mapping: `mock-mappings: [{ button: string, action: string }]`
  (e.g. "Shutter → Toggle cast").

### 42-I — Scanner page template (`peripheral_*_scanner_page.slint`)

- [ ] All scanner pages share the same chrome:

  ```slint
  VerticalLayout {
      Text { text: "Scanning for nearby devices…"; color: Theme.text-secondary; }
      // Spinner placeholder
      LoadingView { /* from Phase 3 */ }
      // Scanned devices list
      for device in mock-found: SettingsValueRow {
          label: device.name; value: device.rssi + " dBm";
          clicked => { /* attempt to pair */ }
      }
      DestructiveButton { label: "Cancel";
          clicked => { /* close panel */ } }
  }
  ```

---

## Exit criteria

1. `PeripheralsSettingsPage` lists 6 peripheral categories.
2. Each per-peripheral page renders mock devices + pair button.
3. Scanner pages render a spinner + 2–3 mock found devices.
4. `cargo build -p android-sender` passes.

---

## What's NOT in this phase

- Any Bluetooth scanning / pairing.
- Real DJI Mimo / GoPro Quik protocol handling.
- Real BLE HID button mapping for selfie sticks.
- Real cat-printer thermal print pipeline.

---

## Slint best practices applied here

- **`LoadingView`** from Phase 3 reused for scanner spinners — no need
  for a custom spinner per page.
- **One scanner template, six occurrences** — keep the chrome
  consistent so the user immediately recognises the "scanning for
  Bluetooth" pattern across all peripherals.
