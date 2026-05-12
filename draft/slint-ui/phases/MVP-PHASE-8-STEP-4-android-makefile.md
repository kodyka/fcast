# MVP-PHASE-8 — Step 4: bundle the SRT plugin in the Android build

> Part 4 of 6. Parent doc: [`MVP-PHASE-8-srt-destination-family.md`](./MVP-PHASE-8-srt-destination-family.md).
> Previous: [Step 3 — wire the `Srt` arm in `build_live_pipeline`](./MVP-PHASE-8-STEP-3-build-live-pipeline.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

Add `srt` to the sender's `GSTREAMER_PLUGINS` list in
`senders/android/app/jni/Android.mk` so `srtsink` (and `srtsrc`)
become discoverable at runtime.

This step is **mandatory** for any on-device test of PHASE-8: without
it, [Step 3](./MVP-PHASE-8-STEP-3-build-live-pipeline.md)'s
`Self::make_element("srtsink", None)?` call fails at runtime with:

```
ERROR: Could not create element of type srtsink. Plugin missing?
```

…and the destination transitions to `Stopped` with
`last_error: Some("…")`.

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| Sender's `GSTREAMER_PLUGINS` list | `senders/android/app/jni/Android.mk:32-66` |
| Receiver's `GSTREAMER_PLUGINS_NET_NO_RSWEBRTC` list (already includes `srt`) | `receivers/experimental/android/app/jni/Android.mk:34` |

### 1.2 What's in the sender list today

```makefile
GSTREAMER_PLUGINS := \
    coreelements \
    app \
    audioconvert \
    audiomixer \
    /* … many entries … */
    tcp \
    rtsp \
    rtp \
    rtpmanager \
    udp \
    dtls \
    srtp \
    webrtc \
    nice \
    rsrtp \
    rsrtsp \
    rswebrtc
```

`srt` is **not** in this list. Compare to the receiver
(`receivers/experimental/android/app/jni/Android.mk:34`):

```makefile
GSTREAMER_PLUGINS_NET_NO_RSWEBRTC := tcp rtsp rtp rtpmanager udp dtls \
    rist rtpmanagerbad rtponvif sctp sdpelem srtp srt webrtc nice \
    mpegtslive rsonvif raptorq rsrtp rsrtsp
```

The receiver bundles `srt`; the sender doesn't. This step adds the
one missing line.

### 1.3 Why this works without rebuilding the prebuilt GStreamer SDK

The `srt` plugin lives in `gst-plugins-bad` and is conditional on
`libsrt` being available at the prebuilt SDK's build time. The
prebuilt GStreamer Android SDK that the sender consumes ships with
`libsrt.so` (confirmed by the receiver's `Android.mk:34` referencing
the plugin under the same prebuilt path), so the only change needed
is *selecting* the plugin in the sender's plugin list — no rebuild
of the SDK itself.

If `libsrt` is **not** in the prebuilt SDK on a target ABI, the NDK
link step fails with `undefined reference to srt_*` symbols. That
would require rebuilding the SDK with `libsrt`; out of scope for
this phase. **Verify before promising end users SRT support.**

---

## 2. The change

**File:** `senders/android/app/jni/Android.mk` (extend
`GSTREAMER_PLUGINS` at lines 32-66, just before `webrtc`):

```makefile
# senders/android/app/jni/Android.mk (excerpt)

GSTREAMER_PLUGINS := \
    coreelements \
    app \
    audioconvert \
    audiomixer \
    /* … existing entries … */
    rtp \
    rtpmanager \
    udp \
    dtls \
    srtp \
    srt \                  # ← NEW
    webrtc \
    nice \
    rsrtp \
    rsrtsp \
    rswebrtc
```

**Exactly one line added**, sandwiched between `srtp` and `webrtc` to
group the secure-transport plugins together (matching the receiver's
ordering convention).

The plugin name is `srt` — **not** `gstsrt`, `gst-srt`, or `srtsrc`.
The plugin ships both `srtsrc` and `srtsink`, and is automatically
registered via `GST_PLUGIN_STATIC_REGISTER` in the prebuilt
`libgstreamer_android.so` startup sequence — no additional Rust
registration call is needed.

---

## 3. Verification

### 3.1 NDK build

```bash
cd senders/android
./gradlew :app:assembleDebug --rerun-tasks
```

Expect **success**. If the NDK link step fails with:

```
undefined reference to `srt_socket`
undefined reference to `srt_listen`
...
```

…then the prebuilt GStreamer Android SDK on this NDK host does **not**
include `libsrt.so` for the current ABI. Mitigations:

1. Switch to a SDK build that includes `libsrt` (confirm with the
   release notes of the GStreamer Android binaries).
2. Rebuild `gst-plugins-bad` with `--with-srt` enabled and `libsrt`
   in the link path. **Out of scope for PHASE-8.**

### 3.2 Plugin presence at runtime

```bash
adb shell am force-stop org.fcast.android.sender
adb shell am start -n org.fcast.android.sender/.MainActivity

# Inspect the registered factories.
adb logcat | grep -E 'srtsink|srtsrc|GST_REGISTRY'
```

Expected output (paraphrased — exact phrasing depends on GStreamer
version):

```
GST_REGISTRY ... gst-plugin-scanner ... loaded plugin srt
GST_REGISTRY ... registered: srtsrc, srtsink
```

If the lines are absent, the gradle build cached the previous plugin
list. Force-rebuild:

```bash
./gradlew clean :app:assembleDebug
```

### 3.3 Programmatic confirmation (one-shot, in app startup)

Drop this snippet into `senders/android/src/lib.rs` after the
`gst::init()` call, **temporarily**:

```rust
// TEMPORARY — remove after PHASE-8 §3.3 confirms.
log::info!(
    "srtsink factory: {:?}",
    gst::ElementFactory::find("srtsink").map(|_| "found")
);
log::info!(
    "srtsrc factory: {:?}",
    gst::ElementFactory::find("srtsrc").map(|_| "found")
);
```

Expect both lines to log `Some("found")` after launching the app. If
either is `None`, Step 4 didn't take effect. **Remove the logs after
confirming.**

### 3.4 Plugin file inspection

```bash
adb shell run-as org.fcast.android.sender \
    ls /data/data/org.fcast.android.sender/lib | grep -E 'srt|gstsrt'
# → expected: libsrt.so (and possibly libgstsrt.so as a separate plugin .so)
```

Depending on how the SDK ships its plugins, `srt` may be statically
linked into `libgstreamer_android.so` rather than a separate `.so`.
**Both layouts are fine** — the runtime
`gst_registry_get_default()` will pick it up either way. The key test
is §3.3 — if `ElementFactory::find("srtsink")` returns `Some(...)`,
the plugin is correctly loaded regardless of whether it's a separate
`.so` file.

### 3.5 Grep recipe

```bash
grep -nE '^\s+srt\b' senders/android/app/jni/Android.mk
# → expect: exactly one match (the new line added in Step 4).
```

The `^\s+` matches the leading indentation; `\b` keeps `srt` distinct
from `srtp` (also in the list, two lines above).

---

## 4. Pitfalls specific to this step

### S4-P1 — Misspelling the plugin name

The plugin name is `srt`, not:

| Wrong | Source of confusion |
|---|---|
| `gstsrt` | The `.so` file is named `libgstsrt.so` — the prefix is added by gst-plugin-scanner. The Makefile uses the bare name. |
| `gst-srt` | The pkg-config file is `gstreamer-1.0` (singular); plugin names in the Makefile don't use hyphens. |
| `srtsrc` / `srtsink` | These are **element** names registered by the plugin, not the plugin name itself. |

If you write the wrong name, gradle silently builds without complaint
but the runtime registry doesn't find `srtsink`. Symptom: clean APK,
runtime "Plugin missing?" error.

### S4-P2 — Forgetting the line-continuation `\` on the previous line

```makefile
    srtp \    ← keep this backslash!
    srt \
    webrtc \
```

If `srtp` lacks the `\`, the Makefile parser treats `srt` as the
beginning of a new variable assignment — wrong. The build fails with
`*** missing separator. Stop.` or, worse, silently truncates the
plugin list at `srtp`. **Always verify the backslash on the line
before your insertion.**

### S4-P3 — Inserting `srt` in the wrong group

The plugin list is grouped by GStreamer category (audio, video,
network, codec, …). The receiver groups `srt` next to `srtp` /
`webrtc` / `nice` in the "secure / encrypted transport" cluster.
Inserting `srt` mid-list (e.g. between `coreelements` and `app`)
works functionally but obscures intent. **Match the receiver's
grouping** — it's the canonical reference for plugin ordering.

### S4-P4 — Plugin name reservations

GStreamer reserves `srt` as the plugin name for the SRT protocol
support (Secure Reliable Transport). It is **not** the same as
`subparse`'s SRT subtitle support, which lives inside the `subparse`
plugin. If a future contributor adds a generic `srt` subtitle filter,
the plugin-name namespace collision must be resolved at the
`gst-plugins-base` level — beyond PHASE-8 scope.

### S4-P5 — Stripping the plugin in a release build

If `senders/android/app/build.gradle` defines a release flavor that
strips unused plugins by listing a subset of `GSTREAMER_PLUGINS`,
verify the SRT plugin is also in that release-time list. Build-time
plugin lists are **separate** from the registry's runtime contents.
For PHASE-8 we only target debug builds; flag release-build coverage
as a follow-up.

---

## 5. Next step

After this lands, [STEP 5 — unit tests](./MVP-PHASE-8-STEP-5-unit-tests.md)
adds the protocol-level and dispatch-level unit tests that validate
all of Steps 1–4 together. These tests don't require GStreamer to
be initialised or the SRT plugin to be available; they validate the
JSON shape and `NodeManager` dispatch path only.
