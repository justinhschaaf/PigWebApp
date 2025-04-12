use crate::data::state::ClientState;
use crate::pages::layout::Layout;
use crate::pages::pigpage::PigPage;
use crate::pages::{PageImpl, Pages, Routes};
use crate::style;
use egui::Context;
use matchit::Router;
use pigweb_common::users::Roles;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
// TODO figure out whether we really need to save any state or if it's better to just reset
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct PigWebClient {
    /// Global app info
    state: ClientState,

    /// The currently open page
    page: Pages,

    /// The page router
    #[serde(skip)]
    router: Router<Routes>,
}

impl Default for PigWebClient {
    fn default() -> Self {
        Self { state: ClientState::default(), page: Pages::Pigs(PigPage::default()), router: Router::new() }
    }
}

impl eframe::App for PigWebClient {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Show the global layout first
            Layout::ui(ui, &mut self.state);

            // Then show the current route
            // TODO actually get the URL path
            // TODO there's gotta be a better way to define these
            if let Ok(route) = &self.router.at("/pigs") {
                match route.value {
                    Routes::Pigs => {
                        if !matches!(self.page, Pages::Pigs(_)) {
                            self.page = Pages::Pigs(PigPage::default());
                        }

                        if self.state.has_role(Roles::PigViewer) {
                            self.page.data().ui(ui, &mut self.state, &route.params);
                        } // TODO 403 Forbidden
                    }
                }
            } else {
                // TODO 404 not found
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
        res.register_routes();
        res
    }

    pub fn register_routes(&mut self) {
        self.router.insert("/pigs/{*id}", Routes::Pigs).expect("Can't add route!");
        self.router.insert("/pigs", Routes::Pigs).expect("Can't add route!");
    }
}
