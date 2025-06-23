# Install

> [!CAUTION]
> ***So you want to host this yourself, huh?*** This app is first and foremost designed to fit my use case and to do the bare minimum.
>
> - **The polish on the client is not perfect.** While the UI looks good, there are quirks in its behavior, and it may lack features you want.
> - **The server runs slow in some cases.** Rocket's code is good, mine is not as I haven't done much manual optimization. Creating BulkImports is especially slow as it checks all the new names for duplicates one at a time.
> - **The server may be difficult to maintain.** Assuming you're not on NixOS, you need to set up PostgreSQL and systemd (or another service manager) on your own.
>
> If you're fine with figuring out everything on your own and realize this app is effectively an alpha product not meant to scale to many users, feel free to continue.

## NixOS Setup

1. **Import the Flake and the module.** In your `flake.nix`, add the following:

    ```nix
    {
        inputs.pigweb.url = "github:justinhschaaf/PigWebApp/main";

        outputs = { nixpkgs, ... }@inputs:
        let
            system = "x86_64-linux";
            pkgs = import nixpkgs { inherit system; };
        in {
            nixosConfigurations.nixos = nixpkgs.lib.nixosSystem {
                specialArgs = { inherit inputs system; };
                modules = [
                    inputs.pigweb.nixosModules.default
                    ./configuration.nix
                    ...
                ];
            };
        }
    }
    ```

2. **Enable and configure the server.** See [the config guide](Config.md) for all options. As an example, in your `configuration.nix`:

    ```nix
    { inputs, pkgs, ... }: {

        services.pigweb = {
            enable = true;
            openFirewall = true;
            environmentFile = "/run/secrets/pigweb/pigweb-env";
            config = {
                groups = {
                    user = [ "PigViewer" "PigEditor" "BulkEditor" ];
                    admin = [ "BulkAdmin" "UserViewer" "UserAdmin" "LogViewer"];
                };

                oidc = {
                    auth_uri = "https://authentik.local/application/o/authorize/";
                    token_uri = "https://authentik.local/application/o/token/";
                    redirect_uri = "http://pigweb.local/auth/oidc/response";
                    logout_uri = "https://authentik.local/application/o/pigweb-indev/end-session/";
                    scopes = [ "openid" "profile" ];
                };
            };
        };

        ...

    }
    ```

3. **Rebuild the system.**

## Manual Setup

1. Follow the *Workspace Setup* instructions in the [README](../README.md).
2. In the Nix shell, run `cargo make -p production`.
3. Copy the contents of the `./target/release/` folder to your final install location.
4. In your install folder, create a file named `PigWeb.toml`. See [the config guide](Config.md) for all options. You'll want to set up the following:
    - A [PostgreSQL](https://www.postgresql.org/) database
    - A reverse proxy with HTTPS, e.g. [Caddy](https://caddyserver.com/)
    - An SSO provider which supports OIDC, e.g. [Authentik](https://goauthentik.io/)
    - Set a secure [secret_key](https://rocket.rs/guide/v0.5/configuration/#secret-key) using `openssl rand -base64 32`
5. Navigate to your install folder in a terminal, then run `./pigweb_server`. The app should now be available at <http://localhost:8000> or the hostname you configured with your reverse proxy.
