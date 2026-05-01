# Task 12 — Evaluate WebRTC/WHIP as RTMP replacement

- **Priority:** P3 (Low — optimization and future-proofing)
- **Source:** [../TODO.codecs.md §12](../TODO.codecs.md)
- **File:** `senders/android/src/migration/nodes/destination.rs`

## Tasks

- [ ] Add `DestinationFamily::Whip { endpoint: String }` variant to the
      protocol.
- [ ] Implement a destination pipeline using `whipclientsink` (from
      `gst-plugins-rs`).
- [ ] Note: WHIP over UDP avoids TCP head-of-line blocking on mobile
      networks.
- [ ] If `gst-plugins-rs` cross-compilation is set up for `rtmp2sink`,
      `whipclientsink` comes for free.
- [ ] Consider making WHIP the default output protocol for mobile.

## Related tasks

- [02 — `rtmp2sink` absence](02-rtmp2sink-absence.md)
- [16 — `gst-plugins-rs` cross-compilation pipeline](16-gst-plugins-rs-cross-compile.md)
