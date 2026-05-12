# MVP-PHASE-8 — Step 1: extend the JSON protocol with `DestinationFamily::Srt`

> Part 1 of 6. Parent doc: [`MVP-PHASE-8-srt-destination-family.md`](./MVP-PHASE-8-srt-destination-family.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

Add a new `Srt` variant to the `DestinationFamily` enum (and propagate
through `DestinationInfo`) so the migration runtime accepts:

```json
{
  "createdestination": {
    "id": "srt-out-1",
    "family": {
      "Srt": {
        "uri": "srt://media-server.example.com:1234",
        "latency": 200,
        "passphrase": "topsecret",
        "pbkeylen": 16
      }
    },
    "audio": true,
    "video": true
  }
}
```

The wire format must stay backward-compatible for the four pre-existing
variants (`Rtmp / Udp / LocalFile / LocalPlayback`).

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| `DestinationFamily` enum | `senders/android/src/migration/protocol.rs:126-138` |
| Existing variants (Rtmp / Udp / LocalFile / LocalPlayback) | same |
| `DestinationInfo` consumer | `senders/android/src/migration/protocol.rs:151-160` |
| `node_manager.rs` dispatch (`create_destination`) | family-agnostic — no change needed in this step |

### 1.2 Why a new variant (not extending `Udp`)

`Udp { host }` carries a single `host: String` and uses a numeric port
property on the sink. SRT carries:

- `uri: String` (full URL with optional `?mode=…&streamid=…` query params),
- `latency` (ms, optional),
- `passphrase` + `pbkeylen` (encryption pair, optional).

That's a different shape. Modelling it as its own variant keeps the
JSON protocol self-documenting and the pipeline-construction code in
**Step 3** linear.

### 1.3 Why optional fields use `#[serde(default, skip_serializing_if = ...)]`

Keeps the wire format minimal: a destination with no encryption and
default latency serializes to `{"Srt":{"uri":"srt://h:1"}}`, not
`{"Srt":{"uri":"srt://h:1","latency":null,"passphrase":null,"pbkeylen":null}}`.

---

## 2. The change

**File:** `senders/android/src/migration/protocol.rs` (extend the
`DestinationFamily` enum at lines 126-138):

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
        /// Must be 10–79 ASCII characters when present.
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

`DestinationInfo` (lines 151-160) stores the family by value, so the
new variant flows through to `getinfo` responses automatically — **no
edit needed** on `DestinationInfo` itself for this step. (Step 3 will
plumb a couple of optional runtime-state fields onto
`DestinationInfo`, but they're for status reporting, not for the
inbound JSON.)

`Hash` + `Eq` derives are already on the enum — the new fields are all
`Eq`-compatible (`String`, `Option<u32>`), so no further annotations
are needed.

---

## 3. Verification

### 3.1 Compile check

```bash
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android
```

Expect **clean** — the change is purely additive. Any existing match on
`DestinationFamily` that was previously exhaustive will now fail to
compile until **Step 2** lands an arm for `Srt` in
`DestinationPipelineProfile::from_family` and **Step 3** lands the
`Srt` arm in `build_live_pipeline`. **That's intentional:** the three
steps want to land together.

If you want to commit Step 1 in isolation, add a temporary `_` arm to
those two `match` blocks:

```rust
// Temporary — replaced by Step 2.
DestinationFamily::Srt { .. } => Self::default(),
```

```rust
// Temporary — replaced by Step 3.
DestinationFamily::Srt { .. } => unimplemented!("Srt arm — see MVP-PHASE-8 Step 3"),
```

…but this defers the compile error to runtime. **Preferred:** land
Steps 1+2+3 in the same commit.

### 3.2 Serde shape

Drop into the existing protocol-test module
(`senders/android/src/migration/protocol.rs` or a sibling test file)
and verify the round-trip:

```rust
#[test]
fn srt_destination_with_encryption_serdes_roundtrip() {
    let original = DestinationFamily::Srt {
        uri: "srt://example.com:1234?mode=caller".into(),
        latency: Some(120),
        passphrase: Some("0123456789".into()), // 10 chars — SRT minimum
        pbkeylen: Some(16),
    };
    let json = serde_json::to_string(&original).unwrap();
    let parsed: DestinationFamily = serde_json::from_str(&json).unwrap();
    assert_eq!(original, parsed);

    // Wire format is the externally-tagged enum shape, same as the
    // other variants.
    assert!(json.starts_with(r#"{"Srt":{"uri":"srt://example.com:1234"#));
}

#[test]
fn srt_destination_optional_fields_omitted_in_minimal_json() {
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

Both green after Step 1.

### 3.3 Grep recipe

```bash
grep -nA2 'DestinationFamily::Srt' senders/android/src/migration/protocol.rs
# → expect: one match (the enum arm) with `uri`, `latency`, `passphrase`, `pbkeylen`.
```

---

## 4. Pitfalls specific to this step

### S1-P1 — Forgetting `#[serde(default, skip_serializing_if = "Option::is_none")]` on the optional fields

If you write only `latency: Option<u32>` without the attributes, two
things break:

1. Inbound JSON without a `latency` key fails to deserialize with
   `missing field 'latency'` — `Option<u32>` does **not** imply
   `#[serde(default)]` on its own.
2. Outbound JSON includes `"latency":null` for the four pre-existing
   variants — backward-incompat for anyone parsing the wire format
   strictly.

Both attributes are necessary. The other `Destination*` variants do
the same — see `LocalFile.max_size_time` at line 137 for the existing
pattern.

### S1-P2 — Adding `pbkeylen` as `u8` to "save space"

`srtsink`'s `pbkeylen` property is `i32` in the GStreamer GObject
type system. Using `u32` is the smallest type that round-trips
through serde without sign issues. **`u8` is wrong** — serde accepts
values up to 255 but SRT only takes `16`/`24`/`32`, so the wider
type also doesn't cost anything. Don't optimize this.

### S1-P3 — Storing `latency` as `Duration`

`serde` can't serialize `std::time::Duration` to a single JSON
integer — it goes to `{"secs": N, "nanos": M}`, which is verbose and
non-idiomatic for transport-tuning properties. Keep `latency` as
`Option<u32>` milliseconds.

---

## 5. Next step

After this lands, [STEP 2 — extend `DestinationPipelineProfile::from_family`](./MVP-PHASE-8-STEP-2-pipeline-profile.md)
adds the new arm to the diagnostic element listing so
`getinfo`-style introspection lists `mpegtsmux` + `srtsink` for
`Srt` destinations.
