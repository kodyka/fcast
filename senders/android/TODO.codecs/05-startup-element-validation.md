# Task 5 — Validate all GStreamer elements exist at startup

- **Priority:** P1 (High — functional gap, will be hit in production)
- **Source:** [../TODO.codecs.md §5](../TODO.codecs.md)
- **File:** `senders/android/src/migration/runtime.rs`
- **Function:** `start_graph_runtime()`

## Problem

Currently no validation that required elements are in the registry.

## Tasks

- [ ] Add a startup probe that checks for critical elements and logs
      availability, e.g.:
  ```
  compositor yes  audiomixer yes  amcvidenc yes  rtmp2sink no  fallbacksrc no
  ```
- [ ] Expose element availability via a `GET /health` endpoint so external
      tools can check.
- [ ] Fail fast with a descriptive error if absolute-minimum elements are
      missing (`appsrc`, `appsink`, `videoconvert`, `audioconvert`).

## Related tasks

- [17 — element availability integration tests](17-element-availability-tests.md)
