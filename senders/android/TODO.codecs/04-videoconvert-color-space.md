# Task 4 — Add `videoconvert` color space negotiation before hardware encoders

- **Priority:** P1 (High — functional gap, will be hit in production)
- **Source:** [../TODO.codecs.md §4](../TODO.codecs.md)
- **File:** `senders/android/src/migration/nodes/destination.rs`
- **All encoder-using arms:** RTMP, UDP, LocalFile

## Problem

Android MediaCodec encoders are extremely strict about input color format.
Desktop encoders accept nearly anything.

## Tasks

- [ ] Ensure `videoconvert` is always placed immediately before the encoder
      element.
- [ ] Add an explicit capsfilter between `videoconvert` and encoder
      constraining to `video/x-raw,format=NV12` on Android.
- [ ] Handle devices where MediaCodec only accepts `COLOR_FormatYUV420Planar`
      (I420) — probe encoder sink-pad caps.
- [ ] Test with frames coming from `appsrc` in I420 format (which is what the
      screen-capture path produces).

## Suggested snippet

```rust
// TODO: Add after videoconvert, before encoder
#[cfg(target_os = "android")]
{
    let caps = gst::Caps::builder("video/x-raw")
        .field("format", "NV12")
        .build();
    let capsfilter = Self::make_element("capsfilter", None)?;
    capsfilter.set_property("caps", &caps);
    // insert into link chain
}
```

## Related tasks

- [01 — H.264 encoder fallback chain](01-h264-encoder-fallback-chain.md)
- [08 — `amcvidenc` property handling](08-amcvidenc-properties.md)
- [15 — `cfg(target_os = "android")` gating](15-cfg-target-os-android-gating.md)
