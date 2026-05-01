# Task 15 — Add `#[cfg(target_os = "android")]` gating throughout

- **Priority:** P3 (Low — optimization and future-proofing)
- **Source:** [../TODO.codecs.md §15](../TODO.codecs.md)
- **Files:** all node files (`senders/android/src/migration/nodes/*.rs`)

## Tasks

- [ ] Gate `amcvidenc` encoder paths with `#[cfg(target_os = "android")]`.
- [ ] Gate `rtmpsink` legacy fallback with `#[cfg(target_os = "android")]`.
- [ ] Keep the desktop encoder chain (`x264enc`, etc.) for
      `#[cfg(not(target_os = "android"))]`.
- [ ] Ensure `cargo check` and `cargo test` still pass on host (x86_64
      Linux) without the Android SDK.

## Related tasks

- [01 — H.264 encoder fallback chain](01-h264-encoder-fallback-chain.md)
- [02 — `rtmp2sink` absence](02-rtmp2sink-absence.md)
- [04 — videoconvert color-space negotiation](04-videoconvert-color-space.md)
- [11 — deinterlace audit](11-deinterlace-audit.md)
