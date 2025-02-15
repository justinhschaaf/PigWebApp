{
    description = "PigWebApp";

    inputs = {
        nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
        crane.url = "github:ipetkov/crane";
        rust-overlay.url = "github:oxalica/rust-overlay";
        flake-utils.url = "github:numtide/flake-utils";
    };

    outputs = { nixpkgs, flake-utils, ... }@inputs: {
        # we don't need to replicate this per system
        nixosModules.default = { ... }: {
            imports = [ ./module.nix ];
        };
    } // flake-utils.lib.eachDefaultSystem (system:
        let
            # Setup pkgs and Rust overlay
            overlays = [ (import inputs.rust-overlay) ];
            pkgs = import nixpkgs { inherit system overlays; };
            rust = pkgs.rust-bin.stable.latest.default.override {
                extensions = [ "rust-src" ];
                targets = [ "wasm32-unknown-unknown" "x86_64-unknown-linux-gnu" ];
            };

            # CRANE BUILD SYSTEM
            # To make the code look like spaghetti, Crane recommends defining
            # most of the parameters for your packages ahead of time in a let
            # statement like this. You then have to use those parameters to
            # precompile the dependencies for each package for caching.
            #
            # https://crane.dev/examples/trunk-workspace.html
            # https://crane.dev/examples/custom-toolchain.html
            craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rust;

            # Base options used by both client and server packages
            basePkgArgs = {
                # Pull the version from the parent Cargo.toml
                # https://m7.rs/blog/2022-11-01-package-a-rust-app-with-nix/index.html
                version = (pkgs.lib.importTOML ./Cargo.toml).workspace.package.version;

                # "My source is I made it the fuck up!"
                #
                # Using the nixpkgs clean function instead of Crane's fixes
                # "ERROR error getting the canonical path to the build target HTML file"
                src = pkgs.lib.cleanSource ./.;

                # This is set for some reason idfk what it does
                strictDeps = true;

                # By default, checks are enabled for Cargo builds. Disable them.
                doCheck = false;

                # Mostly needed for Trunk, compile in release mode
                CARGO_PROFILE = "release";
            };

            # Various client-specific options
            clientPkgArgs = basePkgArgs // {
                pname = (pkgs.lib.importTOML ./client/Cargo.toml).package.name;

                # Needed to tell Cargo to build dependencies for WASM
                CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
                cargoExtraArgs = "--package=pigweb_client";

                # Needed by egui, probably
                buildInputs = with pkgs; [
                    openssl
                    libxkbcommon
                    libGL
                    fontconfig
                ];

                # "this version must match EXACTLY the one defined in Cargo.lock"
                # why tf do i have to *manually* define this?
                wasm-bindgen-cli = pkgs.wasm-bindgen-cli.override {
                    version = "0.2.93";
                    # To find the hashes on a new version, set both to
                    # pkgs.lib.fakeHash and run a build. The first hash you get
                    # will be the hash. Run it again to get the cargoHash
                    hash = "sha256-DDdu5mM3gneraM85pAepBXWn3TMofarVR4NbjMdz3r0=";
                    cargoHash = "sha256-birrg+XABBHHKJxfTKAMSlmTVYLmnmqMDfRnmG6g/YQ=";
                };
            };

            # Server-specific options
            serverPkgArgs = basePkgArgs // {
                pname = (pkgs.lib.importTOML ./server/Cargo.toml).package.name;
                cargoExtraArgs = "--package=pigweb_server";
            };

            # Build the dependencies to cache them ahead of time
            clientDeps = craneLib.buildDepsOnly clientPkgArgs;
            serverDeps = craneLib.buildDepsOnly serverPkgArgs;
        in with pkgs; {
            devShells.default = mkShell rec {
                buildInputs = [
                    # rust
                    rust

                    # rust tools
                    cargo-make # build tool
                    cargo-watch # cargo-make's watch feature uses this
                    diesel-cli # diesel.rs doesn't tell you how to use it without this
                    rustfmt # code formatting
                    trunk # WASM compilation for the client module

                    # database, we need it installed here for the dev server
                    postgresql

                    # misc. tools
                    tmux

                    # misc. libraries
                    openssl

                    # GUI libs
                    libxkbcommon
                    libGL
                    fontconfig
                ];

                # Library path environment variables
                LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
                RUST_SRC_PATH = "${rustPlatform.rustLibSrc}"; # https://wiki.nixos.org/wiki/Rust#Shell.nix_example

                # Cargo Make: let workspace members inherit parent makefile
                # https://github.com/sagiegurari/cargo-make?tab=readme-ov-file#automatically-extend-workspace-makefile
                CARGO_MAKE_EXTEND_WORKSPACE_MAKEFILE = true;

                # Cargo Make: don't bother trying to compile the common member on its own
                CARGO_MAKE_WORKSPACE_SKIP_MEMBERS = "common";

                # Env vars for test runs, paths should be relative to ./run
                PIGWEB_CONFIG = "./PigWeb.toml";
                PIGWEB_CLIENT_PATH = "../client/dist";
                PIGWEB_DATABASE_URI = "${DATABASE_URL}";
                ROCKET_CONFIG = "./Rocket.toml";

                # Postgres vars, commands are run by cargo-make
                # https://zeroes.dev/p/nix-recipe-for-postgresql/
                # https://mgdm.net/weblog/postgresql-in-a-nix-shell/
                PGHOST = "/tmp"; # where the unix socket is located
                PGDATA = "./db"; # data dir, relative to ./run
                PGUSER = "pigweb"; # db user to run commands through
                PGDATABASE = "${PGUSER}"; # the name of the postgres database

                # database location for diesel-cli
                DATABASE_URL = "postgres://${PGUSER}@localhost/${PGDATABASE}";
            };

            # The settings here are only really important for Trunk, Cargo ones set above
            packages.pigweb_client = craneLib.buildTrunkPackage (clientPkgArgs // {
                cargoArtifacts = clientDeps;

                # Trunk expects the current directory to be the crate to compile
                # Fixes "ERROR could not find the root package of the target crate"
                preBuild = ''
                    cd ./client
                '';

                # After building, move the `dist` artifacts and restore the working directory
                postBuild = ''
                    mv ./dist ..
                    cd ..
                '';
            });

            packages.pigweb_server = craneLib.buildPackage (serverPkgArgs // {
                cargoArtifacts = serverDeps;
            });
        });
}
