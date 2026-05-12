# MVP-PHASE-5 — Step 2: extend `DestinationPipelineProfile::from_family`

> Part 2 of 7. Parent doc: [`MVP-PHASE-5-whep-destination-family.md`](./MVP-PHASE-5-whep-destination-family.md).
> Previous: [Step 1 — extend the JSON protocol](./MVP-PHASE-5-STEP-1-protocol-extension.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

After [Step 1](./MVP-PHASE-5-STEP-1-protocol-extension.md) added
`DestinationFamily::Whep { server_port }`, every `match` over
`DestinationFamily` is non-exhaustive. This step adds the
**diagnostic element listing** arm to
`DestinationPipelineProfile::from_family` — the function that
populates the human-readable "what GStreamer factories will this
pipeline use" hint surfaced via `getinfo`.

This is **not** the pipeline construction itself. That's
[Step 4](./MVP-PHASE-5-STEP-4-build-live-pipeline.md). The two
look similar but serve different roles:

| Concern | Owner |
|---|---|
| "What factories will be in the pipeline?" (for `getinfo` / debug) | `from_family` — **this step** |
| "Build the GStreamer pipeline at start time" | `build_live_pipeline` — Step 4 |

WHEP's element list is **shorter** than the other families because
`BaseWebRTCSink` is a Rust-constructed sink — there is no
`gst_element_factory_make("basewebrtcsink")` factory at the GStreamer
level. The element listing here is purely descriptive.

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| `DestinationPipelineProfile::from_family` | `senders/android/src/migration/nodes/destination.rs:35-105` |
| Existing arms (Rtmp / Udp / LocalFile / LocalPlayback) | same, lines 40-88 |
| Audio/video filter (the `retain` after the match) | same, lines 91-96 |

### 1.2 Why the WHEP element list is just `videoconvert + basewebrtcsink`

The actual WHEP pipeline in
[Step 4](./MVP-PHASE-5-STEP-4-build-live-pipeline.md) is just:

```
appsrc → videoconvert → BaseWebRTCSink
```

`BaseWebRTCSink` handles the entire WebRTC transport stack internally
(SRTP, ICE, encoding, signaller, packet pacing). There is no separate
`h264enc` / `mpegtsmux` / network-sink chain — the sink **is** the
encoder + transport.

The `"basewebrtcsink"` string in the element list is **illustrative
only**. There is no factory by that name registered with the
GStreamer registry. The real sink is constructed via the
`gst_rs_webrtc::webrtcsink::BaseWebRTCSink::with_signaller` Rust
constructor (see Step 4).

### 1.3 Why no audio chain

The legacy cast loop in
`sdk/mirroring_core/src/transmission.rs:475-528` is **video-only**
on Android. Matching that, the WHEP destination here is video-only.
Audio support is a follow-up — explicitly out of scope for
PHASE-5.

The `audio` flag must still be **accepted** (so an `audio: true`
graph command doesn't fail validation), but it has no effect on the
emitted element list. Use `let _ = audio;` to silence the
unused-variable lint.

---

## 2. The change

**File:** `senders/android/src/migration/nodes/destination.rs`
(extend the `match family { … }` block at line 39):

```rust
impl DestinationPipelineProfile {
    fn from_family(family: &DestinationFamily, audio: bool, video: bool) -> Self {
        let mut elements = Vec::new();

        match family {
            DestinationFamily::Rtmp { .. } => { /* …existing… */ }
            DestinationFamily::Udp { .. } => { /* …existing… */ }
            DestinationFamily::LocalFile { .. } => { /* …existing… */ }
            DestinationFamily::LocalPlayback => { /* …existing… */ }

            // NEW —
            DestinationFamily::Whep { .. } => {
                elements.extend([
                    "videoconvert",
                    "basewebrtcsink", // illustrative — the real factory
                                      // is gst_rs_webrtc::webrtcsink::BaseWebRTCSink
                                      // constructed in Rust, not by name.
                ]);
                // WHEP currently sends video-only (matching the live
                // cast loop in transmission.rs:475-528). Audio support
                // is a follow-up; keep the `audio` flag honored but
                // emit no audio elements.
                let _ = audio;
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

Two element-name strings, one match arm.

The retention filters at lines 91-96 work unmodified — when
`video: false`, both `videoconvert` and `basewebrtcsink` survive
(neither contains `"video"`'s substring after `videoconvert` is
stripped) wait — `"videoconvert"` does contain `"video"`. The
filter strips it. That's fine: a `video: false` WHEP destination is
useless (it has nothing to send), and the resulting empty element
list correctly signals "this destination will do nothing".

If you want a `video: false` WHEP destination to be rejected at
dispatch time, add an explicit check in
`node_manager.rs::create_destination` — but that's out of scope here.

---

## 3. Verification

### 3.1 Compile check

```bash
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android
```

After Step 1 + Step 2, the compile error
`non-exhaustive patterns: 'DestinationFamily::Whep { ... }' not covered`
inside `from_family` disappears. The same error in
`build_live_pipeline` remains until Step 4 lands.

### 3.2 Diagnostic introspection

Drop these tests into the existing
`#[cfg(test)] mod tests` block in `nodes/destination.rs`:

```rust
#[test]
fn whep_profile_lists_basewebrtcsink() {
    let family = DestinationFamily::Whep { server_port: 0 };
    let profile = DestinationPipelineProfile::from_family(&family, false, true);
    assert!(profile.elements.iter().any(|el| el == "basewebrtcsink"));
    assert!(profile.elements.iter().any(|el| el == "videoconvert"));
}

#[test]
fn whep_profile_ignores_audio_flag() {
    // Even with audio=true, WHEP currently emits no audio elements
    // (matches mcore::transmission::WhepSink on Android).
    let family = DestinationFamily::Whep { server_port: 0 };
    let profile = DestinationPipelineProfile::from_family(&family, true, true);
    assert!(!profile.elements.iter().any(|el| el == "audioconvert"));
    assert!(!profile.elements.iter().any(|el| el == "avenc_aac"));
}

#[test]
fn whep_profile_empty_when_video_disabled() {
    let family = DestinationFamily::Whep { server_port: 0 };
    let profile = DestinationPipelineProfile::from_family(&family, false, false);
    // `videoconvert` is stripped (contains "video"); `basewebrtcsink`
    // doesn't contain "video" or "h264" so it remains. This is a
    // degenerate config — a WHEP destination with neither audio nor
    // video has nothing to send.
    assert_eq!(profile.elements.len(), 1);
    assert_eq!(profile.elements[0], "basewebrtcsink");
}
```

All three green.

### 3.3 Grep recipe

```bash
grep -nA6 'DestinationFamily::Whep { .. } =>' \
    senders/android/src/migration/nodes/destination.rs
# → expect: one match in `from_family` (this step) listing 2 factories.
#   Step 4 adds a second match in `build_live_pipeline` with the real
#   pipeline construction.
```

---

## 4. Pitfalls specific to this step

### S2-P1 — Listing `webrtcsink` instead of `basewebrtcsink`

`webrtcsink` and `basewebrtcsink` are different Rust types in
`gst_rs_webrtc::webrtcsink`. The migration runtime uses
**`BaseWebRTCSink`** (the lower-level base type, constructed with
`with_signaller`). `WebRTCSink` is a thin convenience wrapper that
the legacy `mcore::transmission::WhepSink` does NOT use.

The element-name string here is purely diagnostic, but using the
wrong name will confuse readers who try to look up the factory in
the GStreamer registry. **Use `"basewebrtcsink"`.**

### S2-P2 — Treating the string as a real factory name

```rust
let sink = Self::make_element("basewebrtcsink", None)?;
// ❌ This fails at runtime — there is no such factory.
```

`BaseWebRTCSink` is a Rust struct, **not** a registered factory. It
has no `gst_element_factory_make("basewebrtcsink")` accessor. The
real instantiation lives in
[Step 4](./MVP-PHASE-5-STEP-4-build-live-pipeline.md) §2:

```rust
let sink = gst_rs_webrtc::webrtcsink::BaseWebRTCSink::with_signaller(
    gst_rs_webrtc::signaller::Signallable::from(signaller),
);
```

Anyone who refactors `from_family` into a "make all listed elements"
helper will hit this wall. Keep the list **descriptive only**.

### S2-P3 — Adding `videotestsrc` or `appsrc` to the list

These are **upstream** elements (the producer side of the
appsink/appsrc bridge), not part of the destination pipeline. They
don't appear in the other destination arms' lists either. **Keep
the WHEP list to the destination-side elements only.**

### S2-P4 — Forgetting `let _ = audio;`

```rust
DestinationFamily::Whep { .. } => {
    elements.extend(["videoconvert", "basewebrtcsink"]);
    // `audio` is now unused — triggers a compiler warning.
}
```

`from_family` takes `audio: bool` and `video: bool` parameters that
the other arms use. The `let _ = audio;` line **explicitly silences**
the unused-variable lint without `#[allow(unused_variables)]`,
documenting that the omission is intentional. (The `video` flag is
consumed by the `retain` filter at line 91 — no `let _ = video;`
needed.)

---

## 5. Next step

After this lands, [Step 3 — add bound-port fields on `DestinationNode`](./MVP-PHASE-5-STEP-3-destination-node-fields.md)
extends the `DestinationNode` struct with the `whep_bound_port_v4`
and `whep_bound_port_v6` fields that `DestinationInfo`'s new fields
(added in Step 1) will read from.
