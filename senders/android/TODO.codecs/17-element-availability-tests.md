# Task 17 — Add element availability integration tests

- **Priority:** P3 (Low — optimization and future-proofing)
- **Source:** [../TODO.codecs.md §17](../TODO.codecs.md)
- **File:** new — e.g. `senders/android/tests/gst_elements.rs` or extend
  existing tests.

## Tasks

- [ ] Write a test that initializes GStreamer and probes for every element
      used across all node types.
- [ ] Categorize results: `available`, `missing-optional`, `missing-critical`.
- [ ] Run on both host (full desktop GStreamer) and Android (prebuilt SDK)
      via `adb shell`.
- [ ] Elements to probe:

  ```
  appsrc, appsink, compositor, audiomixer, videotestsrc, audiotestsrc,
  videoconvert, audioconvert, audioresample, deinterlace, level,
  timecodestamper, timeoverlay, queue, capsfilter,
  uridecodebin, autovideosink, autoaudiosink,
  flvmux, mpegtsmux, splitmuxsink, udpsink,
  avenc_aac, h264parse, multiqueue,
  fallbacksrc, rtmp2sink, rtmpsink,
  x264enc, openh264enc, nvh264enc,
  amcvidenc-*
  ```

## Related tasks

- [05 — startup element validation](05-startup-element-validation.md)
