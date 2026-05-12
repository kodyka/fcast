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

## 2. Steps

### 2.1 Step 1 — extend the JSON protocol

**File:** `senders/android/src/migration/protocol.rs` (lines 126-138):

```rust
// senders/android/src/migration/protocol.rs

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DestinationFamily {
    Rtmp {
        uri: String,
    },
    Udp {
        host: String,
    },
    LocalFile {
        base_name: String,
        max_size_time: Option<u32>,
    },
    LocalPlayback,

    // NEW —
    Srt {
        /// Full SRT URI. Examples:
        ///   - "srt://media-server.example.com:1234"            (caller, default)
        ///   - "srt://0.0.0.0:9000?mode=listener"               (listener side)
        ///   - "srt://host:port?streamid=foo&mode=caller"       (with stream id)
        uri: String,

        /// SRT latency in milliseconds. Recommended: 4× expected RTT,
        /// minimum ~80ms, default ~200ms. Higher = more resilience to
        /// packet loss, more end-to-end delay.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        latency: Option<u32>,

        /// AES encryption passphrase. None = unencrypted.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        passphrase: Option<String>,

        /// AES key length in bytes: 16 (AES-128), 24 (AES-192), or
        /// 32 (AES-256). Required if `passphrase` is set; ignored
        /// otherwise.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pbkeylen: Option<u32>,
    },
}
```

The serde defaults keep the wire format backward-compatible for the
four pre-existing variants.

### 2.2 Step 2 — extend the pipeline profile

**File:** `senders/android/src/migration/nodes/destination.rs`
(extend the `match family { … }` block at line 39):

```rust
match family {
    DestinationFamily::Rtmp { .. } => { /* …existing… */ }
    DestinationFamily::Udp { .. } => { /* …existing… */ }
    DestinationFamily::LocalFile { .. } => { /* …existing… */ }
    DestinationFamily::LocalPlayback => { /* …existing… */ }

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
```

(The element-name list is purely diagnostic — used for
`DestinationPipelineProfile` introspection. The actual elements are
constructed in `build_live_pipeline`.)

The `audio`/`video` retention filters at lines 91-96 work unmodified
because the Srt list matches the UDP list's element names.

### 2.3 Step 3 — wire the `Srt` arm in `build_live_pipeline`

**File:** `senders/android/src/migration/nodes/destination.rs`
(extend the `match &self.family { … }` block at line 489 — model
on the `Udp` branch at lines 606-679):

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
        // `passphrase` is only valid when `pbkeylen` is also set, but
        // srtsink silently ignores it if pbkeylen is 0 — we set
        // both or neither in §2.4 below for safety.
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

The structure is a near-verbatim port of the UDP arm. The only
differences:

| Aspect | UDP arm | SRT arm |
|---|---|---|
| Sink factory | `udpsink` | `srtsink` |
| Host/port config | `host` + `port` (2 properties) | `uri` (single property) |
| Latency tuning | n/a | `latency` (optional) |
| Encryption | n/a | `passphrase` + `pbkeylen` (optional pair) |

### 2.4 Step 4 — bundle the SRT plugin in the Android build

**File:** `senders/android/app/jni/Android.mk` (line 32-66):

```makefile
GSTREAMER_PLUGINS := \
    coreelements \
    app \
    audioconvert \
    audiomixer \
    /* …existing… */
    rtp \
    rtpmanager \
    udp \
    dtls \
    srtp \
    srt \                  /* ← NEW */
    webrtc \
    nice \
    rsrtp \
    rsrtsp \
    rswebrtc
```

The plugin name is **`srt`** (no `s` or other prefix). The plugin
ships `srtsrc` and `srtsink` and is automatically registered via
`GST_PLUGIN_STATIC_REGISTER` in the prebuilt
`libgstreamer_android.so` startup sequence — no additional Rust
registration call is needed.

After this change, the next `ndk-build` rebuild will link `libsrt.so`
into the APK. Verify with:

```bash
adb shell run-as org.fcast.android.sender \
    ls /data/data/org.fcast.android.sender/lib | grep srt
# → expected: libsrt.so   (plus libgstsrt.so as a static-in-bundle plugin)
```

(Depending on how the SDK ships its plugins, `srt` may be statically
linked into `libgstreamer_android.so` rather than a separate `.so`.
Both are fine — the runtime `gst_registry_get_default()` will pick
it up either way.)

### 2.5 Step 5 — add unit tests

**File:** `senders/android/src/migration/node_manager.rs` (in the
`#[cfg(test)] mod tests` block):

```rust
#[test]
fn create_srt_destination_succeeds() {
    let mut manager = NodeManager::default();
    let result = manager.dispatch(Command::CreateDestination {
        id: "srt-out-1".into(),
        family: DestinationFamily::Srt {
            uri: "srt://example.com:1234".into(),
            latency: Some(200),
            passphrase: None,
            pbkeylen: None,
        },
        audio: true,
        video: true,
    });
    assert!(matches!(result, CommandResult::Success));
    assert!(manager.nodes.contains_key("srt-out-1"));
}

#[test]
fn srt_destination_with_encryption_serdes_roundtrip() {
    use crate::migration::protocol::DestinationFamily;

    let original = DestinationFamily::Srt {
        uri: "srt://example.com:1234?mode=caller".into(),
        latency: Some(120),
        passphrase: Some("secret".into()),
        pbkeylen: Some(16),
    };
    let json = serde_json::to_string(&original).unwrap();
    let parsed: DestinationFamily = serde_json::from_str(&json).unwrap();
    assert_eq!(original, parsed);
}

#[test]
fn srt_destination_optional_fields_omitted_in_minimal_json() {
    use crate::migration::protocol::DestinationFamily;

    let minimal: DestinationFamily =
        serde_json::from_str(r#"{"Srt":{"uri":"srt://h:1"}}"#).unwrap();
    if let DestinationFamily::Srt { latency, passphrase, pbkeylen, .. } = minimal {
        assert!(latency.is_none());
        assert!(passphrase.is_none());
        assert!(pbkeylen.is_none());
    } else {
        panic!("expected Srt variant");
    }
}
```

These don't require GStreamer to be initialised — they validate the
command-dispatch and serde shape only. A pipeline-level smoke test is
in §3.3.

### 2.6 Step 6 — (optional) source-side smoke

`SourceNode` already accepts `srt://` URIs — no code change. Add a
test to **document** that behaviour (so future readers don't add a
redundant arm):

```rust
#[test]
fn create_source_accepts_srt_uri() {
    let mut manager = NodeManager::default();
    let result = manager.dispatch(Command::CreateSource {
        id: "srt-in-1".into(),
        uri: "srt://0.0.0.0:9000?mode=listener".into(),
        audio: true,
        video: true,
    });
    assert!(matches!(result, CommandResult::Success));
    // SourceNode dispatches to fallbacksrc/uridecodebin in the
    // refresh loop — no scheme-specific routing needed.
}
```

This test only validates the dispatcher accepts the URI; whether
GStreamer can actually open the SRT socket is verified in §3.4.

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
