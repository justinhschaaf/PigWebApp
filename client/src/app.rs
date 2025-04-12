use crate::data::state::ClientState;
use crate::pages::layout::Layout;
use crate::pages::pigpage::PigPage;
use crate::pages::{Page, PageImpl};
use crate::style;
use egui::Context;
use pigweb_common::users::Roles;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
// TODO figure out whether we really need to save any state or if it's better to just reset
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct PigWebClient {
    /// Global app info
    state: ClientState,

    /// The currently open page
    page: Page,
}

impl Default for PigWebClient {
    fn default() -> Self {
        Self { state: ClientState::default(), page: Page::Pigs(PigPage::default()) }
    }
}

impl eframe::App for PigWebClient {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Show the global layout first
            Layout::ui(ui, &mut self.state);

            // Then show the current route
            match &mut self.page {
                Page::Pigs(page) => {
                    if self.state.has_role(Roles::PigViewer) {
                        page.ui(ui, &mut self.state);
                    }
                }
                _ => {}
            }
        });
    }

    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, Self::APP_KEY, &self);
    }
}

impl PigWebClient {
    /// The key used to access cached data for the Pig Web App
    pub const APP_KEY: &'static str = "pigweb";

    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load previous app state (if any) or default state data
        // Note that you must enable the `persistence` feature for this to work.
        let mut res: PigWebClient =
            cc.storage.and_then(|storage| eframe::get_value(storage, Self::APP_KEY)).unwrap_or_default();
        res.state.colorix = style::set_styles(cc);
        res
    }
}
