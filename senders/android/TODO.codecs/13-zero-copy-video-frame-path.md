# Task 13 — Implement zero-copy video frame path

- **Priority:** P3 (Low — optimization and future-proofing)
- **Source:** [../TODO.codecs.md §13](../TODO.codecs.md)
- **File:** `senders/android/src/lib.rs`
- **Function:** `process_frame()`

## Tasks

- [ ] Current path copies Y/U/V planes row-by-row from a Java `ByteBuffer`
      into a GStreamer buffer pool.
- [ ] Investigate `GstGLMemory` + `VideoFrameGLExt` to map Android
      `HardwareBuffer` directly to GL textures.
- [ ] This eliminates CPU-side pixel copies entirely, reducing thermal
      throttling.
- [ ] Requires changes on the Java side: use `ImageReader` with the
      `USAGE_GPU_SAMPLED_IMAGE` flag.
- [ ] Only viable if downstream consumers (encoder) support GL input —
      `amcvidenc` does on most devices.

## Related tasks

- [01 — H.264 encoder fallback chain](01-h264-encoder-fallback-chain.md)
- [04 — videoconvert color-space negotiation](04-videoconvert-color-space.md)
