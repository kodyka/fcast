# MVP-PHASE-8 — `Srt` destination family (Optional, Tier 1.4)

> **Optional architectural extension.** This phase adds an `Srt`
> variant to `DestinationFamily` so the migration runtime can push
> MPEG-TS-over-SRT to a remote receiver (or a media server like SRT
> Live Server / Haivision SRT Gateway). The construction mirrors the
> existing `Udp` branch in `nodes/destination.rs::build_live_pipeline`
> almost line-for-line — both use `mpegtsmux`; the only differences
> are `srtsink` vs `udpsink` and the SRT-specific properties
> (`latency`, `passphrase`, `pbkeylen`, `mode`).
>
> **SRT as a source** (`uridecodebin` / `fallbacksrc` with an
> `srt://` URI) **already works** through `SourceNode` —
> no code change required. The work here is the destination side.

---

## 0. Goal

After this phase ships, the migration runtime accepts:

```json
{
  "createdestination": {
    "id": "srt-out-1",
    "family": {
      "Srt": {
        "uri": "srt://media-server.example.com:1234",
        "latency": 200,
        "passphrase": "secret-shared-passphrase",
        "pbkeylen": 16
      }
    },
    "audio": true,
    "video": true
  }
}
```

…and the runtime builds an `appsrc → videoconvert → h264enc → h264parse
→ mpegtsmux → srtsink` pipeline (plus the audio chain), pushing
MPEG-TS over SRT to the configured URI.

The `Srt` family is **architecturally identical to `Udp`**: both
mux to MPEG-TS and stream-out via a network sink element. The new
properties (`latency`, `passphrase`, `pbkeylen`) are SRT-specific
tuning that `udpsink` doesn't have.

This phase is **not** an MVP gate, **not** part of the Tier 1
unification (PHASES 4 → 5 → 6), and **not** required for the
Android cast loop. It's an opt-in protocol expansion for the
migration runtime — useful for live-streaming workflows, contribution
feeds to broadcast infrastructure, and any deployment where UDP is
too lossy and RTMP too high-latency.

---

## 1. Pre-flight

### 1.1 What already exists (do not re-create)

| Component | Location |
|---|---|
| `DestinationFamily` enum (Rtmp / Udp / LocalFile / LocalPlayback) | `senders/android/src/migration/protocol.rs:126-138` |
| `DestinationPipelineProfile::from_family` (element listing) | `senders/android/src/migration/nodes/destination.rs:35-105` |
| `DestinationNode::build_live_pipeline` (UDP branch — closest template) | `senders/android/src/migration/nodes/destination.rs:606-679` |
| `Self::select_video_encoder` (encoder fallback chain) | `senders/android/src/migration/nodes/destination.rs` (search for `fn select_video_encoder`) |
| `Self::add_video_encoder_chain` / `Self::link_video_encoder_chain` helpers | same file |
| MPEG-TS muxer (`mpegtsmux`) properties (`alignment = 7`) | `nodes/destination.rs:619-621` (in the UDP branch) |
| `SourceNode::build_live_pipeline` (`fallbacksrc` / `uridecodebin`) | `senders/android/src/migration/nodes/source.rs` (search for `fallbacksrc`) |

### 1.2 What `SourceNode` already supports

`uridecodebin` and `fallbacksrc` both call `gst::uri_handler_factory`
to resolve URI scheme → source element. GStreamer ships `srtsrc`
under `gst-plugins-bad`, which registers itself as the URI handler
for `srt://`. **There is no work in `SourceNode`** — pass an
`srt://` URI to an existing `CreateSource` command and the pad-added
dispatch flows transparently:

```json
{
  "createsource": {
    "id": "srt-in-1",
    "uri": "srt://0.0.0.0:9000?mode=listener",
    "audio": true,
    "video": true
  }
}
```

(Prerequisite: §1.4 — the SRT plugin must be in the build.)

### 1.3 What needs to change

| File | Edit | Diff |
|---|---|---|
| `senders/android/src/migration/protocol.rs` | Add `Srt { uri, latency, passphrase, pbkeylen }` to `DestinationFamily`. | ~10 lines |
| `senders/android/src/migration/nodes/destination.rs` | (1) Extend `DestinationPipelineProfile::from_family` with an `Srt` arm (element list). (2) Extend `build_live_pipeline` with an `Srt` match arm (mirror of `Udp` branch). | ~100 lines |
| `senders/android/app/jni/Android.mk` | Add `srt` to `GSTREAMER_PLUGINS` so `srtsink` is registered. | 1 line |
| `senders/android/src/migration/node_manager.rs` | **No new dispatch arm** — family-agnostic routing already works. New `Srt`-specific unit tests though. | ~30 lines of tests |

Approximate scope: **~150 lines of Rust across 2 edited files**,
plus 1 line in `Android.mk`.

### 1.4 The build-system prerequisite

Look at `senders/android/app/jni/Android.mk:32-66`:

```makefile
GSTREAMER_PLUGINS := \
    coreelements \
    app \
    audioconvert \
    /* … */
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

`srt` is **not** in this list. Compare to the **receiver**'s
`Android.mk` (`receivers/experimental/android/app/jni/Android.mk:34`):

```makefile
GSTREAMER_PLUGINS_NET_NO_RSWEBRTC := tcp rtsp rtp rtpmanager udp dtls \
    rist rtpmanagerbad rtponvif sctp sdpelem srtp srt webrtc nice \
    mpegtslive rsonvif raptorq rsrtp rsrtsp
```

The receiver bundles `srt`; the sender doesn't. Step 4 below adds it.

The `srt` plugin lives in `gst-plugins-bad` and is conditional on
`libsrt` being available at the prebuilt SDK's build time. The
prebuilt GStreamer Android SDK that the sender consumes ships with
`libsrt.so` (confirmed by the receiver's `Android.mk` referencing the
plugin under the same prebuilt path), so the only change needed is
*selecting* the plugin in the sender's plugin list — no rebuild of
the SDK itself.

If `libsrt` is **not** in the prebuilt SDK on a target ABI, the
NDK link step fails with `undefined reference to srt_*` symbols.
That would require rebuilding the SDK with `libsrt`; out of scope
for this phase. Verify before promising end users SRT support.

---

## 2. Steps — split into six per-step files

To keep each step skimmable and reviewable in isolation, the
implementation is split across six per-step `MVP-PHASE-8-STEP-N-*.md`
files. Each file follows the same smaller five-section template
(Goal-of-this-step / Pre-flight / The change / Verification /
Next step) and is self-contained — you don't need to flip back to
this parent doc while implementing a single step.

| # | File | Scope | Net diff |
|---|---|---|---|
| 1 | [`MVP-PHASE-8-STEP-1-protocol-extension.md`](./MVP-PHASE-8-STEP-1-protocol-extension.md) | Add `Srt { uri, latency, passphrase, pbkeylen }` to `DestinationFamily`. Backward-compatible wire format via `#[serde(default, skip_serializing_if = …)]` on the optional fields. | ~30 lines, 1 file (`protocol.rs`) |
| 2 | [`MVP-PHASE-8-STEP-2-pipeline-profile.md`](./MVP-PHASE-8-STEP-2-pipeline-profile.md) | Extend `DestinationPipelineProfile::from_family` with an `Srt` arm — diagnostic element listing for `getinfo`. | ~10 lines, 1 file (`nodes/destination.rs`) |
| 3 | [`MVP-PHASE-8-STEP-3-build-live-pipeline.md`](./MVP-PHASE-8-STEP-3-build-live-pipeline.md) | Wire the `Srt` arm into `DestinationNode::build_live_pipeline`. Mirror of the existing `Udp` branch — `appsrc → videoconvert → h264enc → h264parse → mpegtsmux → srtsink`. Largest step in PHASE-8. | ~90 lines, 1 file (`nodes/destination.rs`) |
| 4 | [`MVP-PHASE-8-STEP-4-android-makefile.md`](./MVP-PHASE-8-STEP-4-android-makefile.md) | Add `srt` to `GSTREAMER_PLUGINS` in `senders/android/app/jni/Android.mk`. **Mandatory** for any on-device test — without it, `srtsink` is missing at runtime. | 1 line, 1 file (`Android.mk`) |
| 5 | [`MVP-PHASE-8-STEP-5-unit-tests.md`](./MVP-PHASE-8-STEP-5-unit-tests.md) | ~12 host-runnable unit tests across `protocol.rs`, `node_manager.rs`, and `nodes/destination.rs`. No GStreamer initialisation required. | ~150 lines of tests across 3 files |
| 6 | [`MVP-PHASE-8-STEP-6-source-side.md`](./MVP-PHASE-8-STEP-6-source-side.md) | **Documentation step.** SRT sources already work via `uridecodebin` + Step 4's plugin registration. Adds one trivial dispatcher test and the anti-pattern call-outs. No `SourceNode` change. | 1 test (already in Step 5) |

### Recommended landing order

```
Step 1 ──► Step 2 ──► Step 3 ──┐
                                ├── single squash-commit (compile stays clean)
                                │
                                ▼
                              Step 4 (Android.mk — required before on-device smoke)
                                │
                                ▼
                              Step 5 (unit tests — green after Step 4)
                                │
                                ▼
                              Step 6 (docs + one test from Step 5)
```

**Steps 1+2+3** must land together — if any of them lands alone, the
remaining steps' arms are missing and the `match` blocks become
non-exhaustive. Step 1 in isolation will compile only with a
temporary `_` arm placeholder (see Step 1 §3.1) — but the cleanest
path is squashing 1+2+3 into one commit.

**Step 4** is independent of 1–3 and can land in either order
(separately, even). The smoke verification in Step 3 §3.3 requires
Step 4 to be in place.

**Steps 5+6** are test-only and can land after the runtime changes.

---

## 2b. Why the per-step split?

The original monolithic §2 block ran to ~340 lines with six
sub-steps interleaved. Splitting it gives:

- Per-step files small enough to review on a phone screen.
- Independent verification recipes per step (each step's §3
  describes only that step's compile/test/grep checks).
- Step-specific pitfalls without scrolling past unrelated content.
- Easy follow-up PRs: if a reviewer asks for changes on Step 3
  only, you edit one file.

The pattern mirrors the existing `PHASE-8-Section-*.md` split
that converted the monolithic `MVP-PHASE-8.md` (the prior
Phase-8 doc) into one Section per concern.

---

<!-- Per-step content moved to MVP-PHASE-8-STEP-N-*.md.
     The remainder of this file (§3 onward) covers cross-cutting
     concerns: verification recipes that span multiple steps, the
     pitfalls catalogue, stop conditions, and why-it-matters. -->

> **Looking for inline §2.1 — §2.6?** The per-step content has
> moved into the six `MVP-PHASE-8-STEP-N-*.md` files listed in the
> table above. Each STEP file is self-contained — Goal, Pre-flight,
> The change, Verification, and Pitfalls for that step alone.

---

## 3. Verification

### 3.1 Compile check

```bash
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android
```

Expect **clean**.

### 3.2 Unit tests

```bash
cargo +nightly test -p fcast-sender-android \
    migration::node_manager::tests::create_srt_destination_succeeds \
    migration::node_manager::tests::srt_destination_with_encryption_serdes_roundtrip \
    migration::node_manager::tests::srt_destination_optional_fields_omitted_in_minimal_json \
    migration::node_manager::tests::create_source_accepts_srt_uri
```

All four green.

### 3.3 Plugin presence

```bash
adb shell am force-stop org.fcast.android.sender
adb shell am start -n org.fcast.android.sender/.MainActivity

# Inspect the registered factories.
adb logcat | grep -E 'srtsink|srtsrc|GST_REGISTRY'
```

Expected: the GStreamer registry log mentions both `srtsink` and
`srtsrc` factories. If they're absent, Step 4 didn't take effect —
re-run `ndk-build` and re-install the APK.

Alternative confirmation from inside Rust (one-shot, in app startup):

```rust
let _ = gst::ElementFactory::find("srtsink")
    .expect("srtsink plugin not loaded — see MVP-PHASE-8 §3.3");
let _ = gst::ElementFactory::find("srtsrc")
    .expect("srtsrc plugin not loaded");
```

(Only useful during bring-up — remove after confirming.)

### 3.4 End-to-end smoke (destination)

Pre-reqs:
- MVP-PHASE-3 verified the migration runtime command server is
  reachable via `MIGRATION_COMMAND_BIND=127.0.0.1:8080` + `adb forward
  tcp:8080 tcp:8080`.
- A second host with `srt-live-transmit` (from
  [Haivision/srt](https://github.com/Haivision/srt)) or
  `gst-launch-1.0 srtsrc ! tsdemux ! ...`.

```bash
# 1. On a separate host, start an SRT listener accepting MPEG-TS.
gst-launch-1.0 -v \
    srtsrc uri="srt://0.0.0.0:1234?mode=listener" latency=200 \
    ! tsdemux name=demux \
    ! queue ! h264parse ! avdec_h264 ! videoconvert ! autovideosink \
    demux. \
    ! queue ! aacparse ! avdec_aac ! audioconvert ! autoaudiosink

# 2. Back on the phone (via adb forward), build the SRT destination
# graph in the migration runtime.

LISTENER_HOST=10.0.0.42  # IP of the laptop running srtsrc above
curl -X POST http://127.0.0.1:8080/command \
     -d '{"createvideogenerator":{"id":"gen-1"}}'
curl -X POST http://127.0.0.1:8080/command \
     -d "{\"createdestination\":{\"id\":\"srt-out\",\"family\":{\"Srt\":{\"uri\":\"srt://${LISTENER_HOST}:1234\",\"latency\":200}},\"audio\":false,\"video\":true}}"
curl -X POST http://127.0.0.1:8080/command \
     -d '{"connect":{"link_id":"L1","src_id":"gen-1","sink_id":"srt-out","audio":false,"video":true}}'
curl -X POST http://127.0.0.1:8080/command \
     -d '{"start":{"id":"srt-out"}}'
curl -X POST http://127.0.0.1:8080/command \
     -d '{"start":{"id":"gen-1"}}'
```

**Expected** within ~1s on the listener host: the GStreamer `autovideosink`
window opens and displays the ball-pattern test source the
`videogenerator` node produces.

### 3.5 End-to-end smoke (source)

```bash
# 1. On a separate host, push an SRT stream to the phone.
# Make sure the phone's listening IP is reachable.

PHONE_IP=$(adb shell ip route | awk '/wlan|rmnet/ {print $9; exit}')
gst-launch-1.0 -v \
    videotestsrc is-live=true ! videoconvert ! x264enc tune=zerolatency \
    ! h264parse ! mpegtsmux ! srtsink uri="srt://${PHONE_IP}:9000" latency=200

# 2. On the phone, create the SRT source and a local-playback destination.
curl -X POST http://127.0.0.1:8080/command \
     -d '{"createsource":{"id":"srt-in","uri":"srt://0.0.0.0:9000?mode=listener","audio":false,"video":true}}'
curl -X POST http://127.0.0.1:8080/command \
     -d '{"createdestination":{"id":"local","family":"LocalPlayback","audio":false,"video":true}}'
curl -X POST http://127.0.0.1:8080/command \
     -d '{"connect":{"link_id":"L2","src_id":"srt-in","sink_id":"local","audio":false,"video":true}}'
curl -X POST http://127.0.0.1:8080/command \
     -d '{"start":{"id":"local"}}'
curl -X POST http://127.0.0.1:8080/command \
     -d '{"start":{"id":"srt-in"}}'
```

**Expected:** the phone's casting overlay (or local playback surface,
depending on which `LocalPlayback` element is used) shows the ball
pattern within ~2s.

### 3.6 Encryption smoke

Repeat §3.4 with a passphrase:

```bash
# Listener requires the same passphrase + key length.
gst-launch-1.0 -v \
    srtsrc uri="srt://0.0.0.0:1234?mode=listener&passphrase=topsecret&pbkeylen=16" \
    ! tsdemux ! /* … */

# Phone-side:
curl -X POST http://127.0.0.1:8080/command -d '{
  "createdestination":{
    "id":"srt-enc",
    "family":{"Srt":{
      "uri":"srt://10.0.0.42:1234",
      "latency":200,
      "passphrase":"topsecret",
      "pbkeylen":16
    }},
    "audio":false, "video":true
  }
}'
```

**Expected:** stream flows. If the listener side's passphrase is
**different**, expect `srtsink` to log `SRT connection rejected:
unauthorized` and the destination to enter the `Stopped` state with
`last_error: Some("…")` visible via `getinfo`.

---

## 4. Common pitfalls

### P1 — `srtsink` not found at runtime

```
ERROR: Could not create element of type srtsink. Plugin missing?
```

Means Step 4 didn't take effect. Verify:

```bash
adb shell run-as org.fcast.android.sender \
    cat /data/data/org.fcast.android.sender/files/.gstreamer-1.0/registry.*.bin \
    | strings | grep -i srt
# → expect: srtsink, srtsrc entries
```

If empty, `ndk-build` cached the previous plugin list. Force-rebuild
with `ndk-build clean && ndk-build`.

### P2 — `srt://0.0.0.0` listener never connects

`srtsink` defaults to **caller** mode (it connects out). To accept
connections, append `?mode=listener` to the URI:

```rust
DestinationFamily::Srt {
    uri: "srt://0.0.0.0:1234?mode=listener".into(),
    /* … */
}
```

For `srtsrc` (source side) the same query-param applies. **Don't**
guess at the side that initiates — explicitly set `mode=caller` or
`mode=listener` on both ends.

### P3 — `passphrase` set, `pbkeylen` not set → silently unencrypted

`srtsink` requires **both** `passphrase` AND `pbkeylen` to enable
encryption. If you set only one, the other side's authentication
will reject the connection with no warning on the sender. The
sender-side log will look like a clean connect followed by an
abrupt close.

**Mitigation:** in the JSON validator (post-MVP), reject
`Srt { passphrase: Some(_), pbkeylen: None }` as malformed. For
this phase, document the gotcha here and let the test in §2.5
catch it.

### P4 — Latency mismatch between endpoints

SRT's latency is **end-to-end** and **both endpoints must agree
within ±50%**. If the sender sets `latency=200` but the receiver
sets `latency=2000`, SRT silently downgrades both to the larger
value, leading to unexpected end-to-end delay. **Recommend
documenting the convention** that both sides use the same value;
default to `200` ms (matching `gst-launch` defaults).

### P5 — `mpegtsmux` `alignment=7` is critical

The UDP arm sets `mux.set_property("alignment", 7i32)` at lines
619-621. The Srt arm **must** do the same. Without it, MPEG-TS
packets emitted by the muxer aren't aligned to 188-byte boundaries
that `srtsink` expects, and some receivers (notably FFmpeg) report
`continuity counter` errors on every packet.

### P6 — `srtsink` blocks `pipeline.set_state(Playing)` if no receiver

In `caller` mode, `srtsink` synchronously attempts to connect on
`PAUSED → PLAYING`. If the listener side isn't running, the
transition **blocks for up to `connect-timeout` (default 3000ms)**
and then fails. The `DestinationNode::refresh()` polls states with
a 100ms tick, so the symptom is the destination sitting in `Starting`
for 3s before either succeeding or transitioning to `Stopped` with
`last_error: Some("Could not connect to receiver")`.

**Mitigation:** for one-way contribution feeds where the listener
might come and go, set `mode=listener` on the sender side instead,
so it accepts inbound connections. This requires the receiver to
initiate the connection — flip the topology.

### P7 — `srt://` URI with IPv6 needs bracket escaping

```rust
DestinationFamily::Srt {
    uri: "srt://[fe80::1]:1234".into(),  // ✓ correct
    /* … */
}
```

Without the brackets, GStreamer's URI parser splits on the wrong
colon and reports `Invalid URI: srt://fe80::1:1234`. The same rule
applies to `udpsink` URI inputs — copy that behaviour.

### P8 — `srtsink`'s `latency` is **i32** milliseconds, not microseconds

```rust
sink.set_property("latency", *lat as i32);  // ✓ milliseconds
```

If you pass `i64` (treating it as nanoseconds, like GStreamer's
clock-time helpers), `srtsink` complains:

```
GLib-GObject-WARNING **: cannot set property 'latency' of type 'gint' from value of type 'gint64'
```

Stick to `i32`.

### P9 — `passphrase` length must be 10–79 characters

SRT spec requires the passphrase to be 10–79 ASCII characters. A
6-character passphrase is silently accepted by `srtsink` (no
property warning) but the handshake fails. Validate in the JSON
deserializer or document the constraint.

---

## 5. Stop conditions

The phase is "done" when:

1. `cargo check` is clean across all targets in
   `senders/android/Cargo.toml`.
2. All four unit tests in §3.2 / §2.6 pass.
3. `srtsink` and `srtsrc` are present in the runtime element registry
   (§3.3 confirms).
4. The destination smoke in §3.4 displays the ball pattern on the
   remote `gst-launch` listener within ~1s of `start`.
5. The source smoke in §3.5 displays the remote ball pattern on the
   phone within ~2s of `start`.
6. The encryption smoke in §3.6 succeeds with matching passphrases
   and fails (with `last_error`) on mismatched passphrases.
7. New surface area is visible to:

```bash
grep -n 'DestinationFamily::Srt' \
    senders/android/src/migration/
# → expect: protocol.rs, nodes/destination.rs
```

8. The Android plugin list now bundles `srt`:

```bash
grep -nE '^\s*srt\b' senders/android/app/jni/Android.mk
# → expect: one line in GSTREAMER_PLUGINS
```

---

## 6. Why this matters

SRT is the **standard** transport for live-video contribution feeds
in broadcast and streaming infrastructure: low latency (sub-second
end-to-end), built-in encryption (AES-128/192/256), packet loss
recovery via ARQ (better than RTP/RTCP), and NAT-friendly listener
mode.

Adding it as a `DestinationFamily` variant lets the migration
runtime push from:

| Source | Sink |
|---|---|
| `ScreenCapture` (MVP-PHASE-4) | `Srt` (this phase) |
| `Source(uri)` (existing) | `Srt` (this phase) |
| `VideoGenerator` (existing) | `Srt` (this phase) |
| `Mixer` (existing) | `Srt` (this phase) |

…and pull from:

| Source | Sink |
|---|---|
| `Source(srt://…)` (already works) | any |

…opening up workflows like:

- **Mobile contribution feed**: phone screen → SRT → broadcast
  truck → on-air. Replaces RTMP-over-cellular with sub-second SRT.
- **Remote production**: laptop screen capture → SRT (encrypted) →
  cloud media server → distribution. Replaces VPN+RTMP setups.
- **SRT relay**: receive SRT, transcode (via `Mixer`), re-emit as
  SRT or RTMP. The runtime is already graph-shaped, so building a
  relay is `CreateSource srt://in → Connect → CreateDestination Srt`.

This phase is **optional, independent, and post-MVP**. It does not
block, and is not blocked by, any of PHASES 1–7. It can ship any
time after PHASE-3 (which establishes the migration runtime smoke
infrastructure used in §3.4–3.6).
