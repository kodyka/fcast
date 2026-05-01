# Task 2 — Handle `rtmp2sink` absence for RTMP destinations

- **Priority:** P0 (Critical — app crashes / features completely broken)
- **Source:** [../TODO.codecs.md §2](../TODO.codecs.md)
- **File:** `senders/android/src/migration/nodes/destination.rs`
- **Function:** `build_live_pipeline()` → `DestinationFamily::Rtmp` arm
- **Approx. lines:** ~200-260

## Problem

`rtmp2sink` is from `gst-plugins-rs` — not in the prebuilt Android SDK.

## Tasks

- [ ] Try `rtmp2sink` first, fall back to legacy `rtmpsink` (in
      `GSTREAMER_PLUGINS_NET_RESTRICTED`).
- [ ] If using `rtmpsink`, remove `tls-validation-flags` property set (not
      supported on the legacy element).
- [ ] Add a warning log when falling back to `rtmpsink` about thread-safety
      and deprecation (removal in GStreamer 1.28).
- [ ] Document that RTMP over TLS (`rtmps://`) will not work with legacy
      `rtmpsink`.
- [ ] Long-term: evaluate migrating RTMP destinations to `whipclientsink`
      (WebRTC/WHIP).

## Suggested pattern

```rust
// TODO pattern:
let sink = Self::make_element("rtmp2sink", None)
    .or_else(|_| {
        tracing::warn!("rtmp2sink unavailable, falling back to deprecated rtmpsink");
        Self::make_element("rtmpsink", None)
    })?;
```

## Related tasks

- [12 — WHIP as RTMP replacement](12-whip-as-rtmp-replacement.md)
- [16 — `gst-plugins-rs` cross-compilation pipeline](16-gst-plugins-rs-cross-compile.md)
