# Phase 31 — Streaming Protocols Placeholder

> **UI-only.** Per-protocol settings forms (RTMP / SRT / RIST / WHIP / SRTLA)
> consumed by Phase 30's wizard and per-stream pages. **No real protocol
> code, no SRT adaptive bitrate logic.**

**Status:** `[ ] Not started`
**Depends on:** Phases 1, 2, 3, 7, 30
**Functional integration:** Permanently deferred — FCast does not author these protocols.

**Moblin source analogues** (~10 files):
- `View/Settings/Streams/Stream/Rtmp/StreamRtmpSettingsView.swift`
- `View/Settings/Streams/Stream/Srt/StreamSrtSettingsView.swift`
- `View/Settings/Streams/Stream/Srt/StreamSrtAdaptiveBitrateSettingsView.swift`
- `View/Settings/Streams/Stream/Srt/StreamSrtConnectionPriority2View.swift`
- `View/Settings/Streams/Stream/Rist/StreamRistSettingsView.swift`
- `View/Settings/Streams/Stream/Whip/StreamWhipSettingsView.swift`

**Files to add:**
- `senders/android/ui/pages/protocol_rtmp_settings_page.slint`
- `senders/android/ui/pages/protocol_srt_settings_page.slint`
- `senders/android/ui/pages/protocol_srt_adaptive_bitrate_page.slint`
- `senders/android/ui/pages/protocol_srt_connection_priority_page.slint`
- `senders/android/ui/pages/protocol_rist_settings_page.slint`
- `senders/android/ui/pages/protocol_whip_settings_page.slint`

---

## Goal

Surface the per-protocol parameter sets so wizards in Phase 30 have something
to delegate to. The point is to lock in the visual language for protocol
configuration (server URL, port, latency, FEC, encryption) without writing
any client code.

---

## Moblin source pattern

Each protocol's settings view is a `Form` with sections for *Connection*,
*Reliability*, *Encryption*, *Diagnostics*. Latency and bitrate are
sliders; encryption uses a `Picker` over an enum.

```swift
// View/Settings/Streams/Stream/Srt/StreamSrtSettingsView.swift (excerpt)
Form {
    Section { TextEditNavigationView(value: $stream.srt.url) }
    Section {
        Picker("Latency", selection: $stream.srt.latency) {
            ForEach(SrtLatency.allCases, id: \.self) { Text("\($0.rawValue) ms") }
        }
        Picker("Encryption", selection: $stream.srt.encryption) {
            ForEach(SrtEncryption.allCases, id: \.self) { Text($0.toString()) }
        }
    }
}
```

In Slint each picker is a `SettingsValueRow` cycler that walks an inline
options array.

---

## Tasks

### 31-A — Add Panel variants

- [ ] In `bridge.slint`'s `Panel` enum: `protocol-rtmp`, `protocol-srt`,
  `protocol-srt-adaptive`, `protocol-srt-priority`, `protocol-rist`,
  `protocol-whip`.

### 31-B — Reusable LatencyValueRow / EncryptionValueRow primitives

For each pickable setting that's used by 3+ protocol pages, prefer
inlining a `SettingsValueRow` cycler over creating a wrapper component
— keep the placeholder simple. Example:

```slint
// Inside ProtocolSrtSettingsPage
property <[int]>     srt-latency-options-ms: [120, 200, 500, 1000, 2000];
in-out property <int> mock-srt-latency-idx: 1;

SettingsValueRow {
    label: "Latency";
    value: "\{srt-latency-options-ms[root.mock-srt-latency-idx]} ms";
    clicked => {
        root.mock-srt-latency-idx =
            mod(root.mock-srt-latency-idx + 1, srt-latency-options-ms.length);
    }
}
```

### 31-C — `ProtocolRtmpSettingsPage`

- [ ] Sections:
  - URL (LineEdit `password` mode for stream key)
  - "Reveal stream key" toggle
  - Reconnect-on-disconnect toggle

### 31-D — `ProtocolSrtSettingsPage`

- [ ] Sections:
  - URL (multi-line, supports `srt://host:port?streamid=...&latency=...`)
  - Latency picker (cycler — see 31-B)
  - Encryption picker (`["None", "AES-128", "AES-192", "AES-256"]`)
  - Passphrase LineEdit (revealed if encryption ≠ None)
  - Adaptive bitrate row → `protocol-srt-adaptive`
  - Connection priority row → `protocol-srt-priority`
  - Stats grid (placeholder values: RTT, packet loss %, sent bitrate)

### 31-E — `ProtocolSrtAdaptiveBitratePage`

- [ ] Toggle "Enabled", min/max bitrate sliders, `mock-algorithm-idx`
  cycler (`["Belabox", "Conservative", "Aggressive"]`), step-down
  step-size slider.

### 31-F — `ProtocolSrtConnectionPriorityPage`

- [ ] List of network interfaces (cellular / Wi-Fi / Ethernet) each
  with priority value (1–5) cycled via `SettingsValueRow`. Inline
  `mock-priorities: [{ interface: string, priority: int }]`.

### 31-G — `ProtocolRistSettingsPage`

- [ ] URL, profile picker (`["Simple", "Main", "Advanced"]`), bonded
  links section (similar `mock-links: [{ host, port, weight }]` table).

### 31-H — `ProtocolWhipSettingsPage`

- [ ] URL (`https://...`), Bearer token LineEdit (`password` mode),
  ICE servers list (read-only stub with one placeholder STUN server).

---

## Exit criteria

1. Each of the 6 pages renders the correct sections.
2. Cycler values increment correctly (last item → wraps back to first).
3. URLs and tokens are echoed back as the user types (LineEdit
   binding works).
4. `cargo build -p android-sender` passes.

---

## What's NOT in this phase

- Real RTMP / SRT / RIST / WHIP client implementation.
- Real packet-loss measurement, RTT calculation, or stats.
- Real ICE candidate gathering.
- Validation of URLs (the wizard accepts whatever the user types).

---

## Slint best practices applied here

- **Inline cycler `mod(idx + 1, len)` pattern** is the cheapest
  placeholder for a `Picker` enum without wiring a `ComboBox`.
- **`LineEdit { input-type: password; }`** for stream keys / passphrases —
  `LineEdit` is from `std-widgets.slint` and supports a `password` mode
  (per the [LineEdit reference](https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/reference/std-widgets/views/lineedit.mdx)).
- **`if encryption-idx > 0: SettingsTextRow { label: "Passphrase"; }`** —
  conditional pages inside the same surface keep field count short
  when defaults are picked.
