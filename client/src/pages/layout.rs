use crate::data::api::{ApiError, AuthApi, Status};
use crate::data::state::ClientState;
use crate::modal::Modal;
use crate::pages::RenderPage;
use eframe::emath::Align;
use egui::{menu, Context, OpenUrl, SelectableLabel, TopBottomPanel, Ui, ViewportCommand};
use matchit::Params;
use pigweb_common::users::Roles;
use pigweb_common::{yuri, AUTH_API_ROOT};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Layout {
    /// The error message currently on display, if any
    pub display_error: Option<ApiError>,
}

impl Default for Layout {
    fn default() -> Self {
        Self { display_error: None }
    }
}

pub struct LayoutRender {
    auth_api: AuthApi,
}

impl Default for LayoutRender {
    fn default() -> Self {
        let mut auth_api = AuthApi::default();

        // Check whether the user is logged in
        auth_api.is_authenticated.request(false); // this arg doesn't matter

        Self { auth_api }
    }
}

impl RenderPage for LayoutRender {
    fn ui(&mut self, ui: &mut Ui, state: &mut ClientState, _params: Option<&Params>) {
        // Handle all the incoming data
        self.process_promises(state);

        TopBottomPanel::top("top_panel").resizable(false).show(ui.ctx(), |ui| {
            menu::bar(ui, |ui| {
                self.populate_menu(ui, state);
            });
        });

        self.show_modals(ui.ctx(), state);
    }
}

impl LayoutRender {
    fn process_promises(&mut self, state: &mut ClientState) {
        match self.auth_api.is_authenticated.resolve() {
            Status::Received(authorized) => state.authorized = authorized,
            Status::Errored(err) => state.pages.layout.display_error = Some(err),
            Status::Pending => {}
        }
    }

    fn populate_menu(&mut self, ui: &mut Ui, state: &mut ClientState) {
        ui.add_space(2.0);

        // Use the Colorix theme picker instead of egui's
        state.colorix.light_dark_toggle_button(ui, 14.0);

        ui.separator();

        // attention to detail: if the user doesn't have access to any pages and
        // debug is enabled, there will be two separators with an awkward gap
        // between them. this will remove the second separator if no pages are
        // allowed
        let mut show_second_separator = false;

        // TODO make these actually change the page if it's needed
        if state.has_role(Roles::PigViewer) {
            ui.toggle_value(&mut true, " 🐖 Pigs ");
            show_second_separator = true;
        }
        if state.has_role(Roles::BulkEditor) {
            ui.add_enabled(false, SelectableLabel::new(false, " 📥 Import "));
            show_second_separator = true;
        }
        if state.has_role(Roles::LogViewer) {
            ui.add_enabled(false, SelectableLabel::new(false, " 📄 Logs "));
            show_second_separator = true;
        }
        if state.has_role(Roles::UserViewer) {
            ui.add_enabled(false, SelectableLabel::new(false, " 😐 Users "));
            show_second_separator = true;
        }
        //ui.add_enabled(false, SelectableLabel::new(false, " ⛭ System "));

        // Show debug warning
        if cfg!(debug_assertions) {
            if show_second_separator {
                ui.separator();
            }
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

    fn show_modals(&mut self, ctx: &Context, state: &mut ClientState) {
        if state.authorized.is_none() {
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
        if let Some(err_unwrapped) = state.pages.layout.display_error.as_ref() {
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
                state.pages.layout.display_error = None;
            }
        }
    }
}
