{ self, lib, pkgs, config, ... }: let
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
            type = lib.types.submodule {
                freeformType = format.type;
                options = {
                    port = lib.mkOption {
                        type = lib.types.port;
                        default = 8000;
                        description = "Port to serve on.";
                    };
                    database = lib.mkOption {
                        description = ''
                            The configuration for the PostgreSQL database PigWeb
                            uses. You can set each option individually or just
                            the URI.
                        '';
                        type = lib.types.submodule {
                            freeformType = format.type;
                            options = {
                                uri = lib.mkOption {
                                    type = lib.types.singleLineStr;
                                    description = ''
                                        The full connection URI to use. If
                                        defined, all other options are ignored
                                        and this is used instead. Refer to the
                                        [Postgres docs](https://www.postgresql.org/docs/9.4/libpq-connect.html#LIBPQ-CONNSTRING)
                                        for formatting.
                                    '';
                                };
                                host = lib.mkOption {
                                    type = lib.types.singleLineStr;
                                    default = "localhost";
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
                                    description = "The password for the user, if required.";
                                };
                            };
                        };
                    };
                    groups = lib.mkOption {
                        type = lib.types.attrsOf lib.types.listOf lib.types.enum [
                            "PigViewer"
                            "PigEditor"
                            "BulkEditor"
                            "BulkAdmin"
                            "UserViewer"
                            "UserAdmin"
                            "LogViewer"
                        ];
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
            default = null;
            type = lib.types.nullOr lib.types.path;
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

}
