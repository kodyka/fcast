# Task 1 — Replace H.264 software encoder fallback chain with Android MediaCodec

- **Priority:** P0 (Critical — app crashes / features completely broken)
- **Source:** [../TODO.codecs.md §1](../TODO.codecs.md)
- **File:** `senders/android/src/migration/nodes/destination.rs`
- **Function:** `select_video_encoder()`
- **Approx. lines:** ~155-170

## Problem

The current encoder selection tries `nvh264enc`, `x264enc`, `openh264enc` —
all three are absent from the prebuilt Android SDK. Every non-`LocalPlayback`
destination (RTMP, UDP, LocalFile) will fail with:

> "Failed to create a H.264 video encoder"

```rust
// CURRENT (broken on Android)
for encoder in ["nvh264enc", "x264enc", "openh264enc"] { ... }

// TODO: Replace with rank-based hardware encoder discovery
```

## Tasks

- [ ] Add rank-based `amcvidenc` discovery using
      `gst::ElementFactory::list_get_elements()` filtered by `video/x-h264`
      output caps.
- [ ] Fall back to any available `amcvidenc-*` variant before trying software
      encoders.
- [ ] Add `videoconvert` with explicit `video/x-raw,format=NV12` capsfilter
      upstream of `amcvidenc` to handle Android
      `COLOR_FormatYUV420SemiPlanar` requirement.
- [ ] Retain `x264enc` / `openh264enc` as last-resort fallbacks for non-Android
      builds.
- [ ] Gate encoder selection with `#[cfg(target_os = "android")]` where needed.
- [ ] Add per-device logging of which encoder was actually selected.
- [ ] Handle `amcvidenc` quirks: some devices require `bitrate` property in
      bps, others in kbps — probe the `GParamSpec` range.

## Affected destinations

- [ ] RTMP (`DestinationFamily::Rtmp`) — completely broken.
- [ ] UDP (`DestinationFamily::Udp`) — completely broken.
- [ ] LocalFile (`DestinationFamily::LocalFile`) — completely broken.
- [ ] LocalPlayback — unaffected (no encoder needed).

## Related tasks

- [04 — videoconvert color-space negotiation](04-videoconvert-color-space.md)
- [07 — rank-based encoder discovery](07-rank-based-encoder-discovery.md)
- [08 — `amcvidenc` property handling](08-amcvidenc-properties.md)
- [15 — `cfg(target_os = "android")` gating](15-cfg-target-os-android-gating.md)
