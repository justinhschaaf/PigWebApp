{
    description = "eframe devShell";

    inputs = {
        nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
        rust-overlay.url = "github:oxalica/rust-overlay";
        flake-utils.url = "github:numtide/flake-utils";
    };

    outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
        let
            overlays = [ (import rust-overlay) ];
            pkgs = import nixpkgs { inherit system overlays; };
            rust = pkgs.rust-bin.stable.latest.default.override {
                extensions = [ "rust-src" ];
                targets = [ "wasm32-unknown-unknown" "x86_64-unknown-linux-gnu" ];
            };
        in with pkgs; {
            devShells.default = mkShell rec {
                buildInputs = [
                    # rust
                    rust

                    # rust tools
                    cargo-make # build tool
                    cargo-watch # cargo-make's watch feature uses this
                    rustfmt # code formatting
                    trunk # WASM compilation for the client module

                    # misc. tools
                    tmux

                    # misc. libraries
                    openssl

                    # GUI libs
                    libxkbcommon
                    libGL
                    fontconfig
                ];

                LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
                RUST_SRC_PATH = "${rustPlatform.rustLibSrc}"; # https://wiki.nixos.org/wiki/Rust#Shell.nix_example

            };
        });
}
