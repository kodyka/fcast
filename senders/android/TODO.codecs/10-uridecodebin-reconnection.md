# Task 10 — Add stream reconnection for `uridecodebin` fallback path

- **Priority:** P2 (Medium — robustness and quality)
- **Source:** [../TODO.codecs.md §10](../TODO.codecs.md)
- **File:** `senders/android/src/migration/nodes/source.rs`
- **Function:** `poll_bus_messages()`

## Problem

When `fallbacksrc` is unavailable and `uridecodebin` is used, network drops
kill the pipeline permanently.

## Tasks

- [ ] On `gst::MessageType::Error` from `uridecodebin`, check if the error is
      network-related.
- [ ] Implement automatic teardown + rebuild with exponential backoff.
- [ ] Add configurable `max_reconnect_attempts` to `SourcePipelineProfile`.
- [ ] Emit `NodeStatusMessage::Error` to UI/listeners before reconnect
      attempt.
- [ ] Track consecutive failures and transition to `State::Stopped` after
      exhausting retries.

## Related tasks

- [03 — `fallbacksrc` absence](03-fallbacksrc-absence.md)
