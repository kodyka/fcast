# MVP-PHASE-8 — Step 2: extend `DestinationPipelineProfile::from_family`

> Part 2 of 6. Parent doc: [`MVP-PHASE-8-srt-destination-family.md`](./MVP-PHASE-8-srt-destination-family.md).
> Previous: [Step 1 — extend the JSON protocol](./MVP-PHASE-8-STEP-1-protocol-extension.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

After [Step 1](./MVP-PHASE-8-STEP-1-protocol-extension.md) added
`DestinationFamily::Srt { … }`, every `match` over `DestinationFamily`
is non-exhaustive. This step adds the **diagnostic element listing**
arm to `DestinationPipelineProfile::from_family` — the function that
populates the human-readable "what GStreamer factories will this
pipeline use" hint surfaced via `getinfo`.

This is **not** the pipeline construction itself. That's
[Step 3](./MVP-PHASE-8-STEP-3-build-live-pipeline.md). The two
look similar but serve different roles:

| Concern | Owner |
|---|---|
| "What factories will be in the pipeline?" (for `getinfo` / debug) | `from_family` — **this step** |
| "Build the GStreamer pipeline at start time" | `build_live_pipeline` — Step 3 |

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| `DestinationPipelineProfile::from_family` | `senders/android/src/migration/nodes/destination.rs:35-105` |
| Existing arms (Rtmp / Udp / LocalFile / LocalPlayback) | same, lines 40-88 |
| Audio/video filter (the `retain` after the match) | same, lines 91-96 |

### 1.2 Why the `Srt` arm copies the `Udp` element list

Both pipelines mux to MPEG-TS via `mpegtsmux` and encode video via
`h264enc` / `h264parse` / audio via `avenc_aac`. The **only**
factory-level difference is the network sink: `srtsink` vs
`udpsink`. **Step 3** wires those into a real `gst::Pipeline`; **this
step** just records them as element-name strings.

The retention filters at lines 91-96:

```rust
if !audio { elements.retain(|el| !el.contains("audio")); }
if !video { elements.retain(|el| !el.contains("video") && !el.contains("h264")); }
```

…work unmodified because the new Srt element list re-uses the same
factory names (`videoconvert`, `h264enc`, `h264parse`, `audioconvert`,
`audioresample`, `avenc_aac`) — the substring matches the same as
they do for `Udp`.

---

## 2. The change

**File:** `senders/android/src/migration/nodes/destination.rs`
(extend the `match family { … }` block at line 39):

```rust
impl DestinationPipelineProfile {
    fn from_family(family: &DestinationFamily, audio: bool, video: bool) -> Self {
        let mut elements = Vec::new();

        match family {
            DestinationFamily::Rtmp { .. } => {
                elements.extend([
                    "flvmux", "queue", "rtmp2sink",
                    "videoconvert", "timecodestamper", "timeoverlay",
                    "h264enc", "h264parse",
                    "audioconvert", "audioresample", "avenc_aac",
                ]);
            }
            DestinationFamily::Udp { .. } => {
                elements.extend([
                    "mpegtsmux", "udpsink",
                    "videoconvert", "h264enc", "h264parse",
                    "audioconvert", "audioresample", "avenc_aac",
                ]);
            }
            DestinationFamily::LocalFile { .. } => {
                elements.extend([
                    "splitmuxsink", "multiqueue",
                    "videoconvert", "h264enc", "h264parse",
                    "audioconvert", "audioresample", "avenc_aac",
                ]);
            }
            DestinationFamily::LocalPlayback => {
                elements.extend([
                    "autovideosink", "autoaudiosink",
                    "videoconvert",
                    "audioconvert", "audioresample",
                    "queue",
                ]);
            }

            // NEW —
            DestinationFamily::Srt { .. } => {
                elements.extend([
                    "mpegtsmux",
                    "srtsink",
                    "videoconvert",
                    "h264enc",
                    "h264parse",
                    "audioconvert",
                    "audioresample",
                    "avenc_aac",
                ]);
            }
        }

        if !audio {
            elements.retain(|el| !el.contains("audio"));
        }
        if !video {
            elements.retain(|el| !el.contains("video") && !el.contains("h264"));
        }

        Self {
            family: family.clone(),
            elements: elements.into_iter().map(str::to_string).collect(),
            wait_for_eos_on_stop: true,
            stage: DestinationPipelineStage::Idle,
        }
    }
}
```

That's the entire diff for this step. **8 element-name strings, one
match arm.**

---

## 3. Verification

### 3.1 Compile check

```bash
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android
```

After this step **plus** Step 1, the compile error
`non-exhaustive patterns: 'DestinationFamily::Srt { ... }' not covered`
that Step 1 introduced inside `from_family` disappears. The same error
in `build_live_pipeline` remains until Step 3 lands.

### 3.2 Diagnostic introspection

Walk the `DestinationPipelineProfile` and confirm the element list
shape. Drop this test into the existing
`#[cfg(test)] mod tests` block in `nodes/destination.rs`:

```rust
#[test]
fn srt_profile_lists_srtsink_and_mpegtsmux() {
    let family = DestinationFamily::Srt {
        uri: "srt://example.com:1234".into(),
        latency: Some(200),
        passphrase: None,
        pbkeylen: None,
    };
    let profile = DestinationPipelineProfile::from_family(&family, true, true);
    assert!(profile.elements.iter().any(|el| el == "srtsink"));
    assert!(profile.elements.iter().any(|el| el == "mpegtsmux"));
    assert!(profile.elements.iter().any(|el| el == "h264enc"));
    assert!(profile.elements.iter().any(|el| el == "avenc_aac"));
}

#[test]
fn srt_profile_filters_audio_when_disabled() {
    let family = DestinationFamily::Srt {
        uri: "srt://example.com:1234".into(),
        latency: None,
        passphrase: None,
        pbkeylen: None,
    };
    let profile = DestinationPipelineProfile::from_family(&family, false, true);
    // Audio-related factories are stripped.
    assert!(!profile.elements.iter().any(|el| el == "audioconvert"));
    assert!(!profile.elements.iter().any(|el| el == "audioresample"));
    assert!(!profile.elements.iter().any(|el| el == "avenc_aac"));
    // Video-related factories remain.
    assert!(profile.elements.iter().any(|el| el == "h264enc"));
}

#[test]
fn srt_profile_filters_video_when_disabled() {
    let family = DestinationFamily::Srt {
        uri: "srt://example.com:1234".into(),
        latency: None,
        passphrase: None,
        pbkeylen: None,
    };
    let profile = DestinationPipelineProfile::from_family(&family, true, false);
    // Video-related factories are stripped (h264enc / videoconvert / h264parse).
    assert!(!profile.elements.iter().any(|el| el.contains("video")));
    assert!(!profile.elements.iter().any(|el| el.contains("h264")));
    // mpegtsmux and srtsink remain (neither contains "video" or "h264").
    assert!(profile.elements.iter().any(|el| el == "mpegtsmux"));
    assert!(profile.elements.iter().any(|el| el == "srtsink"));
}
```

All three green.

### 3.3 Grep recipe

```bash
grep -nA8 'DestinationFamily::Srt { .. } =>' \
    senders/android/src/migration/nodes/destination.rs
# → expect: one match in `from_family` (this step) listing 8 factories.
#   Step 3 adds a second match in `build_live_pipeline` with the real
#   pipeline construction.
```

---

## 4. Pitfalls specific to this step

### S2-P1 — Listing `srtsrc` instead of `srtsink`

`srtsrc` is a source element (used only via `uridecodebin` when the
source URI starts with `srt://`). The destination side **always**
uses `srtsink`. Listing `srtsrc` here will mislead `getinfo` consumers
into thinking the destination pipeline contains source elements,
which is nonsensical.

### S2-P2 — Adding a `queue` to the list

Some destination arms include `queue` (e.g. `LocalPlayback` line 87,
`Rtmp` line 43). The `Srt` arm doesn't need one in the diagnostic
listing — the actual pipeline in **Step 3** doesn't insert an
explicit `queue` element (`mpegtsmux` has its own input queueing).
Adding `queue` here would falsely imply the pipeline does
explicit buffering at this layer.

### S2-P3 — Forgetting `videoconvert` / `audioconvert`

The retention filters at lines 91-96 strip on substring match. If
you list `"x264enc"` (without `videoconvert` ahead of it) and the
user disables video, the `h264enc` is stripped but the listing still
shows `mpegtsmux + srtsink + audio*` — which is correct. **But:** if
the caller flips video back on later, no `videoconvert` appears in
the listing. The mismatch is purely cosmetic (Step 3 builds the
correct pipeline regardless), but it makes `getinfo` output
misleading. **Keep `videoconvert` in the list.**

### S2-P4 — Ordering of the strings

The order is purely for human readability — `from_family` doesn't
build a `gst::Pipeline`, so the strings are not topologically
significant. The convention in the existing arms is: muxer + sink
first, then video chain, then audio chain. **Follow that
convention** so a reader scanning `getinfo` output sees a consistent
pattern across families.

---

## 5. Next step

After this lands, [STEP 3 — wire the `Srt` arm in `build_live_pipeline`](./MVP-PHASE-8-STEP-3-build-live-pipeline.md)
adds the real GStreamer pipeline construction (the work that
`from_family` only describes diagnostically here). Step 3 is the
largest step in PHASE-8 — roughly 90 lines of Rust.
