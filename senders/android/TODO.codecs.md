# TODO: GStreamer Android Prebuilt SDK Compatibility

> **Per-task breakdown:** each numbered task below also has its own dedicated
> file under [`TODO.codecs/`](TODO.codecs/README.md) for independent tracking.

---

## P0 - Critical (App crashes / features completely broken)

### 1. Replace H.264 software encoder fallback chain with Android MediaCodec

**File:** `senders/android/src/migration/nodes/destination.rs`  
**Function:** `select_video_encoder()`  
**Lines:** ~155-170

The current encoder selection tries `nvh264enc`, `x264enc`, `openh264enc` - all three are absent from the prebuilt Android SDK. Every non-`LocalPlayback` destination (RTMP, UDP, LocalFile) will fail with `"Failed to create a H.264 video encoder"`.

```rust
// CURRENT (broken on Android)
for encoder in ["nvh264enc", "x264enc", "openh264enc"] { ... }

// TODO: Replace with rank-based hardware encoder discovery
```

**Tasks:**
- [ ] Add rank-based `amcvidenc` discovery using `gst::ElementFactory::list_get_elements()` filtered by `video/x-h264` output caps.
- [ ] Fall back to any available `amcvidenc-*` variant before trying software encoders.
- [ ] Add `videoconvert` with explicit `video/x-raw,format=NV12` capsfilter upstream of `amcvidenc` to handle Android `COLOR_FormatYUV420SemiPlanar` requirement.
- [ ] Retain `x264enc` / `openh264enc` as last-resort fallbacks for non-Android builds.
- [ ] Gate encoder selection with `#[cfg(target_os = "android")]` where needed.
- [ ] Add per-device logging of which encoder was actually selected.
- [ ] Handle `amcvidenc` quirks: some devices require `bitrate` property in bps, others in kbps - probe the `GParamSpec` range.

**Affected destinations:**
- [ ] RTMP (`DestinationFamily::Rtmp`) - completely broken.
- [ ] UDP (`DestinationFamily::Udp`) - completely broken.
- [ ] LocalFile (`DestinationFamily::LocalFile`) - completely broken.
- [ ] LocalPlayback - unaffected (no encoder needed).

---

### 2. Handle `rtmp2sink` absence for RTMP destinations

**File:** `senders/android/src/migration/nodes/destination.rs`  
**Function:** `build_live_pipeline()` -> `DestinationFamily::Rtmp` arm  
**Lines:** ~200-260

`rtmp2sink` is from `gst-plugins-rs` - not in prebuilt SDK.

**Tasks:**
- [ ] Try `rtmp2sink` first, fall back to legacy `rtmpsink` (in `GSTREAMER_PLUGINS_NET_RESTRICTED`).
- [ ] If using `rtmpsink`, remove `tls-validation-flags` property set (not supported on legacy element).
- [ ] Add warning log when falling back to `rtmpsink` about thread-safety and deprecation (removal in GStreamer 1.28).
- [ ] Document that RTMP over TLS (`rtmps://`) will not work with legacy `rtmpsink`.
- [ ] Long-term: evaluate migrating RTMP destinations to `whipclientsink` (WebRTC/WHIP).

```rust
// TODO pattern:
let sink = Self::make_element("rtmp2sink", None)
    .or_else(|_| {
        tracing::warn!("rtmp2sink unavailable, falling back to deprecated rtmpsink");
        Self::make_element("rtmpsink", None)
    })?;
```

---

### 3. Handle `fallbacksrc` absence gracefully

**File:** `senders/android/src/migration/nodes/source.rs`  
**Function:** `build_source_element()`  
**Lines:** ~80-105

**Status:** Already has fallback logic - needs hardening.

**Tasks:**
- [ ] Verify `uridecodebin` fallback path works end-to-end on Android (dynamic pad linking with `connect_pad_added`).
- [ ] When using `uridecodebin` fallback, implement manual reconnection logic on pipeline bus errors (the app loses automatic recovery that `fallbacksrc` provides).
- [ ] Add a periodic health-check in `poll_bus_messages()` that detects stream stalls and triggers pipeline rebuild.
- [ ] Consider `uridecodebin3` as a middle-ground fallback (better gap handling than `uridecodebin`, available in prebuilt).
- [ ] Audit the fallbacksrc-only property sets (`manual_unblock`, `immediate_fallback`) and keep them strictly inside the fallbacksrc code path. Current code already scopes them to fallbacksrc, so this is a regression guard.

---

## P1 - High (Functional gaps, will hit in production)

### 4. Add `videoconvert` color space negotiation before hardware encoders

**File:** `senders/android/src/migration/nodes/destination.rs`  
**All encoder-using arms:** RTMP, UDP, LocalFile

Android MediaCodec encoders are extremely strict about input color format. Desktop encoders accept nearly anything.

**Tasks:**
- [ ] Ensure `videoconvert` is always placed immediately before the encoder element.
- [ ] Add explicit capsfilter between `videoconvert` and encoder constraining to `video/x-raw,format=NV12` on Android.
- [ ] Handle devices where MediaCodec only accepts `COLOR_FormatYUV420Planar` (I420) - probe encoder sink pad caps.
- [ ] Test with frames coming from `appsrc` in I420 format (which is what the screen capture path produces).

```rust
// TODO: Add after videoconvert, before encoder
#[cfg(target_os = "android")]
{
    let caps = gst::Caps::builder("video/x-raw")
        .field("format", "NV12")
        .build();
    let capsfilter = Self::make_element("capsfilter", None)?;
    capsfilter.set_property("caps", &caps);
    // insert into link chain
}
```

---

### 5. Validate all GStreamer elements exist at startup

**File:** `senders/android/src/migration/runtime.rs`  
**Function:** `start_graph_runtime()`

Currently no validation that required elements are in the registry.

**Tasks:**
- [ ] Add a startup probe that checks for critical elements and logs availability:
  ```
  compositor yes  audiomixer yes  amcvidenc yes  rtmp2sink no  fallbacksrc no
  ```
- [ ] Expose element availability via `GET /health` endpoint so external tools can check.
- [ ] Fail fast with descriptive error if absolute minimum elements are missing (`appsrc`, `appsink`, `videoconvert`, `audioconvert`).

---

### 6. Register Rust-compiled GStreamer plugins via JNI

**File:** new file needed - e.g., `senders/android/src/migration/gst_registration.rs`

If any `gst-plugins-rs` elements are cross-compiled and statically linked, they bypass the automatic `GST_PLUGIN_STATIC_REGISTER` sequence in the prebuilt `libgstreamer_android.so`.

**Tasks:**
- [ ] Create explicit plugin registration calls for any statically linked Rust plugins.
- [ ] Call registration before `gst_init()` or immediately after, in `android_main`.
- [ ] Document the required `GST_PLUGIN_STATIC_DECLARE` / `GST_PLUGIN_STATIC_REGISTER` pattern for each Rust plugin added.
- [ ] Add build-time feature flags: `feature = "gst-rs-fallbacksrc"`, `feature = "gst-rs-rtmp2"`, etc.

---

## P2 - Medium (Robustness and quality)

### 7. Implement rank-based encoder/decoder discovery

**File:** `senders/android/src/migration/nodes/destination.rs`

**Tasks:**
- [ ] Replace hardcoded element name lists with `gst::ElementFactory` registry queries.
- [ ] Query by caps: `"video/x-h264"` on src pad for encoders.
- [ ] Sort by `gst::Rank` descending - Android will naturally surface `amcvidenc` variants with `Rank::Primary`.
- [ ] Cache discovery results per-session (element registry does not change at runtime).
- [ ] Log the full list of discovered encoders at startup for debugging device-specific issues.

```rust
fn discover_h264_encoder() -> Result<gst::Element, String> {
    let factories = gst::ElementFactory::list_get_elements(
        gst::ElementFactoryListType::ENCODER | gst::ElementFactoryListType::MEDIA_VIDEO,
        gst::Rank::MARGINAL,
    );
    for factory in factories {
        if factory.can_src_any_caps(&gst::Caps::builder("video/x-h264").build()) {
            if let Ok(elem) = factory.create().name("destination-venc").build() {
                return Ok(elem);
            }
        }
    }
    Err("No H.264 encoder available".into())
}
```

---

### 8. Handle `amcvidenc` bitrate and keyframe properties

**File:** `senders/android/src/migration/nodes/destination.rs`  
**Function:** `select_video_encoder()` and callers

**Tasks:**
- [ ] After selecting `amcvidenc`, probe for available properties before setting:
  - `bitrate` - some variants use bps, others kbps
  - `i-frame-interval` - Android equivalent of `key-int-max`
  - not `tune` or `zerolatency` - these are x264-specific and do not exist on `amcvidenc`
- [ ] Replace the current unconditional property sets:
  ```rust
  // CURRENT (will warn/fail on amcvidenc)
  if venc.has_property("tune") { venc.set_property_from_str("tune", "zerolatency"); }
  if venc.has_property("key-int-max") { venc.set_property("key-int-max", 30u32); }

  // TODO: Add amcvidenc-specific path
  if venc.has_property("i-frame-interval") { venc.set_property("i-frame-interval", 1i32); }
  ```

---

### 9. Handle `timecodestamper` / `timeoverlay` availability

**File:** `senders/android/src/migration/nodes/destination.rs`  
**RTMP arm**

**Tasks:**
- [ ] Make `timecodestamper` and `timeoverlay` optional in the RTMP chain - skip them if unavailable.
- [ ] These elements are in `gst-plugins-bad` which is in the prebuilt, but verify they are in the Android subset.
- [ ] If unavailable, link directly: `appsrc -> videoconvert -> encoder` (skip timecode insertion).

---

### 10. Add stream reconnection for `uridecodebin` fallback path

**File:** `senders/android/src/migration/nodes/source.rs`  
**Function:** `poll_bus_messages()`

When `fallbacksrc` is unavailable and `uridecodebin` is used, network drops kill the pipeline permanently.

**Tasks:**
- [ ] On `gst::MessageType::Error` from `uridecodebin`, check if error is network-related.
- [ ] Implement automatic teardown + rebuild with exponential backoff.
- [ ] Add configurable `max_reconnect_attempts` to `SourcePipelineProfile`.
- [ ] Emit `NodeStatusMessage::Error` to UI/listeners before reconnect attempt.
- [ ] Track consecutive failures and transition to `State::Stopped` after exhausting retries.

---

### 11. Audit `deinterlace` element usage

**Files:** `source.rs`, `video_generator.rs`

**Tasks:**
- [ ] Verify `deinterlace` is in prebuilt Android SDK (it is in `gst-plugins-good`, should be present).
- [ ] On Android, most content is progressive - `deinterlace` adds unnecessary latency.
- [ ] Make `deinterlace` conditional: skip it on Android unless input is actually interlaced.
- [ ] Probe input caps for `interlace-mode` before inserting deinterlace element.

---

## P3 - Low (Optimization and future-proofing)

### 12. Evaluate WebRTC/WHIP as RTMP replacement

**File:** `senders/android/src/migration/nodes/destination.rs`

**Tasks:**
- [ ] Add `DestinationFamily::Whip { endpoint: String }` variant to protocol.
- [ ] Implement destination pipeline using `whipclientsink` (from `gst-plugins-rs`).
- [ ] WHIP over UDP avoids TCP head-of-line blocking on mobile networks.
- [ ] If `gst-plugins-rs` cross-compilation is set up for `rtmp2sink`, `whipclientsink` comes for free.
- [ ] Consider making WHIP the default output protocol for mobile.

---

### 13. Implement zero-copy video frame path

**File:** `senders/android/src/lib.rs`  
**Function:** `process_frame()`

**Tasks:**
- [ ] Current path copies Y/U/V planes row-by-row from Java `ByteBuffer` into GStreamer buffer pool.
- [ ] Investigate `GstGLMemory` + `VideoFrameGLExt` to map Android `HardwareBuffer` directly to GL textures.
- [ ] This eliminates CPU-side pixel copies entirely, reducing thermal throttling.
- [ ] Requires changes on Java side: use `ImageReader` with `USAGE_GPU_SAMPLED_IMAGE` flag.
- [ ] Only viable if downstream consumers (encoder) support GL input - `amcvidenc` does on most devices.

---

### 14. Minimize APK size from static Rust linking

**Tasks:**
- [ ] Enable LTO (`lto = true`) in release profile for `android-sender`.
- [ ] Add `strip = "symbols"` to Cargo release profile.
- [ ] Audit `codegen-units = 1` for maximum optimization.
- [ ] If cross-compiling `gst-plugins-rs`, build only the specific crates needed (do not pull entire workspace).
- [ ] Measure `.so` size delta before/after adding each Rust plugin.

---

### 15. Add `#[cfg(target_os = "android")]` gating throughout

**Files:** all node files

**Tasks:**
- [ ] Gate `amcvidenc` encoder paths with `#[cfg(target_os = "android")]`.
- [ ] Gate `rtmpsink` legacy fallback with `#[cfg(target_os = "android")]`.
- [ ] Keep desktop encoder chain (`x264enc`, etc.) for `#[cfg(not(target_os = "android"))]`.
- [ ] Ensure `cargo check` and `cargo test` still pass on host (x86_64 Linux) without Android SDK.

---

### 16. Set up cross-compilation pipeline for `gst-plugins-rs`

**Tasks:**
- [ ] Document Cerbero-based build for Android `aarch64-linux-android` target.
- [ ] Set up `PKG_CONFIG_SYSROOT_DIR` and `PKG_CONFIG_PATH` pointing to NDK/Cerbero sysroot.
- [ ] Use `cargo-c` to produce `.a` static libraries for: `fallbackswitch`, `rsrtmp`, `rswebrtc`.
- [ ] Integrate static libs into `CMakeLists.txt` alongside prebuilt GStreamer.
- [ ] Add CI job that cross-compiles and verifies linkage.
- [ ] Pin specific `gst-plugins-rs` commit/tag matching the prebuilt SDK version (e.g., 1.24.x).

---

### 17. Add element availability integration tests

**File:** new - `senders/android/tests/gst_elements.rs` or extend existing tests

**Tasks:**
- [ ] Write test that initializes GStreamer and probes for every element used across all node types.
- [ ] Categorize results: `available`, `missing-optional`, `missing-critical`.
- [ ] Run on both host (full desktop GStreamer) and Android (prebuilt SDK) via `adb shell`.
- [ ] Elements to probe:
  ```
  appsrc, appsink, compositor, audiomixer, videotestsrc, audiotestsrc,
  videoconvert, audioconvert, audioresample, deinterlace, level,
  timecodestamper, timeoverlay, queue, capsfilter,
  uridecodebin, autovideosink, autoaudiosink,
  flvmux, mpegtsmux, splitmuxsink, udpsink,
  avenc_aac, h264parse, multiqueue,
  fallbacksrc, rtmp2sink, rtmpsink,
  x264enc, openh264enc, nvh264enc,
  amcvidenc-*
  ```

---

## Summary Matrix

| ID | Priority | Component | Status |
|----|----------|-----------|--------|
| 1 | P0 | H.264 encoder selection | missing - all 3 encoders missing on Android |
| 2 | P0 | `rtmp2sink` fallback | missing - RTMP destinations broken |
| 3 | P0 | `fallbacksrc` fallback | partial - code exists but needs hardening |
| 4 | P1 | Color space for `amcvidenc` | missing |
| 5 | P1 | Startup element validation | missing |
| 6 | P1 | Rust plugin JNI registration | missing |
| 7 | P2 | Rank-based discovery | missing |
| 8 | P2 | `amcvidenc` property handling | missing |
| 9 | P2 | Optional timecode elements | partial - needs verification |
| 10 | P2 | Stream reconnection | missing |
| 11 | P2 | Deinterlace optimization | partial - works but wasteful |
| 12 | P3 | WHIP destination | missing |
| 13 | P3 | Zero-copy frames | missing |
| 14 | P3 | APK size optimization | missing |
| 15 | P3 | cfg gating | partial |
| 16 | P3 | Cross-compilation CI | missing |
| 17 | P3 | Element availability tests | missing |
