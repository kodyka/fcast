# Task 16 — Set up cross-compilation pipeline for `gst-plugins-rs`

- **Priority:** P3 (Low — optimization and future-proofing)
- **Source:** [../TODO.codecs.md §16](../TODO.codecs.md)

## Tasks

- [ ] Document Cerbero-based build for the Android `aarch64-linux-android`
      target.
- [ ] Set up `PKG_CONFIG_SYSROOT_DIR` and `PKG_CONFIG_PATH` pointing to the
      NDK / Cerbero sysroot.
- [ ] Use `cargo-c` to produce `.a` static libraries for: `fallbackswitch`,
      `rsrtmp`, `rswebrtc`.
- [ ] Integrate static libs into `CMakeLists.txt` alongside the prebuilt
      GStreamer.
- [ ] Add a CI job that cross-compiles and verifies linkage.
- [ ] Pin a specific `gst-plugins-rs` commit/tag matching the prebuilt SDK
      version (e.g., 1.24.x).

## Related tasks

- [02 — `rtmp2sink` absence](02-rtmp2sink-absence.md)
- [06 — Rust plugin JNI registration](06-rust-plugin-jni-registration.md)
- [12 — WHIP as RTMP replacement](12-whip-as-rtmp-replacement.md)
- [14 — APK size optimization](14-apk-size-static-linking.md)
