# Task 6 — Register Rust-compiled GStreamer plugins via JNI

- **Priority:** P1 (High — functional gap, will be hit in production)
- **Source:** [../TODO.codecs.md §6](../TODO.codecs.md)
- **File:** new file needed — e.g. `senders/android/src/migration/gst_registration.rs`

## Problem

If any `gst-plugins-rs` elements are cross-compiled and statically linked,
they bypass the automatic `GST_PLUGIN_STATIC_REGISTER` sequence in the
prebuilt `libgstreamer_android.so`.

## Tasks

- [ ] Create explicit plugin registration calls for any statically linked
      Rust plugins.
- [ ] Call registration before `gst_init()` or immediately after, in
      `android_main`.
- [ ] Document the required `GST_PLUGIN_STATIC_DECLARE` /
      `GST_PLUGIN_STATIC_REGISTER` pattern for each Rust plugin added.
- [ ] Add build-time feature flags:
      `feature = "gst-rs-fallbacksrc"`, `feature = "gst-rs-rtmp2"`, etc.

## Related tasks

- [02 — `rtmp2sink` absence](02-rtmp2sink-absence.md)
- [03 — `fallbacksrc` absence](03-fallbacksrc-absence.md)
- [16 — `gst-plugins-rs` cross-compilation pipeline](16-gst-plugins-rs-cross-compile.md)
