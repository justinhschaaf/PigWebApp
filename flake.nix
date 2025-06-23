{
    description = "PigWebApp";

    inputs = {
        nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
        crane.url = "github:ipetkov/crane";
        rust-overlay.url = "github:oxalica/rust-overlay";
        flake-utils.url = "github:numtide/flake-utils";
    };

    outputs = { self, nixpkgs, flake-utils, ... }@inputs:
        flake-utils.lib.eachDefaultSystem (system: let
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
                #
                # According to Nix, "self" evaluates faster than "./."
                src = pkgs.lib.cleanSource self;

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
            };

            # Server-specific options
            serverPkgArgs = basePkgArgs // {
                pname = (pkgs.lib.importTOML ./server/Cargo.toml).package.name;
                cargoExtraArgs = "--package=pigweb_server";

                # postgresql lib needed for server. kudos to duck.ai for
                # interpreting the abysmal error message complaining about it
                buildInputs = with pkgs; [
                    libpq
                ];
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
                    gcc
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
                PIGWEB_DATABASE__URI = "${DATABASE_URL}";

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

                # why tf do i have to *manually* define this?
                #
                # To find the hashes on a new version, set both to
                # pkgs.lib.fakeHash and run a build. The first hash you get will
                # be for src. Run it again to get the hash for cargoDeps
                wasm-bindgen-cli = pkgs.buildWasmBindgenCli rec {
                    src = pkgs.fetchCrate {
                        pname = "wasm-bindgen-cli";
                        hash = "sha256-3RJzK7mkYFrs7C/WkhW9Rr4LdP5ofb2FdYGz1P7Uxog=";
                        #hash = pkgs.lib.fakeHash;

                        # as per the docs: "this version must match EXACTLY the
                        # one defined in Cargo.lock." this function sets the
                        # version so i don't have to manually. it:
                        #
                        # - loads the "package" list from Cargo.lock
                        # - finds the first package with the name "wasm-bindgen"
                        #   defers to the attrset explicitly defined in the
                        #   function call if not found
                        # - get the version
                        #
                        # https://teu5us.github.io/nix-lib.html#lib.lists.findfirst
                        version = (pkgs.lib.lists.findFirst (crate: crate.name == "wasm-bindgen") { version = "0.2.100"; } (pkgs.lib.importTOML ./Cargo.lock).package).version;
                    };

                    cargoDeps = pkgs.rustPlatform.fetchCargoVendor {
                        inherit src;
                        inherit (src) pname version;
                        hash = "sha256-qsO12332HSjWCVKtf1cUePWWb9IdYUmT+8OPj/XP2WE=";
                        #hash = pkgs.lib.fakeHash;
                    };
                };
            });

            packages.pigweb_server = craneLib.buildPackage (serverPkgArgs // {
                cargoArtifacts = serverDeps;
            });
        }) // rec {
            # NIXOS MODULE DECLARATION
            # we don't need to replicate this per system
            #
            # this WAS in a separate module.nix file, imported into the flake,
            # but nix can't find inputs.self.outputs.packages.${pkgs.system}.pigweb_server
            # (for some dumb fucking reason that is beyond my comprehension).
            # moving everything here does make the file more complicated, but
            # it's less headache-inducing than trying to debug nix
            nixosModules.default = { lib, pkgs, config, ... }: let
                # https://github.com/NixOS/nixpkgs/blob/f3bcd5a33555796da50d1e5675c0dfebcf94c6cf/nixos/modules/services/networking/corerad.nix
                cfg = config.services.pigweb;
                format = pkgs.formats.toml {};
            in {
                options.services.pigweb = {
                    enable = lib.mkEnableOption "the PigWebApp server";
                    openFirewall = lib.mkOption {
                        default = false;
                        type = lib.types.bool;
                        description = "Whether to open the firewall for the PigWeb server.";
                    };
                    createDatabase = lib.mkOption {
                        default = true;
                        type = lib.types.bool;
                        description = "Whether to create a local database automatically.";
                    };
                    config = lib.mkOption {
                        default = {};
                        description = ''
                            The configuration for the PigWeb server. Also includes options
                            for the underlying Rocket web server, which you can view at
                            <https://rocket.rs/guide/v0.5/configuration/#overview>

                            Does not support profiles.
                        '';
                        # everything here must have default values or else the
                        # toml generator complains. also no null values.
                        # "Error: Cannot convert data to TOML (null values are not supported)"
                        # why? because fuck you, that's why.
                        type = lib.types.submodule {
                            freeformType = format.type;
                            options = {
                                port = lib.mkOption {
                                    type = lib.types.port;
                                    default = 8000;
                                    description = "Port to serve on.";
                                };
                                database = lib.mkOption {
                                    default = {};
                                    description = ''
                                        The configuration for the PostgreSQL database PigWeb
                                        uses. You can set each option individually or just
                                        the URI.
                                    '';
                                    type = lib.types.submodule {
                                        freeformType = format.type;
                                        options = {
                                            #uri = lib.mkOption {
                                            #    type = lib.types.singleLineStr;
                                            #    default = null;
                                            #    description = ''
                                            #        The full connection URI to use. If
                                            #        defined, all other options are ignored
                                            #        and this is used instead. Refer to the
                                            #        [Postgres docs](https://www.postgresql.org/docs/9.4/libpq-connect.html#LIBPQ-CONNSTRING)
                                            #        for formatting.
                                            #    '';
                                            #};
                                            host = lib.mkOption {
                                                type = lib.types.singleLineStr;
                                                default = "/var/run/postgresql"; # unix socket location
                                                description = "Name of the host to connect to.";
                                            };
                                            port = lib.mkOption {
                                                type = lib.types.port;
                                                default = config.services.postgresql.settings.port;
                                                description = "Port on the host to connect to.";
                                            };
                                            dbname = lib.mkOption {
                                                type = lib.types.singleLineStr;
                                                default = cfg.config.database.user;
                                                description = "Name of the database to use.";
                                            };
                                            user = lib.mkOption {
                                                type = lib.types.singleLineStr;
                                                default = "pigweb";
                                                description = "The Postgres user to sign in as.";
                                            };
                                            password = lib.mkOption {
                                                type = lib.types.singleLineStr;
                                                default = "";
                                                description = "The password for the user, if required.";
                                            };
                                        };
                                    };
                                };
                                groups = lib.mkOption {
                                    type = lib.types.attrsOf (lib.types.listOf (lib.types.enum [
                                        "PigViewer"
                                        "PigEditor"
                                        "BulkEditor"
                                        "BulkAdmin"
                                        "UserViewer"
                                        "UserAdmin"
                                        "LogViewer"
                                    ]));
                                    default = {};
                                    description = ''
                                        The permission groups the server should recognize.

                                        The server will read each user's groups when signing
                                        in with OIDC and grant the corresponding roles
                                        defined in each group here.
                                    '';
                                    example = {
                                        user = [ "PigViewer" "PigEditor" "BulkEditor" ];
                                        admin = [ "BulkAdmin" "UserViewer" "UserAdmin" "LogViewer" ];
                                    };
                                };
                            };
                        };
                    };
                    environmentFile = lib.mkOption {
                        type = lib.types.nullOr lib.types.path;
                        default = null;
                        description = ''
                            The environment file as defined in {manpage}`systemd.exec(5)`.

                            This is used to prevent secrets from being saved in the global
                            /nix/store. All config options should be prefixed by PIGWEB_
                        '';
                    };
                };

                config = lib.mkIf cfg.enable {

                    # Open ports
                    networking.firewall.allowedTCPPorts = lib.optionals cfg.openFirewall [ cfg.config.port ];

                    # Create PostgreSQL DB
                    services.postgresql = lib.mkIf cfg.createDatabase {
                        enable = true;
                        ensureDatabases = [ cfg.config.database.dbname ];
                        ensureUsers = [{
                            name = cfg.config.database.user;
                            ensureDBOwnership = true;
                        }];
                    };

                    # Enable systemd service
                    systemd.services."pigweb" = let
                        pigwebConfigFile = format.generate "PigWeb.toml" cfg.config;
                    in {
                        script = "${lib.getExe self.outputs.packages.${pkgs.system}.pigweb_server}";
                        wantedBy = [ "multi-user.target" ];
                        wants = [ "network-online.target" ];
                        after = [ "network-online.target" ] ++ (lib.optionals cfg.createDatabase [ "postgresql.service" ]);
                        environment = {
                            PIGWEB_CONFIG = "${pigwebConfigFile}";
                            PIGWEB_CLIENT_PATH = "${self.outputs.packages.${pkgs.system}.pigweb_client}";
                        };
                        serviceConfig = {
                            EnvironmentFile = lib.mkIf (cfg.environmentFile != null) [ cfg.environmentFile ];
                            DynamicUser = true;
                            User = "pigweb";
                            Restart = "on-failure";
                            RestartSec = "1s";
                        };
                    };

                };

            };
        };
}
