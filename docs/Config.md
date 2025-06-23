# Config

The server can be configured in each of the following ways. See the ***Config Options*** section for the full list of available options.

### NixOS Module

This is likely the easiest method as it configures the PostgreSQL database and systemd service for you. Make sure you've already added this repo to your flake and imported the module as per the installation instructions.

| key                               | type        | description                                                                                                                                                                                                                    | default |
|-----------------------------------|-------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|---------|
| `services.pigweb.enable`          | `bool`      | Whether to enable the PigWebApp server.                                                                                                                                                                                        | `false` |
| `services.pigweb.openFirewall`    | `bool`      | Whether to open the firewall for the PigWeb server.                                                                                                                                                                            | `false` |
| `services.pigweb.createDatabase`  | `bool`      | Whether to create a local database automatically.                                                                                                                                                                              | `true`  |
| `services.pigweb.config`          | `submodule` | The configuration for the PigWeb server, see below. Also includes options for the underlying Rocket web server, which you can view at <https://rocket.rs/guide/v0.5/configuration/#overview><br><br>Does not support profiles. | `{}`    |
| `services.pigweb.environmentFile` | `path`      | The environment file as defined in `systemd.exec(5)`.<br><br>This is used to prevent secrets from being saved in the global `/nix/store`. All config options should be prefixed by `PIGWEB_`                                   | `null`  |

### TOML file

By default, the server will search for a `PigWeb.toml` file relative to the run directory. You can change this location with the `PIGWEB_CONFIG` environment variable.

#### Example

```toml
port = 8000

[database]
host = "localhost"
port = 5432
dbname = "pigweb"
user = "pigweb"

[groups]
user = ["PigViewer", "PigEditor", "BulkEditor"]
admin = ["BulkAdmin", "UserViewer", "UserAdmin", "LogViewer"]

[oidc]
auth_uri = "https://authentik.local/application/o/authorize/"
token_uri = "https://authentik.local/application/o/token/"
redirect_uri = "http://pigweb.local/auth/oidc/response"
logout_uri = "https://authentik.local/application/o/pigweb-indev/end-session/"
scopes = ["openid", "profile"]
```

### Environment Variable

Environment variables take precedent over the TOML file, allowing you to avoid placing secrets in the config file.

- Prefix the key for each config option with `PIGWEB_`
- Rocket config options should also be prefixed with `PIGWEB_` ***instead of `ROCKET_`***
- For any subsections, replace the separator with two underscores (`__`), e.g. `database.uri` would be `PIGWEB_DATABASE__URI`

## Config Options

The server supports the options below and requires most of them to be set to function.

> [!IMPORTANT]
> Make sure you set a [`secret_key`](https://rocket.rs/guide/v0.5/configuration/#secret-key) for use encrypting values. The server will not start if this isn't set.

- **All [Rocket config options](https://rocket.rs/guide/v0.5/configuration/) are set in the same way.** Ignore the section on Profiles.

| key           | type                      | description                                                                                                                                                                        | default            |
|---------------|---------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|--------------------|
| `client_path` | `String`                  | The path to the compiled client files.                                                                                                                                             | `"dist"`           |
| `database`    | `DatabaseConfig`          | Params for connecting to the Postgres database, see below for options.                                                                                                             | See defaults below |
| `groups`      | `Map<String, Set<Roles>>` | The permission groups the server should recognize. The server will read each user's groups when signing in with OIDC and grant the corresponding roles defined in each group here. | Empty              |
| `oidc`        | `OpenIDConfig`            | Config for the OIDC SSO provider                                                                                                                                                   | `None`             |

### DatabaseConfig

| key                 | type     | description                                                                                                                                                                                                               | default       |
|---------------------|----------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|---------------|
| `database.uri`      | `String` | The full connection URI to use. If defined, all other options are ignored and this is used instead. Refer to the [Postgres docs](https://www.postgresql.org/docs/9.4/libpq-connect.html#LIBPQ-CONNSTRING) for formatting. | `None`        |
| `database.host`     | `String` | Name of the host to connect to                                                                                                                                                                                            | `"localhost"` |
| `database.port`     | `u16`    | Port on the host to connect to                                                                                                                                                                                            | `5432`        |
| `database.dbname`   | `String` | Name of the database to use                                                                                                                                                                                               | `"pigweb"`    |
| `database.user`     | `String` | The Postgres user to sign in as                                                                                                                                                                                           | `"pigweb"`    |
| `database.password` | `String` | The password for the user, if required                                                                                                                                                                                    | `None`        |

### OpenIDConfig

| key             | type          | description                                                                                                                                                                                            |
|-----------------|---------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `auth_uri`      | `String`      | The endpoint to submit the authorization request to                                                                                                                                                    |
| `token_uri`     | `String`      | The token exchange endpoint with the OIDC provider                                                                                                                                                     |
| `redirect_uri`  | `String`      | The URI the OIDC provider should send the response to. ***Should start with your hostname and end in `/auth/oidc/response`.***                                                                         |
| `logout_uri`    | `String`      | When signing out, the user will be redirected here to end the session with the OIDC provider as well                                                                                                   |
| `client_id`     | `String`      | The client id assigned by your OIDC provider                                                                                                                                                           |
| `client_secret` | `String`      | The client secret assigned by your OIDC provider                                                                                                                                                       |
| `scopes`        | `Vec<String>` | The list of scopes to request from the OIDC provider. For [Authentik](https://docs.goauthentik.io/docs/add-secure-apps/providers/oauth2/#default--special-scopes), this is just `openid` and `profile` |
