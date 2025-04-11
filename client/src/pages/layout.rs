use crate::data::api::{AuthApi, Status};
use crate::data::state::ClientState;
use crate::modal::Modal;
use eframe::emath::Align;
use egui::{menu, Context, OpenUrl, SelectableLabel, TopBottomPanel, Ui, ViewportCommand};
use pigweb_common::{yuri, AUTH_API_ROOT};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Layout {
    #[serde(skip)]
    auth_api: AuthApi,
}

impl Default for Layout {
    fn default() -> Self {
        let mut auth_api = AuthApi::default();

        // Check whether the user is logged in
        auth_api.is_authenticated.request(false); // this arg doesn't matter

        Self { auth_api }
    }
}

impl Layout {
    pub fn ui(ui: &mut Ui, state: &mut ClientState) {
        // Handle all the incoming data
        Self::process_promises(state);

        TopBottomPanel::top("top_panel").resizable(false).show(ui.ctx(), |ui| {
            menu::bar(ui, |ui| {
                Self::populate_menu(ui, state);
            });
        });

        Self::show_modals(ui.ctx(), state);
    }

    fn process_promises(state: &mut ClientState) {
        match state.layout.auth_api.is_authenticated.resolve() {
            Status::Received(authenticated) => state.authenticated = authenticated,
            Status::Errored(err) => state.display_error = Some(err),
            Status::Pending => {}
        }
    }

    fn populate_menu(ui: &mut Ui, state: &mut ClientState) {
        ui.add_space(2.0);

        // Use the Colorix theme picker instead of egui's
        state.colorix.light_dark_toggle_button(ui, 14.0);

        ui.separator();

        // TODO only show pages you have access to
        // TODO make these actually route
        ui.toggle_value(&mut true, " 🐖 Pigs ");
        ui.add_enabled(false, SelectableLabel::new(false, " 📄 Logs "));
        ui.add_enabled(false, SelectableLabel::new(false, " 😐 Users "));
        ui.add_enabled(false, SelectableLabel::new(false, " ⛭ System "));

        // Show debug warning
        if cfg!(debug_assertions) {
            ui.separator();
            egui::warn_if_debug_build(ui);
        }

        // This right aligns it on the same row
        let is_web = cfg!(target_arch = "wasm32");
        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            // Show the quit button if somehow this gets run on desktop
            // (you shouldn't, dumbass)
            if !is_web && ui.button("🗙").clicked() {
                ui.ctx().send_viewport_cmd(ViewportCommand::Close);
            }

            // Logout
            if ui.button("⎆").clicked() {
                ui.ctx().open_url(OpenUrl::same_tab(yuri!(AUTH_API_ROOT, "/oidc/logout/")));
            }
        });
    }

    fn show_modals(ctx: &Context, state: &mut ClientState) {
        if !state.authenticated {
            let modal = Modal::new("Login")
                .with_body("You need to login or renew your session to continue.")
                .cancellable(false)
                .show_with_extras(ctx, |ui| {
                    if ui.button("✔ Ok").clicked() {
                        ui.ctx().open_url(OpenUrl::same_tab(yuri!(AUTH_API_ROOT, "/oidc/login/")));
                    }
                });

            if modal.should_close() {
                ctx.open_url(OpenUrl::same_tab(yuri!(AUTH_API_ROOT, "/oidc/login/")));
            }
        }

        // TODO swap error modal for banner
        if let Some(err_unwrapped) = state.display_error.as_ref() {
            let heading = err_unwrapped.reason.as_ref().unwrap_or(&"Error".to_owned()).to_string();
            let heading_with_code = match err_unwrapped.code {
                Some(code) => format!("{:?} {:?}", code, heading),
                None => heading,
            };

            let modal = Modal::new("error")
                .with_heading(heading_with_code)
                .with_body(err_unwrapped.description.as_str())
                .show(ctx);

            if modal.should_close() {
                state.display_error = None;
            }
        }
    }
}
