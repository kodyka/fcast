{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          overlays = [
            (import rust-overlay)
          ];
          pkgs = import nixpkgs {
            inherit system overlays;
            config = { };
          };
          gst = pkgs.gst_all_1;
          lib = pkgs.lib;
          rustToolchain = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default);
          rustPlatform = pkgs.makeRustPlatform {
            cargo = rustToolchain;
            rustc = rustToolchain;
          };
          commonPkgConfigDeps = [
            gst.gstreamer
            gst.gst-plugins-base
            gst.gst-plugins-good
            gst.gst-plugins-bad
            gst.gst-plugins-ugly
            gst.gst-libav
            pkgs.openssl
            pkgs.glib
            pkgs.pango
            pkgs.cairo
            pkgs.fontconfig
            pkgs.freetype
          ];
          linuxGuiDeps = lib.optionals pkgs.stdenv.hostPlatform.isLinux [
            pkgs.alsa-lib
            pkgs.libpulseaudio
            pkgs.libnice
            pkgs.pipewire
            pkgs.libxkbcommon
            pkgs.wayland
            pkgs.libGL
            pkgs.vulkan-loader
            pkgs.libx11
            pkgs.libxcursor
            pkgs.libxi
            pkgs.libxrandr
            pkgs.libxcb
            pkgs.libxext
          ];
        in
        {
          packages = {
            fcast-sender = pkgs.callPackage ./senders/desktop/fcast-sender.nix { };
            fcast-receiver = pkgs.callPackage ./receivers/experimental/desktop/fcast-receiver.nix {
              inherit rustPlatform;
            };
            default = self.packages.${system}.fcast-sender;
          };

          devShells.default = pkgs.mkShell {
            nativeBuildInputs = with pkgs; [
              rustToolchain
              cargo-ndk
              pkg-config
              libclang
              clang
              android-tools
              wget
              unzip
              gnutar
              gnumake
              jdk17_headless
            ];

            buildInputs = with pkgs; [
              openssl
              glib
              glib-networking
              pango
              cairo
              fontconfig
              freetype
              gst.gstreamer
              gst.gst-plugins-base
              gst.gst-plugins-good
              gst.gst-plugins-bad
              gst.gst-plugins-ugly
              gst.gst-libav
            ] ++ linuxGuiDeps;

            LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
            PKG_CONFIG_ALLOW_CROSS = 1;
            PKG_CONFIG_PATH = pkgs.lib.makeSearchPathOutput "dev" "lib/pkgconfig" (
              commonPkgConfigDeps ++ linuxGuiDeps
            );

            shellHook = ''
              export GIO_EXTRA_MODULES="${pkgs.glib-networking}/lib/gio/modules"
              export RUST_BACKTRACE=1
              echo "fcast dev shell ready"
              echo "Run tests with: cargo test -p android-sender screen_capture"
            '';
          };
        }
      );
}
