# Task 3 — Handle `fallbacksrc` absence gracefully

- **Priority:** P0 (Critical — app crashes / features completely broken)
- **Source:** [../TODO.codecs.md §3](../TODO.codecs.md)
- **File:** `senders/android/src/migration/nodes/source.rs`
- **Function:** `build_source_element()`
- **Approx. lines:** ~80-105

## Status

Already has fallback logic — needs hardening.

## Tasks

- [ ] Verify `uridecodebin` fallback path works end-to-end on Android
      (dynamic pad linking with `connect_pad_added`).
- [ ] When using `uridecodebin` fallback, implement manual reconnection logic
      on pipeline bus errors (the app loses automatic recovery that
      `fallbacksrc` provides).
- [ ] Add a periodic health-check in `poll_bus_messages()` that detects
      stream stalls and triggers pipeline rebuild.
- [ ] Consider `uridecodebin3` as a middle-ground fallback (better gap
      handling than `uridecodebin`, available in prebuilt).
- [ ] Audit the fallbacksrc-only property sets (`manual_unblock`,
      `immediate_fallback`) and keep them strictly inside the fallbacksrc
      code path. The current code already scopes them to fallbacksrc — this
      is a regression guard.

## Related tasks

- [10 — `uridecodebin` reconnection](10-uridecodebin-reconnection.md)
