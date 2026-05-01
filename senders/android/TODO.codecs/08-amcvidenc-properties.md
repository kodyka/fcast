# Task 8 — Handle `amcvidenc` bitrate and keyframe properties

- **Priority:** P2 (Medium — robustness and quality)
- **Source:** [../TODO.codecs.md §8](../TODO.codecs.md)
- **File:** `senders/android/src/migration/nodes/destination.rs`
- **Function:** `select_video_encoder()` and callers

## Tasks

- [ ] After selecting `amcvidenc`, probe for available properties before
      setting:
    - `bitrate` — some variants use bps, others kbps
    - `i-frame-interval` — Android equivalent of `key-int-max`
    - **NOT** `tune` or `zerolatency` — these are x264-specific and do not
      exist on `amcvidenc`
- [ ] Replace the current unconditional property sets:

  ```rust
  // CURRENT (will warn/fail on amcvidenc)
  if venc.has_property("tune") { venc.set_property_from_str("tune", "zerolatency"); }
  if venc.has_property("key-int-max") { venc.set_property("key-int-max", 30u32); }

  // TODO: Add amcvidenc-specific path
  if venc.has_property("i-frame-interval") { venc.set_property("i-frame-interval", 1i32); }
  ```

## Related tasks

- [01 — H.264 encoder fallback chain](01-h264-encoder-fallback-chain.md)
- [04 — videoconvert color-space negotiation](04-videoconvert-color-space.md)
- [07 — rank-based encoder discovery](07-rank-based-encoder-discovery.md)
