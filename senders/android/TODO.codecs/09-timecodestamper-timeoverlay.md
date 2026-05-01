# Task 9 — Handle `timecodestamper` / `timeoverlay` availability

- **Priority:** P2 (Medium — robustness and quality)
- **Source:** [../TODO.codecs.md §9](../TODO.codecs.md)
- **File:** `senders/android/src/migration/nodes/destination.rs`
- **Section:** RTMP arm

## Tasks

- [ ] Make `timecodestamper` and `timeoverlay` optional in the RTMP chain —
      skip them if unavailable.
- [ ] These elements are in `gst-plugins-bad`, which is in the prebuilt
      Android SDK, but verify they are present in the Android subset.
- [ ] If unavailable, link directly: `appsrc -> videoconvert -> encoder`
      (skip timecode insertion).

## Related tasks

- [02 — `rtmp2sink` absence](02-rtmp2sink-absence.md)
- [05 — startup element validation](05-startup-element-validation.md)
