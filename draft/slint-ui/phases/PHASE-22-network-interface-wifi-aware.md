# Phase 22 — Network Interface & Wi-Fi Aware Discovery Placeholder

> Settings sub-page listing network interfaces (Wi-Fi, Ethernet, cellular) and
> a Wi-Fi Aware (NAN) opt-in toggle. **UI-only — no real interface
> enumeration, no Wi-Fi Aware permission flow.**

**Status:** `[ ] Not started`
**Depends on:** Phases 2, 3, 7
**Functional integration:** Deferred — Wi-Fi Aware (NAN) requires
`android.permission.ACCESS_FINE_LOCATION` + `WifiAwareManager` API and is
blocked on Rust capability.
**Moblin source analogues:**
- `Settings/Display/NetworkInterfaceNames/LocalOverlaysNetworkInterfaceNamesSettingsView.swift`
- `Settings/WiFiAware/WiFiAwareSettingsView.swift`

**Files:**
- `senders/android/ui/pages/network_page.slint` — new
- `senders/android/ui/bridge.slint` — `NetworkInterface` struct + `Panel.network`

---

## Tasks

### 22-A — `NetworkInterface` struct

- [ ] In `bridge.slint`:

  ```slint
  export struct NetworkInterface {
      name:         string,
      kind:         string,    // "wifi" / "ethernet" / "cellular" / "loopback"
      address-v4:   string,
      address-v6:   string,
      enabled:      bool,
  }
  ```

---

### 22-B — `NetworkPage`

- [ ] Inline mock model:

  ```slint
  in-out property <[NetworkInterface]> mock-interfaces: [
      { name: "wlan0",  kind: "wifi",     address-v4: "192.168.1.42",   address-v6: "fe80::1234", enabled: true  },
      { name: "rmnet0", kind: "cellular", address-v4: "10.20.30.40",     address-v6: "",           enabled: false },
      { name: "lo",     kind: "loopback", address-v4: "127.0.0.1",       address-v6: "::1",        enabled: true  },
  ];
  in-out property <bool> mock-wifi-aware-enabled: false;
  ```

- [ ] Sections:

  ```
  INTERFACES
    For each: name + kind icon + v4 address (overflow: elide), trailing
    enable toggle. Tapping the row opens an inline expander showing v6
    address + a "Use for cast traffic" toggle.

  WI-FI AWARE
    Enable Wi-Fi Aware discovery     toggle (mock-wifi-aware-enabled)
    [Info banner if disabled: "Requires location permission. Enabling does
     not request the permission in this build (placeholder)."]
  ```

- [ ] **Build check.**

---

### 22-C — Wi-Fi Aware permission preview banner

- [ ] When `mock-wifi-aware-enabled` flips to `true`, show a transient banner:
  "Wi-Fi Aware enabled (placeholder — no permission requested)."

---

### 22-D — Bridge + linking

- [ ] Extend `Panel` with `network`.
- [ ] Route in `main.slint`.
- [ ] Link from `FullSettingsPage` "ADVANCED" section.

---

## Exit criteria

1. Network page lists 3 stub interfaces with kind icon + v4 address.
2. Per-interface enable toggle flips stub state.
3. Tapping a row expands to show v6 address.
4. Wi-Fi Aware toggle flips state and shows the placeholder banner on enable.
5. `cargo build -p android-sender` passes.

---

## What's NOT in this phase

- Real `NetworkInterface.getNetworkInterfaces()` enumeration.
- Real interface enable / disable (requires root or VPN profile).
- Real Wi-Fi Aware (NAN) permission flow + session lifecycle.
- Per-interface routing for cast traffic.

---

## Moblin source mapping & Slint primitives

**Source files referenced:**
- `View/Settings/NetworkInterfaceNames/NetworkInterfaceNamesSettingsView.swift`

**Representative SwiftUI excerpt:**

```swift
// View/Settings/NetworkInterfaceNames/NetworkInterfaceNamesSettingsView.swift (excerpt)
Form {
    Section {
        ForEach(database.networkInterfaceNames) { iface in
            HStack {
                TextField("Friendly name", text: $iface.name)
                Spacer()
                Text(iface.ip).foregroundColor(.secondary)
            }
        }
    }
}
```

**Mapping notes:**

A friendly-name editor for each network interface. In Slint each row is
`HorizontalLayout { LineEdit; Text /* IP */; }`. Wi-Fi Aware (Android NAN)
is a separate panel — placeholder shows the toggle + 0 peers detected.

**Relevant Slint docs:**
- [LineEdit](https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/reference/std-widgets/views/lineedit.mdx)

## Slint best practices applied here

- **Expander pattern via local boolean property** — each row stores its own
  `expanded` flag rather than tracking a global "selected interface". Avoids
  the need for ID-based selection state.
- **Info banner inline with the toggle** is simpler than a separate alert
  component for one-off messages.
