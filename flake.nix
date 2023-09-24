# flake.nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, rust-overlay, }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      };
      toolchain = pkgs.rust-bin.fromRustupToolchainFile ./toolchain.toml;
    in {
      devShells.${system}.default = let
        build-deps = with pkgs; [
          pkg-config
          udev
          alsa-lib
          vulkan-loader
          # x11 dpendencies
          xorg.libX11
          xorg.libXcursor
          xorg.libXi
          xorg.libXrandr
          # Wayland dependencies
          libxkbcommon
          wayland
        ];
      in pkgs.mkShell.override {
        stdenv = pkgs.stdenvAdapters.useMoldLinker pkgs.stdenv;
      } {
        packages = build-deps ++ [
          toolchain
          pkgs.cargo-bloat
          pkgs.cargo-unused-features
          pkgs.rust-analyzer-unwrapped
          pkgs.cargo-watch
          pkgs.cargo-sort
          pkgs.pre-commit
        ];

        RUST_SRC_PATH = "${toolchain}/lib/rustlib/src/rust/library";
        NIX_LD = pkgs.runCommand "ld.so" { } ''
          ln -s "$(cat '${pkgs.stdenv.cc}/nix-support/dynamic-linker')" $out
        '';
        LD_LIBRARY_PATH =
          "${pkgs.lib.makeLibraryPath build-deps}:/run/opengl-driver/lib/:${
            pkgs.lib.makeLibraryPath ([ pkgs.libGL pkgs.libGLU ])
          }";
      };
    };
}
