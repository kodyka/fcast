# Task 14 — Minimize APK size from static Rust linking

- **Priority:** P3 (Low — optimization and future-proofing)
- **Source:** [../TODO.codecs.md §14](../TODO.codecs.md)

## Tasks

- [ ] Enable LTO (`lto = true`) in the release profile for `android-sender`.
- [ ] Add `strip = "symbols"` to the Cargo release profile.
- [ ] Audit `codegen-units = 1` for maximum optimization.
- [ ] If cross-compiling `gst-plugins-rs`, build only the specific crates
      needed (do not pull the entire workspace).
- [ ] Measure `.so` size delta before/after adding each Rust plugin.

## Related tasks

- [16 — `gst-plugins-rs` cross-compilation pipeline](16-gst-plugins-rs-cross-compile.md)
