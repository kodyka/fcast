# MVP-PHASE-8 — Step 6: source-side (documentation only)

> Part 6 of 6. Parent doc: [`MVP-PHASE-8-srt-destination-family.md`](./MVP-PHASE-8-srt-destination-family.md).
> Previous: [Step 5 — unit tests](./MVP-PHASE-8-STEP-5-unit-tests.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

Document that **SRT sources already work without any source-side
code change** and add one trivial dispatcher test to enforce that
fact at the test layer (so a future contributor doesn't add a
redundant arm to `SourceNode`).

This is a **documentation step**. No new Rust constructs are added
to `SourceNode`; no new factories, no new pipeline arms, no new
URI parser.

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| `SourceNode::build_live_pipeline` | `senders/android/src/migration/nodes/source.rs` (search for `fn build_live_pipeline`) |
| Source factory selection (`fallbacksrc` or `uridecodebin`) | same file |
| GStreamer's URI handler registry | runtime — populated by Step 4 |
| Existing `Command::CreateSource` tests | `senders/android/src/migration/node_manager.rs` |

### 1.2 Why no source-side code change is needed

GStreamer's `uridecodebin` (and `fallbacksrc`, which wraps it) calls
`gst::URIHandlerFactory::find` to resolve a URI scheme to a source
element factory. The `srt` plugin (bundled in Step 4) registers
**`srtsrc` as the URI handler for `srt://`** automatically via
`gst_plugin_register`.

When `SourceNode::build_live_pipeline` receives:

```json
{"createsource": {"id": "srt-in", "uri": "srt://0.0.0.0:9000?mode=listener"}}
```

…it constructs `uridecodebin` (or `fallbacksrc`) and sets its
`uri` property to `srt://0.0.0.0:9000?mode=listener`. GStreamer
internally:

1. Strips the scheme → `srt://`.
2. Looks up the URI handler → `srtsrc`.
3. Wraps `srtsrc` in a bin together with the right demuxer
   (`tsdemux`, since SRT carries MPEG-TS by convention).
4. Exposes the demuxed pads as the `pad-added` signal that
   `SourceNode` already handles.

Steps 1–4 happen inside `uridecodebin`, transparent to `SourceNode`.
**The existing `pad-added` dispatch in `build_live_pipeline` handles
dynamic pads from any demuxer.**

The only thing that could break this is the SRT plugin not being
registered at runtime — which is exactly what Step 4 fixes.

### 1.3 The same pattern as `udp://`, `rtmp://`, `file://`

`udp://` is **not** in `SourceNode` because `udpsrc` registers itself
as the URI handler. Same for `rtmp://` / `rtmp2src`, `file://` /
`filesrc`, `http://` / `souphttpsrc`. The pattern is:

> If a GStreamer plugin registers a `URIHandlerFactory` for a
> scheme, `SourceNode` requires zero code changes to accept that
> scheme — `uridecodebin` / `fallbacksrc` dispatches transparently.

SRT follows this pattern: zero code changes.

---

## 2. The change

This step is **documentation only**. The only "code change" is the
single dispatcher test added in
[Step 5 — unit tests](./MVP-PHASE-8-STEP-5-unit-tests.md) §2.1
under `create_source_accepts_srt_uri`. That test exists to:

1. Prevent a future contributor from adding redundant `srt://`
   handling to `SourceNode` (e.g. a `match` arm in
   `build_live_pipeline` that explicitly constructs `srtsrc`). The
   test passes whether or not such an arm exists, so its real
   value is as **documentation in test form** — anyone removing
   the test must justify why `srt://` should require special
   handling.

2. Catch any future refactor that breaks `Command::CreateSource`'s
   URI passthrough.

The test itself (repeated here from Step 5 for completeness):

```rust
// senders/android/src/migration/node_manager.rs
#[test]
fn create_source_accepts_srt_uri() {
    let mut manager = NodeManager::default();
    let result = manager.dispatch(Command::CreateSource {
        id: "srt-in-1".into(),
        uri: "srt://0.0.0.0:9000?mode=listener".into(),
        audio: true,
        video: true,
    });
    assert!(matches!(result, CommandResult::Success), "{result:?}");
    // SourceNode dispatches to fallbacksrc/uridecodebin in the refresh
    // loop — no scheme-specific routing needed.
}
```

This test does **not** require the SRT plugin to be loaded — it
verifies that `NodeManager::dispatch` accepts the URI string
unmodified. The actual GStreamer-level SRT handshake only happens
when the source enters the `Playing` state in the refresh loop,
which is covered by the on-device smoke in §3.3 below.

---

## 3. Verification

### 3.1 Unit-test layer

```bash
cargo +nightly test -p fcast-sender-android \
    create_source_accepts_srt_uri
```

Expect green. **No GStreamer initialisation required.**

### 3.2 Documentation grep

```bash
grep -rn 'srt://' senders/android/src/migration/ --include='*.rs'
# → expect: only test fixtures and comments. No production code path
#   matches the literal `srt://` — that's the whole point of this step.
```

If a future PR introduces a production-code match on `srt://` inside
`SourceNode`, the grep above flags it for review. The expected
production behaviour is **opaque URI passthrough** to GStreamer.

### 3.3 On-device smoke (source-side)

Pre-reqs:
- [Step 4](./MVP-PHASE-8-STEP-4-android-makefile.md) landed (SRT
  plugin registered).
- MVP-PHASE-3 verified the migration runtime command server is
  reachable.
- A second host with `gst-launch-1.0`.

```bash
# 1. On a separate host, push an SRT stream to the phone.
# Make sure the phone's listening IP is reachable.

PHONE_IP=$(adb shell ip route | awk '/wlan|rmnet/ {print $9; exit}')
gst-launch-1.0 -v \
    videotestsrc is-live=true \
    ! videoconvert \
    ! x264enc tune=zerolatency \
    ! h264parse \
    ! mpegtsmux \
    ! srtsink uri="srt://${PHONE_IP}:9000" latency=200

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

**Expected:** the phone's local-playback surface shows the ball
pattern within ~2s of `start`. No source-side code path was added —
this works purely because Step 4 registered the `srt` plugin and
`uridecodebin` dispatched the URI to `srtsrc` automatically.

### 3.4 Encryption smoke (source-side)

Same as §3.3, with passphrase:

```bash
# Sender:
gst-launch-1.0 -v videotestsrc is-live=true \
    ! videoconvert ! x264enc tune=zerolatency ! h264parse ! mpegtsmux \
    ! srtsink uri="srt://${PHONE_IP}:9000?passphrase=topsecret&pbkeylen=16"

# Phone source — passphrase in the URI query (uridecodebin passes it
# through to srtsrc).
curl -X POST http://127.0.0.1:8080/command \
     -d '{"createsource":{"id":"srt-in-enc","uri":"srt://0.0.0.0:9000?mode=listener&passphrase=topsecret&pbkeylen=16","audio":false,"video":true}}'
```

**Expected:** stream flows. Mismatched passphrases produce a
silent connection-rejected at the SRT handshake layer — visible in
`adb logcat | grep srtsrc` as `SRT: unauthorized` or `key length
mismatch`.

Source-side encryption properties (passphrase/pbkeylen) are passed
through the URI query string, not as a separate JSON field. **This
is a deliberate asymmetry with the destination side** — the source
URI is a single opaque string by design (it can carry any
`srtsrc`-supported parameter), while the destination has structured
JSON fields for the most common SRT properties.

If you need a structured Source-side `srt` config in the future
(e.g. for UI surfaces that want to introspect SRT-specific
properties), that's a **separate** post-PHASE-8 step — out of scope
here.

---

## 4. Pitfalls specific to this step

### S6-P1 — Adding a `match uri.scheme()` arm to `SourceNode`

```rust
// ❌ Anti-pattern — do not add this.
match uri.scheme() {
    "srt" => Self::build_srt_source_element(...),
    _ => Self::build_uridecodebin_source(...),
}
```

This duplicates work `uridecodebin` already does and creates two
codepaths for the same scheme. The motivation is usually "I want
fine control over SRT properties" — but `uridecodebin`'s
`source-setup` signal lets you configure the underlying `srtsrc`
**without** introducing a parallel codepath:

```rust
uridecodebin.connect("source-setup", false, |args| {
    let source = args[1].get::<gst::Element>().unwrap();
    if source.factory().map(|f| f.name() == "srtsrc").unwrap_or(false) {
        // Apply structured SRT-specific properties here, if needed.
        // …
    }
    None
});
```

Strongly prefer this pattern. The `create_source_accepts_srt_uri`
test in Step 5 stays green either way; the **anti-pattern** is
adding a parallel codepath that bypasses `uridecodebin`.

### S6-P2 — Parsing the URI in Rust to extract query params

```rust
// ❌ Anti-pattern.
let parsed = url::Url::parse(&uri)?;
let passphrase = parsed.query_pairs()
    .find(|(k, _)| k == "passphrase")
    .map(|(_, v)| v.into_owned());
```

GStreamer's URI parser already handles this. Parsing the URI a
second time in Rust risks divergence: GStreamer might URL-decode
`%20`-encoded passphrases while a hand-rolled Rust parser doesn't,
or vice versa. **Trust GStreamer's URI handling.** If the user
provides a malformed URI, `srtsrc` will reject it with a clear
error message at start time.

### S6-P3 — Documenting "Source-side SRT requires PHASE-8"

Phrasing for the changelog / docs:

> ✅ "PHASE-8 enables SRT for both source and destination. Sources
> work transparently via `fallbacksrc` / `uridecodebin` — pass an
> `srt://` URI to `Command::CreateSource`. Destinations use the new
> `DestinationFamily::Srt` variant introduced in PHASE-8."

> ❌ "PHASE-8 adds SRT source support."

The second framing implies new source-side code; there isn't any.
The work is **plugin registration** (Step 4) plus **a destination
arm** (Steps 1–3). Source-side is enabled by Step 4 alone.

---

## 5. Stop conditions for PHASE-8

After Steps 1–6, the phase is "done" when:

1. `cargo check` is clean across all targets.
2. All ~12 unit tests from Step 5 pass.
3. `srtsink` and `srtsrc` are present in the runtime element
   registry (Step 4 §3.3 confirms).
4. The destination smoke in Step 3 §3.3 displays the ball pattern
   on the remote `gst-launch` listener within ~1s of `start`.
5. The source smoke in Step 6 §3.3 displays the remote ball pattern
   on the phone within ~2s of `start`.
6. The encryption smoke in Step 6 §3.4 succeeds with matching
   passphrases and fails (`unauthorized` at the SRT handshake) on
   mismatched passphrases.
7. New surface area is visible to:

```bash
grep -n 'DestinationFamily::Srt' senders/android/src/migration/
# → expect: protocol.rs (Step 1), nodes/destination.rs (Steps 2+3)
```

8. The Android plugin list now bundles `srt`:

```bash
grep -nE '^\s+srt\b' senders/android/app/jni/Android.mk
# → expect: exactly one line in GSTREAMER_PLUGINS.
```

9. No new source-side scheme-specific arm exists:

```bash
grep -rn '"srt"' senders/android/src/migration/nodes/source.rs
# → expect: zero matches in production code (tests only, if any).
```

The phase is **complete** when all nine items hold. PHASE-8 is
**not** an MVP gate, **not** required for the Android cast loop,
and **independent** of every Tier 1 unification phase
(PHASES 4 → 5 → 6).
