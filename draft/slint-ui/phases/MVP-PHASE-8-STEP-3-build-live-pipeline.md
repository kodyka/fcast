# MVP-PHASE-8 — Step 3: wire the `Srt` arm in `build_live_pipeline`

> Part 3 of 6. Parent doc: [`MVP-PHASE-8-srt-destination-family.md`](./MVP-PHASE-8-srt-destination-family.md).
> Previous: [Step 2 — `DestinationPipelineProfile::from_family`](./MVP-PHASE-8-STEP-2-pipeline-profile.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

Build the GStreamer pipeline for the `Srt` destination at start-time:

```
video_appsrc ──► videoconvert ──► h264enc ──► h264parse ─┐
                                                          ├─► mpegtsmux ──► srtsink
audio_appsrc ──► audioconvert ──► audioresample ──► aac ─┘
```

The shape is a **near-verbatim port of the existing `Udp` arm** at
`senders/android/src/migration/nodes/destination.rs:606-679`. The only
differences are the network sink (`srtsink` vs `udpsink`) and a couple
of SRT-specific properties (`latency`, `passphrase`, `pbkeylen`).

This is the largest step in PHASE-8 — roughly **90 lines of Rust**,
one match arm.

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| `DestinationNode::build_live_pipeline` (the outer function) | `senders/android/src/migration/nodes/destination.rs:489+` |
| `Udp` arm (the closest template) | `senders/android/src/migration/nodes/destination.rs:606-679` |
| `Self::make_element` helper | same file (search for `fn make_element`) |
| `Self::select_video_encoder` (encoder fallback chain) | same file |
| `Self::add_video_encoder_chain` / `Self::link_video_encoder_chain` | same file |
| MPEG-TS muxer alignment trick | `nodes/destination.rs:619-621` (in the UDP branch) |

### 1.2 Why mirror the UDP arm

Both pipelines are MPEG-TS over UDP/SRT — same muxer, same encoder
selection, same audio chain. The only divergence is the network sink:

| Concern | UDP arm | SRT arm |
|---|---|---|
| Sink factory | `udpsink` | `srtsink` |
| Host/port config | `host` + `port` properties | `uri` (single property) |
| Latency tuning | n/a | `latency` (i32 ms, optional) |
| Encryption | n/a | `passphrase` + `pbkeylen` (optional pair) |
| `mpegtsmux` alignment | `7` | `7` (same) |
| Video chain | `appsrc → videoconvert → h264 chain → h264parse → mux` | same |
| Audio chain | `appsrc → audioconvert → audioresample → avenc_aac → mux` | same |

Keep the structure literal — every reviewer who has touched UDP will
recognize the shape immediately.

### 1.3 `srtsink` property quirks

`srtsink` exposes several properties; only four are interesting here:

| Property | GObject type | Notes |
|---|---|---|
| `uri` | `gchararray` | Full SRT URI, including `?mode=…&streamid=…&latency=…&passphrase=…` query string. The `latency` / `passphrase` / `pbkeylen` properties duplicate the URI query — set whichever is more ergonomic. |
| `latency` | `gint` | Milliseconds. Default 200ms. Should match the peer. |
| `passphrase` | `gchararray` | 10–79 ASCII characters. Silently rejected by handshake if too short. |
| `pbkeylen` | `gint` | 16 / 24 / 32. Required if `passphrase` is set. |

Use `sink.has_property(...)` before setting `passphrase` / `pbkeylen` —
older versions of `srtsink` (e.g. `gst-plugins-bad` < 1.16) may not
expose them. The `make_element` helper doesn't know the property
schema at compile time; defensive checks are necessary.

---

## 2. The change

**File:** `senders/android/src/migration/nodes/destination.rs`
(extend the `match &self.family { … }` block at line 489 — model on
the `Udp` branch at lines 606-679):

```rust
DestinationFamily::Srt {
    uri,
    latency,
    passphrase,
    pbkeylen,
} => {
    let mux = Self::make_element("mpegtsmux", None)?;
    let sink = Self::make_element("srtsink", None)?;

    pipeline.add(&mux).map_err(|err| {
        format!("Failed to add mpegtsmux to srt pipeline: {err:?}")
    })?;
    pipeline.add(&sink).map_err(|err| {
        format!("Failed to add srtsink to srt pipeline: {err:?}")
    })?;

    // ── SRT-specific properties ────────────────────────────────────
    sink.set_property("uri", uri.clone());

    if let Some(lat) = latency {
        // `srtsink` exposes `latency` as i32 milliseconds.
        sink.set_property("latency", *lat as i32);
    }
    if let Some(pass) = passphrase {
        // `passphrase` only takes effect when `pbkeylen` is also set.
        // The pair must be present together — see pitfall S3-P3.
        if sink.has_property("passphrase") {
            sink.set_property("passphrase", pass.clone());
        }
    }
    if let Some(keylen) = pbkeylen {
        if sink.has_property("pbkeylen") {
            sink.set_property("pbkeylen", *keylen as i32);
        }
    }

    // MPEG-TS alignment — same as UDP (line 619-621). Without this,
    // some receivers (e.g. ffmpeg) misalign on packet boundaries.
    if mux.has_property("alignment") {
        mux.set_property("alignment", 7i32);
    }

    // ── Video chain (mirror of UDP video chain, lines 623-647) ─────
    if let Some(appsrc) = video_appsrc.as_ref() {
        let vconv = Self::make_element("videoconvert", None)?;
        let venc_chain = Self::select_video_encoder(&self.id)?;
        let vparse = Self::make_element("h264parse", None)?;

        pipeline.add(&vconv).map_err(|err| {
            format!("Failed to add videoconvert to srt pipeline: {err:?}")
        })?;
        Self::add_video_encoder_chain(&pipeline, &venc_chain, "srt pipeline")?;
        pipeline.add(&vparse).map_err(|err| {
            format!("Failed to add h264parse to srt pipeline: {err:?}")
        })?;

        gst::Element::link_many(
            [appsrc.upcast_ref::<gst::Element>(), &vconv].as_slice(),
        )
        .map_err(|err| format!("Failed to link srt video preprocessing: {err:?}"))?;

        Self::link_video_encoder_chain(
            &vconv,
            &venc_chain,
            &vparse,
            "srt video encoder chain",
        )?;

        gst::Element::link_many([&vparse, &mux].as_slice())
            .map_err(|err| format!("Failed to link srt video output: {err:?}"))?;
    }

    // ── Audio chain (mirror of UDP audio chain, lines 649-675) ─────
    if let Some(appsrc) = audio_appsrc.as_ref() {
        let aconv = Self::make_element("audioconvert", None)?;
        let aresample = Self::make_element("audioresample", None)?;
        let aenc = Self::make_element("avenc_aac", None)?;

        pipeline.add(&aconv).map_err(|err| {
            format!("Failed to add audioconvert to srt pipeline: {err:?}")
        })?;
        pipeline.add(&aresample).map_err(|err| {
            format!("Failed to add audioresample to srt pipeline: {err:?}")
        })?;
        pipeline.add(&aenc).map_err(|err| {
            format!("Failed to add avenc_aac to srt pipeline: {err:?}")
        })?;

        gst::Element::link_many(
            [
                appsrc.upcast_ref::<gst::Element>(),
                &aconv,
                &aresample,
                &aenc,
                &mux,
            ]
            .as_slice(),
        )
        .map_err(|err| format!("Failed to link srt audio chain: {err:?}"))?;
    }

    // ── Connect muxer to sink ──────────────────────────────────────
    mux.link(&sink)
        .map_err(|err| format!("Failed to link mpegtsmux to srtsink: {err:?}"))?;
}
```

That's the entire diff for this step. ~90 lines, one match arm.

---

## 3. Verification

### 3.1 Compile check

```bash
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android
```

After Steps 1+2+3, all `DestinationFamily` matches are exhaustive
again — expect **clean**.

### 3.2 Pipeline construction smoke (no GStreamer registry required)

The `make_element` helper returns `Err(...)` when the factory is
missing — so a unit test that **expects success** can't run without
`gst::init()` and the SRT plugin actually being available. For
host-side tests on a developer laptop with `gst-plugins-bad`
installed, this works:

```rust
#[cfg(all(test, target_os = "linux"))]
mod srt_pipeline_construction_tests {
    use super::*;

    #[test]
    fn srt_destination_builds_without_panic() {
        gst::init().expect("gst init");
        if gst::ElementFactory::find("srtsink").is_none() {
            eprintln!("srtsink not registered on this host; skipping");
            return;
        }

        let node = DestinationNode::new(
            "srt-out-test".into(),
            DestinationFamily::Srt {
                uri: "srt://127.0.0.1:9999".into(),
                latency: Some(200),
                passphrase: None,
                pbkeylen: None,
            },
            /* audio */ true,
            /* video */ true,
        );
        let result = node.build_live_pipeline_for_test();
        assert!(result.is_ok(), "{result:?}");
    }
}
```

The `build_live_pipeline_for_test()` helper is a thin wrapper that
calls `build_live_pipeline` with synthesized `appsrc`s and returns
the `gst::Pipeline`. Add it as a `#[cfg(test)]` method if one doesn't
already exist:

```rust
#[cfg(test)]
fn build_live_pipeline_for_test(&self) -> Result<gst::Pipeline, String> {
    let pipeline = gst::Pipeline::new(Some(&format!("{}-test", self.id)));

    let video_appsrc = if self.video {
        Some(gst_app::AppSrc::builder().build())
    } else { None };
    let audio_appsrc = if self.audio {
        Some(gst_app::AppSrc::builder().build())
    } else { None };

    self.build_live_pipeline(&pipeline, &video_appsrc, &audio_appsrc)?;
    Ok(pipeline)
}
```

On a host without `srtsink`, the test no-ops with a printed reason.

### 3.3 On-device smoke

Pre-reqs:
- [Step 4](./MVP-PHASE-8-STEP-4-android-makefile.md) landed (the SRT
  plugin is in `GSTREAMER_PLUGINS`).
- MVP-PHASE-3 verified the migration runtime command server is
  reachable via `MIGRATION_COMMAND_BIND=127.0.0.1:8080` +
  `adb forward tcp:8080 tcp:8080`.
- A second host with `srt-live-transmit` or `gst-launch-1.0`.

```bash
# 1. On a separate host, listen for the SRT stream.
gst-launch-1.0 -v \
    srtsrc uri="srt://0.0.0.0:1234?mode=listener" latency=200 \
    ! tsdemux name=demux \
    ! queue ! h264parse ! avdec_h264 ! videoconvert ! autovideosink \
    demux. \
    ! queue ! aacparse ! avdec_aac ! audioconvert ! autoaudiosink

# 2. Back on the phone (via adb forward), build the SRT destination
#    graph in the migration runtime.

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

**Expected** within ~1s on the listener host: the GStreamer
`autovideosink` window opens and displays the ball-pattern test source
the `videogenerator` node produces.

### 3.4 Grep recipe

```bash
grep -nA5 'DestinationFamily::Srt {' \
    senders/android/src/migration/nodes/destination.rs
# → expect: two matches —
#     1. from_family (Step 2): 8 element-name strings
#     2. build_live_pipeline (THIS step): real pipeline construction
```

---

## 4. Pitfalls specific to this step

### S3-P1 — `srtsink`'s `latency` is **i32** milliseconds, not microseconds

```rust
sink.set_property("latency", *lat as i32);  // ✓ correct
```

If you pass `i64` (treating it as nanoseconds, the GStreamer
clock-time convention), `srtsink` complains:

```
GLib-GObject-WARNING **: cannot set property 'latency' of type 'gint' from value of type 'gint64'
```

Stick to `i32` and treat the value as milliseconds. The Step 1
protocol field is `Option<u32>` for the same reason.

### S3-P2 — `mpegtsmux` `alignment=7` is critical

The UDP arm sets `mux.set_property("alignment", 7i32)` at
lines 619-621. The Srt arm **must** do the same. Without it,
MPEG-TS packets emitted by the muxer aren't aligned to 188-byte
boundaries that `srtsink` expects, and some receivers (notably
FFmpeg) report `continuity counter` errors on every packet.

### S3-P3 — `passphrase` set, `pbkeylen` not set → silently unencrypted

`srtsink` requires **both** `passphrase` AND `pbkeylen` to enable
encryption. If only one is set, the other side's authentication will
reject the connection with no warning on the sender. The sender-side
log will look like a clean connect followed by an abrupt close.

The snippet above sets each property independently for clarity, but
**at the JSON deserializer level (Step 1)** the desired behaviour is
to reject `Srt { passphrase: Some(_), pbkeylen: None }` as malformed.
Optionally, add a constructor-level check in this arm:

```rust
if passphrase.is_some() != pbkeylen.is_some() {
    return Err("Srt destination: passphrase and pbkeylen must both be set, or both unset".into());
}
```

This step doesn't add the check (to keep the diff minimal); document
the gotcha in the parent doc's §4 Pitfalls.

### S3-P4 — `srtsink` blocks `pipeline.set_state(Playing)` if no receiver

In `caller` mode (the default), `srtsink` synchronously attempts to
connect on `PAUSED → PLAYING`. If the listener side isn't running,
the transition **blocks for up to `connect-timeout` (default 3000ms)**
and then fails. The `DestinationNode::refresh()` polls states with a
100ms tick, so the symptom is the destination sitting in `Starting`
for 3s before either succeeding or transitioning to `Stopped` with
`last_error: Some("Could not connect to receiver")`.

For one-way contribution feeds where the listener might come and go,
suggest `?mode=listener` on the sender side (the URI carries it) so
it accepts inbound connections. This requires the receiver to
initiate the connection — flip the topology.

### S3-P5 — `srt://` URI with IPv6 needs bracket escaping

```rust
DestinationFamily::Srt {
    uri: "srt://[fe80::1]:1234".into(),  // ✓ correct
    /* … */
}
```

Without the brackets, GStreamer's URI parser splits on the wrong
colon and reports `Invalid URI: srt://fe80::1:1234`. The same rule
applies to `udpsink` URI inputs — copy that behaviour.

### S3-P6 — Forgetting `upcast_ref::<gst::Element>()` on `AppSrc`

`gst::Element::link_many` takes `&[&gst::Element]`. `AppSrc` doesn't
deref to `gst::Element` automatically — you need
`appsrc.upcast_ref::<gst::Element>()` (as in the snippet) or the UDP
arm's equivalent helper. The compile error is
`expected reference, found struct AppSrc`.

### S3-P7 — `select_video_encoder` returns a multi-element chain

`select_video_encoder` returns a fallback chain (e.g.
`x264enc → vtenc_h264_hw → openh264enc`), not a single element. The
`add_video_encoder_chain` / `link_video_encoder_chain` helpers know
how to plumb the chain into the pipeline. **Don't try to
`pipeline.add(&encoder)` directly** — the chain may be multiple
elements wrapped in a `gst::Bin`.

### S3-P8 — Setting `uri` after `pipeline.set_state(Playing)`

`srtsink` reads `uri` once during the `READY → PAUSED` transition.
Setting `uri` after the pipeline is already playing has no effect
(it caches the value but doesn't reconnect). The snippet above sets
`uri` immediately after `pipeline.add(&sink)`, before any state
transition — that's the safe order. **Don't refactor to "set
properties last" without verifying the state-transition order.**

---

## 5. Next step

After this lands, [STEP 4 — bundle the SRT plugin in `Android.mk`](./MVP-PHASE-8-STEP-4-android-makefile.md)
adds `srt` to the sender's `GSTREAMER_PLUGINS` list so `srtsink`
(and `srtsrc`) actually exist at runtime on Android. **Step 3 will
compile without Step 4, but the runtime will fail with
"Could not create element of type srtsink. Plugin missing?"** —
Step 4 is mandatory before any on-device verification.
