# MVP-PHASE-5 — Step 1: extend the JSON protocol with `DestinationFamily::Whep`

> Part 1 of 7. Parent doc: [`MVP-PHASE-5-whep-destination-family.md`](./MVP-PHASE-5-whep-destination-family.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

Add a new `Whep` variant to `DestinationFamily` and extend
`DestinationInfo` with optional bound-port fields so the migration
runtime can accept:

```json
{
  "createdestination": {
    "id": "tv-1",
    "family": { "Whep": { "server_port": 0 } },
    "audio": false,
    "video": true
  }
}
```

…and surface the OS-picked bound port back via `getinfo` once the
signaller has started.

`server_port: 0` is the convention for "OS picks a free port"; the
actual bound port is emitted **after** the signaller's
`on-server-started` signal fires (see
[Step 6](./MVP-PHASE-5-STEP-6-live-pipeline-port-handle.md)).

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| `DestinationFamily` enum | `senders/android/src/migration/protocol.rs:126-138` |
| Existing variants (`Rtmp / Udp / LocalFile / LocalPlayback`) | same |
| `DestinationInfo` (consumed by `getinfo`) | `senders/android/src/migration/protocol.rs:151-160` |
| `WhepServerSignaller::on-server-started` signal | `sdk/mirroring_core/src/whep_signaller.rs:7, 349-373` |

### 1.2 Why a new variant (not extending `Rtmp` with a `whep: bool`)

Tempting (one fewer enum variant) but bad: WHEP has no flv mux, no
`location` URI, no AAC audio, and the bound port is **emitted as an
event after the signaller starts**. Modelling it as its own family
keeps the pipeline construction code in
[Step 4](./MVP-PHASE-5-STEP-4-build-live-pipeline.md) linear and the
JSON protocol self-documenting.

### 1.3 Why `server_port: 0` is the default

The WHEP signaller's underlying `TcpListener` binds to port 0 by
default — letting the OS pick any free port. The chosen port is then
emitted via `on-server-started`. Hard-coding any other default value
risks conflicts with other processes; `0` is the safe default.

The `#[serde(default)]` on the field makes the JSON shape
`{"Whep":{}}` equivalent to `{"Whep":{"server_port":0}}`.

### 1.4 Why `bound_port_*` fields live on `DestinationInfo`, not `DestinationFamily`

`DestinationFamily` is **inbound config** (what the user requested).
`DestinationInfo` is **outbound state** (what the runtime observed).
The bound port is observed at runtime; putting it on
`DestinationFamily` would mean it's part of the create-time JSON,
which it isn't. Keep the two concerns separate.

---

## 2. The change

**File:** `senders/android/src/migration/protocol.rs`
(extend `DestinationFamily` at lines 126-138 and `DestinationInfo` at
lines 151-160):

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
    Whep {
        /// `0` = OS-picks-free-port. The bound port is emitted via
        /// `DestinationInfo.bound_port_v4` / `bound_port_v6` after
        /// the signaller starts.
        #[serde(default)]
        server_port: u16,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct DestinationInfo {
    pub family: DestinationFamily,
    pub audio_slot_id: Option<String>,
    pub video_slot_id: Option<String>,
    pub cue_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub state: State,

    // NEW — populated only for `DestinationFamily::Whep`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bound_port_v4: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bound_port_v6: Option<u16>,
}
```

The `#[serde(default)]` + `skip_serializing_if = "Option::is_none"`
combo keeps the wire format backward-compatible for the four existing
non-WHEP variants — they serialize without any `bound_port_*` keys.

---

## 3. Verification

### 3.1 Compile check

```bash
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android
```

After Step 1 alone, the change is purely additive at the type level
— `cargo check` should succeed. Any existing `match` block over
`DestinationFamily` that was previously exhaustive will now fail
with `non-exhaustive patterns: 'DestinationFamily::Whep { ... }' not
covered`. Those are addressed in **Step 2** (`from_family`) and
**Step 4** (`build_live_pipeline`).

**Preferred:** land Steps 1+2+3+4 together as a single commit to
keep the tree compiling clean.

### 3.2 Serde shape

Drop into the existing protocol-test module
(`senders/android/src/migration/protocol.rs::tests`):

```rust
#[test]
fn whep_destination_serdes_roundtrip() {
    let original = DestinationFamily::Whep { server_port: 0 };
    let json = serde_json::to_string(&original).unwrap();
    let parsed: DestinationFamily = serde_json::from_str(&json).unwrap();
    assert_eq!(original, parsed);

    // Wire format is the externally-tagged enum shape.
    assert!(json.starts_with(r#"{"Whep":{"server_port":0"#));
}

#[test]
fn whep_destination_default_server_port_when_omitted() {
    let minimal: DestinationFamily =
        serde_json::from_str(r#"{"Whep":{}}"#).unwrap();
    if let DestinationFamily::Whep { server_port } = minimal {
        assert_eq!(server_port, 0);
    } else {
        panic!("expected Whep variant");
    }
}

#[test]
fn destination_info_bound_ports_skipped_when_none() {
    let info = DestinationInfo {
        family: DestinationFamily::LocalPlayback,
        audio_slot_id: None,
        video_slot_id: None,
        cue_time: None,
        end_time: None,
        state: State::Initial,
        bound_port_v4: None,
        bound_port_v6: None,
    };
    let json = serde_json::to_string(&info).unwrap();
    // Optional fields are omitted, not emitted as null.
    assert!(!json.contains("bound_port"));
}
```

All three green after Step 1.

### 3.3 Grep recipe

```bash
grep -nA2 'DestinationFamily::Whep' senders/android/src/migration/protocol.rs
# → expect: one match (the enum arm) with `server_port: u16`.

grep -nA1 'bound_port' senders/android/src/migration/protocol.rs
# → expect: two matches inside DestinationInfo (v4 + v6).
```

---

## 4. Pitfalls specific to this step

### S1-P1 — Forgetting `#[serde(default)]` on `server_port`

If you write only `server_port: u16` without the attribute, inbound
JSON `{"Whep":{}}` fails to deserialize with `missing field 'server_port'`.
The field has no `Option<...>` wrapper because `0` is the meaningful
default (OS picks a free port), but `#[serde(default)]` is still
required to make the JSON ergonomic.

### S1-P2 — Adding `bound_port_*` to `DestinationFamily` instead of `DestinationInfo`

These fields are **observed at runtime**, not requested at creation
time. If they live on `DestinationFamily`, then:

1. The `Hash` derive on `DestinationFamily` (used by some test
   assertions) breaks when the bound port changes.
2. Inbound JSON has a field that the user must specify but isn't a
   meaningful input.
3. Two destinations with the same `server_port: 0` but different
   actual bound ports are not equal — silly.

Keep `bound_port_*` on `DestinationInfo` only.

### S1-P3 — Choosing `u32` for `server_port`

`u16` is the right type — ports are 16-bit. The signaller emits the
bound port as a `u32` (GLib `g_signal_emit` convention) but the
**inbound** field is `u16`. Cast at the signal-handler boundary
([Step 4](./MVP-PHASE-5-STEP-4-build-live-pipeline.md) §2 shows the
`as u16` cast).

### S1-P4 — Skipping `Hash + Eq` because of `u16`

`u16` is `Hash + Eq`. The derives on `DestinationFamily` work
unchanged. (`String` and `Option<u32>` in the other variants are
also `Hash + Eq`, so the trait bound is uniform across the enum.)

---

## 5. Next step

After this lands, [Step 2 — extend `DestinationPipelineProfile::from_family`](./MVP-PHASE-5-STEP-2-pipeline-profile.md)
adds the diagnostic element-listing arm so `getinfo` introspection
shows the placeholder factory names for the WHEP pipeline.
