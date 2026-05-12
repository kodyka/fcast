# MVP-PHASE-8 — Step 5: unit tests

> Part 5 of 6. Parent doc: [`MVP-PHASE-8-srt-destination-family.md`](./MVP-PHASE-8-srt-destination-family.md).
> Previous: [Step 4 — bundle the SRT plugin in Android.mk](./MVP-PHASE-8-STEP-4-android-makefile.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

Add the host-runnable unit tests that validate all of Steps 1–4
together without requiring GStreamer initialisation or the SRT plugin
to be available at test time:

| Test | Validates |
|---|---|
| `create_srt_destination_succeeds` | `NodeManager::dispatch` accepts `CreateDestination` with the new `Srt` family (Steps 1 + 2). |
| `srt_destination_with_encryption_serdes_roundtrip` | Wire format round-trips for all four fields (Step 1). |
| `srt_destination_optional_fields_omitted_in_minimal_json` | Optional fields default to `None` (Step 1's `#[serde(default, skip_serializing_if = …)]`). |
| `srt_destination_passphrase_requires_pbkeylen` | Validator rejects `passphrase` without `pbkeylen` (Step 1's protocol constraint). |
| `srt_profile_lists_srtsink_and_mpegtsmux` | `from_family` returns the correct element list (Step 2). |
| `srt_profile_filters_audio_when_disabled` | Element retention works for the `Srt` variant (Step 2). |
| `srt_profile_filters_video_when_disabled` | Same, for video. |

Plus one documentation-only test:

| Test | Validates |
|---|---|
| `create_source_accepts_srt_uri` | `SourceNode` accepts `srt://` URIs without any code change (Step 6 covers this). |

All tests run as `cargo test` on a Linux/macOS dev host — **no
Android emulator, no GStreamer registry, no SRT plugin** required.
Pipeline-level smoke is in
[Step 3](./MVP-PHASE-8-STEP-3-build-live-pipeline.md) §3.3.

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| Existing `NodeManager` tests | `senders/android/src/migration/node_manager.rs` — search for `#[cfg(test)] mod tests` |
| Existing `protocol.rs` tests | `senders/android/src/migration/protocol.rs` — search for `#[cfg(test)] mod tests` |
| Existing `nodes/destination.rs` tests | `senders/android/src/migration/nodes/destination.rs` — search for `#[cfg(test)] mod tests` |
| `serde_json` is already in `[dev-dependencies]` | `senders/android/Cargo.toml` |

### 1.2 Where each test lives

Match the existing test placement convention:

| Test | Module | Reasoning |
|---|---|---|
| `create_srt_destination_succeeds` | `node_manager.rs` | Tests dispatch via `NodeManager::dispatch(Command::CreateDestination)`. |
| `srt_destination_*_serdes_*` | `protocol.rs` | Validates serde shape of `DestinationFamily`. |
| `srt_destination_passphrase_requires_pbkeylen` | `node_manager.rs` | If implemented as a dispatch-time check; or in `protocol.rs` if validated at deserialization. |
| `srt_profile_*` | `nodes/destination.rs` | Tests `DestinationPipelineProfile::from_family`. |
| `create_source_accepts_srt_uri` | `node_manager.rs` | Tests `Command::CreateSource` dispatch with `srt://` URI. |

---

## 2. The change

### 2.1 In `senders/android/src/migration/node_manager.rs`

Add to the existing `#[cfg(test)] mod tests` block:

```rust
#[cfg(test)]
mod srt_destination_tests {
    use super::*;
    use crate::migration::protocol::{Command, DestinationFamily, CommandResult};

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
        assert!(matches!(result, CommandResult::Success), "{result:?}");
        assert!(manager.nodes.contains_key("srt-out-1"));
    }

    #[test]
    fn create_srt_destination_audio_only_succeeds() {
        let mut manager = NodeManager::default();
        let result = manager.dispatch(Command::CreateDestination {
            id: "srt-audio".into(),
            family: DestinationFamily::Srt {
                uri: "srt://example.com:1234".into(),
                latency: None,
                passphrase: None,
                pbkeylen: None,
            },
            audio: true,
            video: false,
        });
        assert!(matches!(result, CommandResult::Success), "{result:?}");
    }

    #[test]
    fn create_srt_destination_video_only_succeeds() {
        let mut manager = NodeManager::default();
        let result = manager.dispatch(Command::CreateDestination {
            id: "srt-video".into(),
            family: DestinationFamily::Srt {
                uri: "srt://example.com:1234".into(),
                latency: None,
                passphrase: None,
                pbkeylen: None,
            },
            audio: false,
            video: true,
        });
        assert!(matches!(result, CommandResult::Success), "{result:?}");
    }

    #[test]
    fn create_srt_destination_rejects_neither_audio_nor_video() {
        let mut manager = NodeManager::default();
        let result = manager.dispatch(Command::CreateDestination {
            id: "srt-empty".into(),
            family: DestinationFamily::Srt {
                uri: "srt://example.com:1234".into(),
                latency: None,
                passphrase: None,
                pbkeylen: None,
            },
            audio: false,
            video: false,
        });
        // Existing validation rejects (audio=false, video=false) for all families.
        assert!(matches!(result, CommandResult::Error(_)), "{result:?}");
    }

    #[test]
    fn create_srt_destination_passphrase_requires_pbkeylen() {
        // Optional: only if the Step 3 dispatch-time validator is in place.
        let mut manager = NodeManager::default();
        let result = manager.dispatch(Command::CreateDestination {
            id: "srt-bad-encryption".into(),
            family: DestinationFamily::Srt {
                uri: "srt://example.com:1234".into(),
                latency: None,
                passphrase: Some("0123456789".into()),
                pbkeylen: None, // ← missing, should be rejected
            },
            audio: true,
            video: true,
        });
        assert!(matches!(result, CommandResult::Error(_)), "{result:?}");
    }

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
}
```

### 2.2 In `senders/android/src/migration/protocol.rs`

Add to the existing `#[cfg(test)] mod tests` block:

```rust
#[cfg(test)]
mod srt_protocol_tests {
    use super::*;

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

    #[test]
    fn srt_destination_minimal_round_trip_omits_optional_fields() {
        let minimal = DestinationFamily::Srt {
            uri: "srt://h:1".into(),
            latency: None,
            passphrase: None,
            pbkeylen: None,
        };
        let json = serde_json::to_string(&minimal).unwrap();
        // Should serialize to a compact `{"Srt":{"uri":"srt://h:1"}}` shape —
        // optional fields are skipped, not emitted as null.
        assert_eq!(json, r#"{"Srt":{"uri":"srt://h:1"}}"#);
    }

    #[test]
    fn srt_destination_wire_format_uses_externally_tagged_enum() {
        let original = DestinationFamily::Srt {
            uri: "srt://h:1".into(),
            latency: Some(200),
            passphrase: None,
            pbkeylen: None,
        };
        let json = serde_json::to_string(&original).unwrap();
        // Wire format matches the other variants (externally-tagged enum).
        assert!(json.starts_with(r#"{"Srt":{"#));
        assert!(json.contains(r#""uri":"srt://h:1""#));
        assert!(json.contains(r#""latency":200"#));
    }
}
```

### 2.3 In `senders/android/src/migration/nodes/destination.rs`

Add to the existing `#[cfg(test)] mod tests` block:

```rust
#[cfg(test)]
mod srt_profile_tests {
    use super::*;
    use crate::migration::protocol::DestinationFamily;

    fn srt_family() -> DestinationFamily {
        DestinationFamily::Srt {
            uri: "srt://example.com:1234".into(),
            latency: Some(200),
            passphrase: None,
            pbkeylen: None,
        }
    }

    #[test]
    fn srt_profile_lists_srtsink_and_mpegtsmux() {
        let profile = DestinationPipelineProfile::from_family(&srt_family(), true, true);
        assert!(profile.elements.iter().any(|el| el == "srtsink"));
        assert!(profile.elements.iter().any(|el| el == "mpegtsmux"));
        assert!(profile.elements.iter().any(|el| el == "h264enc"));
        assert!(profile.elements.iter().any(|el| el == "avenc_aac"));
    }

    #[test]
    fn srt_profile_filters_audio_when_disabled() {
        let profile = DestinationPipelineProfile::from_family(&srt_family(), false, true);
        assert!(!profile.elements.iter().any(|el| el == "audioconvert"));
        assert!(!profile.elements.iter().any(|el| el == "audioresample"));
        assert!(!profile.elements.iter().any(|el| el == "avenc_aac"));
        // Video-related factories remain.
        assert!(profile.elements.iter().any(|el| el == "h264enc"));
    }

    #[test]
    fn srt_profile_filters_video_when_disabled() {
        let profile = DestinationPipelineProfile::from_family(&srt_family(), true, false);
        assert!(!profile.elements.iter().any(|el| el.contains("video")));
        assert!(!profile.elements.iter().any(|el| el.contains("h264")));
        // mpegtsmux and srtsink remain (neither contains "video" or "h264").
        assert!(profile.elements.iter().any(|el| el == "mpegtsmux"));
        assert!(profile.elements.iter().any(|el| el == "srtsink"));
    }
}
```

---

## 3. Verification

### 3.1 Run the test selectors

```bash
cargo +nightly test -p fcast-sender-android \
    migration::node_manager::srt_destination_tests
```

Expect **5–6 tests** green (depending on whether the dispatch-time
encryption-pairing validator is implemented).

```bash
cargo +nightly test -p fcast-sender-android \
    migration::protocol::srt_protocol_tests
```

Expect **4 tests** green.

```bash
cargo +nightly test -p fcast-sender-android \
    migration::nodes::destination::srt_profile_tests
```

Expect **3 tests** green.

### 3.2 Full test-module sweep

```bash
cargo +nightly test -p fcast-sender-android srt_
```

Names match the prefix `srt_` — should pick up all of the above
plus any future SRT-related tests. Expect **12+ tests** green (5
node_manager + 4 protocol + 3 profile + any future additions).

### 3.3 Grep recipe

```bash
grep -rn 'fn srt_' senders/android/src/migration/ --include='*.rs'
# → expect: ~12 entries (one per test).
```

---

## 4. Pitfalls specific to this step

### S5-P1 — Putting all the tests in one module

Tempting to consolidate into `mod srt_tests` under one of the files,
but it splits concerns:

- `node_manager.rs` tests assert dispatch behaviour and `NodeManager`
  state mutations.
- `protocol.rs` tests assert JSON serde shape.
- `nodes/destination.rs` tests assert the diagnostic
  `DestinationPipelineProfile` listing.

Each module has its own `#[cfg(test)]` block with its own helpers.
**Match the existing convention** — every existing `Destination*`
test follows this layout.

### S5-P2 — Forgetting `serde_json` in `[dev-dependencies]`

`protocol.rs` tests use `serde_json::to_string` / `from_str`. If
`Cargo.toml` lists `serde_json` only under `[dependencies]` and not
`[dev-dependencies]`, host-target builds may still pick it up, but
the **Android target build** (which strips dev-deps) might trip on
something else. Confirm by running:

```bash
grep -A5 '^\[dev-dependencies\]' senders/android/Cargo.toml | grep serde_json
```

If absent, the existing tests would also fail — so this is usually
already in place. Verify before adding the test file.

### S5-P3 — Using `unwrap()` instead of `assert!(matches!(...))`

The `CommandResult` enum is `Success / Error(String) / Info(...)`.
Calling `.unwrap()` on it doesn't compile (it's not a `Result`). The
correct pattern is:

```rust
assert!(matches!(result, CommandResult::Success), "{result:?}");
```

The `"{result:?}"` interpolation in the assertion message prints the
`Error(...)` payload on failure — invaluable when CI logs are the
only diagnostic surface.

### S5-P4 — Asserting on `manager.nodes` cardinality without `clear()`

```rust
assert_eq!(manager.nodes.len(), 1);
```

This is brittle: if `NodeManager::default()` auto-creates some
internal node (e.g. a default mixer), the count is off. **Prefer**
`assert!(manager.nodes.contains_key("srt-out-1"))` — it's
robust to future internal additions.

### S5-P5 — Forgetting to test the `rejects_neither_audio_nor_video` case

Existing `Destination*` tests enforce that **at least one of audio
or video must be true**. The `Srt` variant must respect this
invariant (the validator lives in `node_manager.rs::create_destination`,
not in the family-specific code). Without this test, a regression
that drops the check for `Srt` only would silently pass CI.

### S5-P6 — Testing `passphrase` without `pbkeylen` requires Step 3 to validate

The `passphrase_requires_pbkeylen` test only passes if Step 3's
dispatch-time pair check is implemented. If you skipped that check
(the parent doc lists it as optional), this test fails. Two
options:

1. **Skip the test** (drop it).
2. **Implement the check** in `node_manager.rs::create_destination`
   before the `NodeRecord::Destination(...)` insertion:
   ```rust
   if let DestinationFamily::Srt { passphrase, pbkeylen, .. } = &family {
       if passphrase.is_some() != pbkeylen.is_some() {
           return CommandResult::Error(
               "Srt destination: passphrase and pbkeylen must both be set or both unset".into()
           );
       }
   }
   ```

Pick one and document in the PR.

---

## 5. Next step

After this lands, [STEP 6 — source-side smoke documentation](./MVP-PHASE-8-STEP-6-source-side.md)
documents (with a trivial dispatcher test) that **SRT sources already
work without any source-side code change** — `fallbacksrc` /
`uridecodebin` handle `srt://` URIs natively via the same `srt`
plugin Step 4 bundled. This step is purely informational; the test
it adds prevents a future contributor from adding a redundant arm
to `SourceNode`.
