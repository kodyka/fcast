# Task 7 — Implement rank-based encoder/decoder discovery

- **Priority:** P2 (Medium — robustness and quality)
- **Source:** [../TODO.codecs.md §7](../TODO.codecs.md)
- **File:** `senders/android/src/migration/nodes/destination.rs`

## Tasks

- [ ] Replace hardcoded element-name lists with `gst::ElementFactory`
      registry queries.
- [ ] Query by caps: `"video/x-h264"` on src pad for encoders.
- [ ] Sort by `gst::Rank` descending — Android will naturally surface
      `amcvidenc` variants with `Rank::Primary`.
- [ ] Cache discovery results per-session (the element registry does not
      change at runtime).
- [ ] Log the full list of discovered encoders at startup for debugging
      device-specific issues.

## Suggested snippet

```rust
fn discover_h264_encoder() -> Result<gst::Element, String> {
    let factories = gst::ElementFactory::list_get_elements(
        gst::ElementFactoryListType::ENCODER | gst::ElementFactoryListType::MEDIA_VIDEO,
        gst::Rank::MARGINAL,
    );
    for factory in factories {
        if factory.can_src_any_caps(&gst::Caps::builder("video/x-h264").build()) {
            if let Ok(elem) = factory.create().name("destination-venc").build() {
                return Ok(elem);
            }
        }
    }
    Err("No H.264 encoder available".into())
}
```

## Related tasks

- [01 — H.264 encoder fallback chain](01-h264-encoder-fallback-chain.md)
- [08 — `amcvidenc` property handling](08-amcvidenc-properties.md)
