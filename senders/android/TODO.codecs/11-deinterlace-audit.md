# Task 11 — Audit `deinterlace` element usage

- **Priority:** P2 (Medium — robustness and quality)
- **Source:** [../TODO.codecs.md §11](../TODO.codecs.md)
- **Files:**
  - `senders/android/src/migration/nodes/source.rs`
  - `senders/android/src/migration/nodes/video_generator.rs`

## Tasks

- [ ] Verify `deinterlace` is in the prebuilt Android SDK (it is in
      `gst-plugins-good`, so should be present).
- [ ] On Android, most content is progressive — `deinterlace` adds
      unnecessary latency.
- [ ] Make `deinterlace` conditional: skip it on Android unless the input is
      actually interlaced.
- [ ] Probe input caps for `interlace-mode` before inserting the deinterlace
      element.

## Related tasks

- [15 — `cfg(target_os = "android")` gating](15-cfg-target-os-android-gating.md)
