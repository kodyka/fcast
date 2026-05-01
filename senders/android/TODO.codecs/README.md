# GStreamer Android Prebuilt SDK Compatibility — Task Index

This directory contains the per-task breakdown of
[`../TODO.codecs.md`](../TODO.codecs.md). Each task in the original document
has been split into its own TODO file so it can be worked on, assigned, and
tracked independently.

The source-of-truth task descriptions and code snippets remain in
`../TODO.codecs.md`; the files here are scoped, self-contained checklists for
each individual unit of work.

All tasks relate to the migration code under
[`../src/migration/`](../src/migration/) — primarily the nodes in
`src/migration/nodes/` (`destination.rs`, `source.rs`, `mixer.rs`,
`video_generator.rs`) and the runtime in `src/migration/runtime.rs`.

## Summary Matrix

| ID | Priority | Component                        | File |
|----|----------|----------------------------------|------|
|  1 | P0       | H.264 encoder selection          | [01-h264-encoder-fallback-chain.md](01-h264-encoder-fallback-chain.md) |
|  2 | P0       | `rtmp2sink` fallback             | [02-rtmp2sink-absence.md](02-rtmp2sink-absence.md) |
|  3 | P0       | `fallbacksrc` hardening          | [03-fallbacksrc-absence.md](03-fallbacksrc-absence.md) |
|  4 | P1       | Color space for `amcvidenc`      | [04-videoconvert-color-space.md](04-videoconvert-color-space.md) |
|  5 | P1       | Startup element validation       | [05-startup-element-validation.md](05-startup-element-validation.md) |
|  6 | P1       | Rust plugin JNI registration     | [06-rust-plugin-jni-registration.md](06-rust-plugin-jni-registration.md) |
|  7 | P2       | Rank-based discovery             | [07-rank-based-encoder-discovery.md](07-rank-based-encoder-discovery.md) |
|  8 | P2       | `amcvidenc` property handling    | [08-amcvidenc-properties.md](08-amcvidenc-properties.md) |
|  9 | P2       | Optional timecode elements       | [09-timecodestamper-timeoverlay.md](09-timecodestamper-timeoverlay.md) |
| 10 | P2       | Stream reconnection              | [10-uridecodebin-reconnection.md](10-uridecodebin-reconnection.md) |
| 11 | P2       | Deinterlace optimization         | [11-deinterlace-audit.md](11-deinterlace-audit.md) |
| 12 | P3       | WHIP destination                 | [12-whip-as-rtmp-replacement.md](12-whip-as-rtmp-replacement.md) |
| 13 | P3       | Zero-copy frames                 | [13-zero-copy-video-frame-path.md](13-zero-copy-video-frame-path.md) |
| 14 | P3       | APK size optimization            | [14-apk-size-static-linking.md](14-apk-size-static-linking.md) |
| 15 | P3       | `cfg(target_os = "android")` gating | [15-cfg-target-os-android-gating.md](15-cfg-target-os-android-gating.md) |
| 16 | P3       | Cross-compilation pipeline       | [16-gst-plugins-rs-cross-compile.md](16-gst-plugins-rs-cross-compile.md) |
| 17 | P3       | Element availability tests       | [17-element-availability-tests.md](17-element-availability-tests.md) |

## Priority Legend

- **P0** — Critical: app crashes or features completely broken on Android.
- **P1** — High: functional gaps that will be hit in production.
- **P2** — Medium: robustness and quality improvements.
- **P3** — Low: optimizations and future-proofing.
