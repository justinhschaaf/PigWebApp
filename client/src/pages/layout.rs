use crate::data::api::{ApiError, AuthApi, Status};
use crate::data::state::ClientState;
use crate::pages::{RenderPage, Routes};
use crate::ui::modal::Modal;
use crate::ui::spaced_heading;
use crate::ui::style::{COLOR_REJECTED, SPACE_SMALL};
use eframe::emath::Align;
use egui::{menu, Context, OpenUrl, RichText, SelectableLabel, TopBottomPanel, Ui, ViewportCommand};
use pigweb_common::users::Roles;
use pigweb_common::{yuri, AUTH_API_ROOT};
use urlable::ParsedURL;

/// Persistent data storage for the common layout
#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Layout {
    /// The error message currently on display, if any
    #[serde(skip)]
    pub display_error: Vec<ApiError>,
}

impl Default for Layout {
    fn default() -> Self {
        Self { display_error: Vec::new() }
    }
}

/// The renderer for the main layout. This is run before the current route
/// renderer and holds any elements common to all pages.
pub struct LayoutRender {
    /// API used to check whether the user is signed in upon first loading the
    /// page.
    auth_api: AuthApi,
}

impl Default for LayoutRender {
    fn default() -> Self {
        Self { auth_api: AuthApi::default() }
    }
}

impl RenderPage for LayoutRender {
    fn open(&mut self, _ctx: &Context, _state: &mut ClientState, _url: &ParsedURL) {
        // Check whether the user is logged in
        self.auth_api.is_authenticated.request(false); // this arg doesn't matter
    }

    fn ui(&mut self, ui: &mut Ui, state: &mut ClientState, _url: &ParsedURL) {
        // Handle all the incoming data
        self.process_promises(state);

        TopBottomPanel::top("top_panel").resizable(false).show(ui.ctx(), |ui| {
            menu::bar(ui, |ui| {
                self.populate_menu(ui, state);
            });
        });

        // show error banner, if we have one
        self.display_error(ui, state);

        self.show_modals(ui.ctx(), state);
    }
}

impl LayoutRender {
    /// Checks all APIs for data received from previously submitted requests
    fn process_promises(&mut self, state: &mut ClientState) {
        match self.auth_api.is_authenticated.resolve() {
            Status::Received(authorized) => {
                // upon first loading the app without being signed in, everything will error due to
                // being unauthorized. this clears those up for cleanliness.
                if authorized.is_none() {
                    state.pages.layout.display_error.clear();
                }

                // save the authorized state
                state.authorized = authorized;
            }
            Status::Errored(err) => state.pages.layout.display_error.push(err),
            Status::Pending => {}
        }
    }

    /// Show the menu/nav bar at the top of the screen
    fn populate_menu(&mut self, ui: &mut Ui, state: &mut ClientState) {
        ui.add_space(SPACE_SMALL);

        // Use the Colorix theme picker instead of egui's
        state.colorix.light_dark_toggle_button(ui, 14.0);

        ui.separator();

        // attention to detail: if the user doesn't have access to any pages and
        // debug is enabled, there will be two separators with an awkward gap
        // between them. this will remove the second separator if no pages are
        // allowed
        let mut show_second_separator = false;

        // link to each page the user can see
        if state.has_role(Roles::PigViewer) {
            let current = state.route == Routes::Pigs;
            if ui.add(SelectableLabel::new(current, " 🐖 Pigs ")).clicked() {
                if !current {
                    ui.ctx().open_url(OpenUrl::same_tab("/pigs"))
                }
            }
            show_second_separator = true;
        }
        if state.has_role(Roles::BulkEditor) || state.has_role(Roles::BulkAdmin) {
            let current = state.route == Routes::Bulk;
            if ui.add(SelectableLabel::new(current, " 📥 Import ")).clicked() {
                if !current {
                    ui.ctx().open_url(OpenUrl::same_tab("/bulk"))
                }
            }
            show_second_separator = true;
        }
        if state.has_role(Roles::LogViewer) {
            ui.add_enabled(false, SelectableLabel::new(false, " 📄 Logs "));
            show_second_separator = true;
        }
        if state.has_role(Roles::UserViewer) {
            let current = state.route == Routes::Users;
            if ui.add(SelectableLabel::new(current, " 😐 Users ")).clicked() {
                if !current {
                    ui.ctx().open_url(OpenUrl::same_tab("/users"))
                }
            }
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
            if !is_web && ui.button(" 🗙 ").clicked() {
                ui.ctx().send_viewport_cmd(ViewportCommand::Close);
            }

            // Logout
            if ui.button(" ⎆ ").clicked() {
                ui.ctx().open_url(OpenUrl::same_tab(yuri!(AUTH_API_ROOT, "/oidc/logout/")));
            }
        });
    }

    /// Display all errors as a banner at the top of the page
    fn display_error(&mut self, ui: &mut Ui, state: &mut ClientState) {
        // items which should be removed, borrow check doesn't like it in the for loop
        let mut remove = Vec::new();

        for (i, err) in state.pages.layout.display_error.iter().enumerate() {
            let heading = err.reason.as_ref().unwrap_or(&"Error".to_owned()).to_owned();
            let heading_with_code = match err.code {
                Some(code) => format!("{} {}", code, heading),
                None => heading,
            };

            TopBottomPanel::top(format!("error_panel_{:?}", i)).resizable(false).show(ui.ctx(), |ui| {
                menu::bar(ui, |ui| {
                    state.colorix.draw_background(ui.ctx(), true);

                    // add error message
                    spaced_heading(ui, RichText::new(heading_with_code).color(COLOR_REJECTED).strong());
                    ui.separator();
                    ui.label(RichText::new(err.description.as_str()).color(COLOR_REJECTED));

                    // right align dismiss button
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if ui.button(" 🗙 ").clicked() {
                            remove.push(i);
                        }
                    });
                });
            });
        }

        // remove the errors which should be dismissed
        for i in remove.iter() {
            state.pages.layout.display_error.remove(*i);
        }
    }

    /// Show any page-specific modals which should be visible
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
    }
}
