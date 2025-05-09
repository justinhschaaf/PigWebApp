# Pig Web App

*This is getting out of hand.*

The Pig Web App is a web GUI to manage a list of pig names. Keeping them in a yml file on a server that is never online
is no longer feasible, and even if it were, the plugin is not built to manage such a large list efficiently.

I'm mainly going this far to write the entire app from the ground up to make sure search queries are handled
server-side, as Pocketbase doesn't support serverside functions.

This is also my entryway into Rust, and as such have documented much of what everything does and what I've learned
throughout the process. If there's some horrible mistake I've made, any tips I could use, or any questions you have,
please feel free to [let me know](https://justinschaaf.com/redirect/mailto).

![](docs/images/list_search.png)
![](docs/images/bulk_wizard.png)

For more screenshots, see the [folder](/docs/images).

## Roadmap

### Milestone 1 - *Complete 2025-04-11*

- [x] **Client and Server modules written in Rust.** - *Complete 2024-12-29* - Shared code and data structures should be in a Common module.
- [x] **[CRUD](https://en.wikipedia.org/wiki/Create%2C_read%2C_update_and_delete) pig names.** - *Complete 2025-02-15*
- [x] **[RBAC](https://en.wikipedia.org/wiki/Role-based_access_control) to allow different levels of access.** - *Complete 2025-04-11* - You
  should also be able to configure groups for assigning these roles to users.
- [x] **[OIDC](https://en.wikipedia.org/wiki/OpenID#OpenID_Connect_(OIDC)) authentication, the app should not manage
  authentication.** - *Complete 2025-03-30* - It should, however, be able to read user groups from OIDC user info and manage users' groups through
  it.
- [x] **Fully declarative configuration.** - *Complete 2025-01-31* - Ideally, this is possible through NixOS modules that you can also use to
  deploy it. The config file itself can be TOML as I don't care about reading it, just processing. It should also be
  able to take config from environment variables (takes precedent over config) and possibly CLI options (takes precedent
  over env).

### Milestone 2

- [ ] **Audit log showing a history of changes.** Should show timestamp, pig name/id, who made the change, and what the
  change was.
- [ ] **[MiniMessage](https://docs.advntr.dev/minimessage/index.html) formatting previews.** This will likely require a
  custom interpreter, unfortunately.
- [x] **Mass Add wizard to import en masse.** - *Complete 2025-05-03* - This should hold your hand through the entire import process, cleaning up
  formatting, automatic duplicates, manual duplication checks, etc. There should be a way to save your progress.
- [ ] **OAuth2 authentication for API endpoints.** This should be used to integrate with the plugin itself.

## Workspace

Additional guides are available in the [/docs](./docs) folder.

### Setup

To simplify workspace setup and installing dependencies, most of it is managed for you with Nix and direnv.

1. **If you're not using NixOS,** install the Nix package manager for your system using the [Determinate Nix Installer](https://github.com/DeterminateSystems/nix-installer).

2. **Install [direnv](https://direnv.net/docs/installation.html) for your system.**

    - For NixOS, a better implementation is provided by [nix-community/nix-direnv](https://github.com/nix-community/nix-direnv).

3. **Add direnv integration to your IDE.** For [RustRover](https://www.jetbrains.com/rust/), I recommend [Direnv Integration](https://plugins.jetbrains.com/plugin/15285-direnv-integration). It doesn't work perfectly, but it still works. Be sure to follow the setup instructions for it.

4. **`cd` into the project dir and run `direnv allow`.** If you need to manually enter the dev shell, use `nix develop`.

> [!IMPORTANT]
> Assume ALL commands hereafter are in the Nix shell unless otherwise stated.

### Developing

Run `cargo make serve` to open a development server on [localhost:8000](http://localhost:8000), allowing you to preview changes in (almost) real time.

Since the web server blocks the thread and as such cargo-make [can't stop the server when it's time to build new changes](https://github.com/tmux/tmux/wiki), the server itself is run in a [tmux](https://github.com/tmux/tmux/wiki) session. To view it's output, run `tmux attach-session -t pigweb` in a separate shell to attach to the session.

When you're done, stop the server with `cargo make stop`.

### Building

Builds are configured using [cargo-make](https://github.com/sagiegurari/cargo-make) to avoid ugly wrapper scripts.

For a production build, run `cargo make -p production`. You can build the client and server separately with Nix using `nix build ./#pigweb_[client/server]`.

## Resources

- "How to Write a Web App in Rust" by Garrett Udstrand, see
  parts [1](https://betterprogramming.pub/how-to-write-a-web-app-in-rust-part-1-3047156660a7) [2](https://medium.com/better-programming/how-to-write-a-web-app-in-rust-part-2-2da195369fc1) [3](https://medium.com/better-programming/building-the-rust-web-app-how-to-use-object-relational-mapper-3af2084555b6) [4](https://medium.com/better-programming/building-the-rust-web-app-proper-error-handling-and-return-values-723f1f07f8cd) [5](https://medium.com/better-programming/building-the-rust-web-app-multiple-users-and-authentication-5ca5988ddfe4) [6](https://medium.com/better-programming/building-the-rust-web-app-finishing-up-1624c9b82f80)
- ["A Rust web server / frontend setup like it's 2022 (with axum and yew)"](https://robert.kra.hn/posts/2022-04-03_rust-web-wasm/)
  by Robert Krahn
- ["A web application completely in Rust"](https://medium.com/@saschagrunert/a-web-application-completely-in-rust-6f6bdb6c4471)
  by Sascha Grunert
- ["Rust fullstack web app! WASM + YEW + ROCKET"](https://dev.to/francescoxx/rust-fullstack-web-app-wasm-yew-rocket-3ian)
  by Francesco Ciulla
- ["Full-stack Rust: A complete tutorial with examples"](https://blog.logrocket.com/full-stack-rust-a-complete-tutorial-with-examples/)
  by Mario Zupan
- ["Building cross-platform GUI apps in Rust using egui"](https://blog.logrocket.com/building-cross-platform-gui-apps-rust-using-egui/)
  by Mario Zupan
- ["Part 1: Building a WebSite in Rust Using Rocket and Yew"](https://theadventuresofaliceandbob.com/posts/rust_rocket_yew_part1.md)
