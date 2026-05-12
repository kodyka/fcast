# MVP-PHASE-6 — Step 3: extract the WHEP-URL helper out of `WhepSink`

> Part 3 of 9. Parent doc: [`MVP-PHASE-6-graph-command-cast-loop.md`](./MVP-PHASE-6-graph-command-cast-loop.md).
> Previous: [Step 2 — replace `Event::CaptureStarted`](./MVP-PHASE-6-STEP-2-capturestarted-rewrite.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

The existing `Event::SignallerStarted` handler (`lib.rs:754-794`)
builds the WHEP receiver URL by calling
`self.tx_sink.as_ref().unwrap().get_play_msg(...)`. After
[Step 5](./MVP-PHASE-6-STEP-5-tx-sink-cfg-gate.md) gates `tx_sink`
to non-Android targets, that call site won't compile on Android.

This step moves the URL-construction logic out of
`mcore::transmission::WhepSink` and into a **free function**
`mcore::transmission::build_whep_play_msg(addr, bound_port)` that
both the legacy `WhepSink::get_play_msg` and the new Android
adapter call. The legacy `WhepSink::get_play_msg` becomes a
one-line wrapper around the new helper.

After this step, the `Event::SignallerStarted` handler is one line
shorter and `tx_sink`-free on Android.

This is a **small, structural step** (~10 SDK lines + ~3
`lib.rs` lines). Pure refactor, no behaviour change.

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| `Event::SignallerStarted` handler | `senders/android/src/lib.rs:754-794` |
| The current `get_play_msg(...)` call | `lib.rs:767-771` |
| `WhepSink::get_play_msg` | `sdk/mirroring_core/src/transmission.rs:530-548` |
| `addr_to_url_string` helper | `sdk/mirroring_core/src/transmission.rs:550-562` (or near there — wraps IPv6 in `[...]`) |
| WHEP endpoint path | `whep_signaller.rs:33-35` (just `/endpoint`) |

### 1.2 Why a free function (not a static method on `WhepSink`)

Two reasons:

1. The Android adapter shouldn't have to keep a `WhepSink` alive
   just to call a URL-builder. The whole point of PHASE-6 is to
   delete `WhepSink` usage on Android.
2. Free function = no implicit `self.state` dependency. The URL is
   `http://<addr>:<port>/endpoint` + `application/sdp` content type
   — none of `WhepSink`'s state (`local_address`, `pipeline`,
   `tx_sink`) influences it.

### 1.3 Why content-type is `application/sdp` (not `application/sdp+offer` etc.)

WHEP per the IETF draft (`draft-ietf-wish-whep-04`) defines the
content type as `application/sdp` for the initial POST. The
`WhepSink` legacy implementation already hardcodes this string —
keep it identical.

---

## 2. The change

### 2.1 Add a free function in `mcore`

**File:** `sdk/mirroring_core/src/transmission.rs` (anywhere — top
of the file or just above `WhepSink::get_play_msg`):

```rust
// sdk/mirroring_core/src/transmission.rs

use std::net::IpAddr;

/// Build the (content-type, URL) pair for a WHEP receiver to POST
/// its initial SDP offer to.
///
/// `addr` is the local IP the receiver should connect to (typically
/// the sender's LAN address, learned via mDNS or the FCast
/// `local_address` field). `bound_port` is the OS-picked port
/// emitted by `WhepServerSignaller::on-server-started`.
pub fn build_whep_play_msg(addr: IpAddr, bound_port: u16) -> (String, String) {
    let host = addr_to_url_string(addr);
    let url = format!("http://{host}:{bound_port}/endpoint");
    ("application/sdp".to_string(), url)
}
```

`addr_to_url_string` already exists in the same file (or one of its
helpers) — that's the function that does `format!("[{}]", v6)` for
IPv6 literals.

### 2.2 Make `WhepSink::get_play_msg` a thin wrapper

```rust
// sdk/mirroring_core/src/transmission.rs

impl WhepSink {
    pub fn get_play_msg(&self, addr: IpAddr, bound_port: u16) -> (String, String) {
        build_whep_play_msg(addr, bound_port)
    }
}
```

This preserves the existing API for desktop (the desktop sender
still uses `WhepSink`), but lets Android call the free function
directly without needing a `WhepSink` instance.

### 2.3 Switch `Event::SignallerStarted` to the free function

**File:** `senders/android/src/lib.rs`

**Before** (lines 767-771):

```rust
let (content_type, url) = self
    .tx_sink
    .as_ref()
    .unwrap()
    .get_play_msg(addr.into(), bound_port);
```

**After:**

```rust
let (content_type, url) =
    mcore::transmission::build_whep_play_msg(addr.into(), bound_port);
```

The rest of the handler (which calls `device.load(content_type,
url)`) is unchanged.

### 2.4 What about `local_address` / IPv6 selection?

`Event::SignallerStarted { bound_port_v4, bound_port_v6 }` carries
both ports. The legacy handler picks one based on which family the
discovered receiver was on. That selection logic stays in
`lib.rs:754-794` and is unaffected by this step — we only move the
URL-string assembly out of `WhepSink`.

### 2.5 Re-export from `mcore::transmission` if needed

`mcore::transmission` is already a `pub mod` from
`sdk/mirroring_core/src/lib.rs`. The new `build_whep_play_msg` is
`pub`, so callers can write
`mcore::transmission::build_whep_play_msg(...)` directly. No
re-export changes needed.

---

## 3. Verification

### 3.1 Compile check (Android + desktop)

```bash
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android
cargo +nightly check -p fcast-sender-desktop
```

Both clean. The desktop build still uses `WhepSink::get_play_msg`
(via the legacy cast loop, which exists on non-Android targets
only after [Step 5](./MVP-PHASE-6-STEP-5-tx-sink-cfg-gate.md)), so
the wrapper is genuinely live code.

### 3.2 Unit test for the helper (host-runnable)

**File:** `sdk/mirroring_core/src/transmission.rs` (inside the
existing `#[cfg(test)] mod tests`):

```rust
#[test]
fn build_whep_play_msg_emits_correct_shape() {
    use std::net::Ipv4Addr;
    let (ct, url) = build_whep_play_msg(
        Ipv4Addr::new(192, 168, 1, 50).into(),
        40123,
    );
    assert_eq!(ct, "application/sdp");
    assert_eq!(url, "http://192.168.1.50:40123/endpoint");
}

#[test]
fn build_whep_play_msg_wraps_ipv6_in_brackets() {
    use std::net::Ipv6Addr;
    let (_, url) = build_whep_play_msg(
        Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1).into(),
        40123,
    );
    assert!(url.contains("[fe80::1]"),
            "expected bracketed IPv6 literal, got {url}");
}
```

### 3.3 Grep

```bash
grep -n 'fn build_whep_play_msg' sdk/mirroring_core/src/transmission.rs
# → 1 match
grep -nE 'self\.tx_sink\.as_ref\(\)\.unwrap\(\)\.get_play_msg' senders/android/src/lib.rs
# → 0 matches (replaced by the free-function call)
grep -nE 'build_whep_play_msg' senders/android/src/lib.rs
# → 1 match (the new call site)
```

---

## 4. Pitfalls specific to this step

### P1 — Forgetting `pub` on the new function

`build_whep_play_msg` must be `pub fn` (not the default
private). Without `pub`, `senders/android` can't reach it.

### P2 — Mutating `WhepSink::get_play_msg` signature

Don't change `&self` to `()` on `WhepSink::get_play_msg` — the
desktop sender's existing call sites pass `&self`. Keep the API
backwards-compatible.

### P3 — `IpAddr` ambiguity at the call site

`addr.into()` in the existing call relies on `From<X> for IpAddr`
where `X` is whatever the local-address field is typed as in
`lib.rs` (probably `Ipv4Addr` or `IpAddr` directly). If the call
site fails to compile with `the trait From<…> for IpAddr is not
implemented`, just write `addr.into()` explicitly with type
annotation: `<IpAddr as From<_>>::from(addr)`.

### P4 — Don't move the function out of `mcore::transmission`

Tempting to put it in a new file like
`mcore::whep::play_msg::build`. Don't — the consumers
(`senders/android/lib.rs`, `senders/desktop/lib.rs`, the legacy
`WhepSink`) all reach `mcore::transmission`. Keeping the helper
there minimises import churn.

### P5 — `application/sdp` is hardcoded both places

Don't accidentally extract it to a `const` named
`WHEP_CONTENT_TYPE` — the legacy `WhepSink::get_play_msg` writes
the literal `"application/sdp"` inline. Diff churn outweighs the
"avoid hardcoded string" hygiene.

---

## 5. Next step

Once this lands, [Step 4](./MVP-PHASE-6-STEP-4-stop-cast-rewrite.md)
rewrites `stop_cast(...)` and `Event::EndSession` to issue
`Disconnect` + `Remove` graph commands instead of calling
`tx_sink.shutdown()`.
