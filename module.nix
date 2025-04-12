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

        # Enable systemd service
        systemd.services."pigweb" = let
            pigwebConfigFile = format.generate "PigWeb.toml" cfg.config;
        in {
            script = "${lib.getExe self.outputs.packages.${pkgs.system}.pigweb_server}";
            wantedBy = [ "multi-user.target" ];
            wants = [ "network-online.target" ];
            after = [ "network-online.target" ];
            environment = {
                PIGWEB_CONFIG = "${pigwebConfigFile}";
                PIGWEB_CLIENT_PATH = "${self.outputs.packages.${pkgs.system}.pigweb_client}";
            };
            serviceConfig = {
                EnvironmentFile = lib.mkIf (config.services.pigweb.environmentFile != null) [ config.services.pigweb.environmentFile ];
                DynamicUser = true;
                User = "pigweb";
                Restart = "on-failure";
                RestartSec = "1s";
            };
        };

        # TODO properly configure postgres db

    };

}
