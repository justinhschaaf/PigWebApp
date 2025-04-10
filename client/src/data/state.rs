use crate::data::api::ApiError;
use crate::pages::layout::Layout;
use crate::pages::pigpage::PigPage;
use crate::pages::{Page, PageImpl};
use crate::style;
use egui_colors::Colorix;
use tokio::sync::mpsc;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
// TODO figure out whether we really need to save any state or if it's better to just reset
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct ClientState {
    pub authenticated: bool,

    /// Global theme info
    #[serde(skip)]
    pub colorix: Colorix,

    /// The currently open page
    pub page: Page,

    /// The channel sending page updates to the router
    #[serde(skip)]
    pub route_sender: Option<mpsc::Sender<Page>>,

    /// The common layout shown on all pages
    pub layout: Layout,

    /// Data storage for the pig page
    pub pig_page: PigPage,

    /// The error message currently on display, if any
    pub display_error: Option<ApiError>,
}

impl Default for ClientState {
    fn default() -> Self {
        Self {
            authenticated: false,
            colorix: Colorix::default(),
            page: Page::Pigs(None),
            route_sender: None,
            layout: Layout::new(),
            pig_page: PigPage::new(),
            display_error: None,
        }
    }
}

impl ClientState {
    /// The key used to access cached data for the Pig Web App
    pub const APP_KEY: &'static str = "pigweb";

    pub fn new(cc: &eframe::CreationContext<'_>, route_sender: mpsc::Sender<Page>) -> Self {
        Self {
            colorix: style::set_styles(cc),
            route_sender: Some(route_sender),
            // Load previous app state (if any) or default state data
            // Note that you must enable the `persistence` feature for this to work.
            ..cc.storage.and_then(|storage| eframe::get_value(storage, Self::APP_KEY)).unwrap_or_default()
        }
    }

    pub fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, ClientState::APP_KEY, &self);
    }

    pub fn update_route(&mut self, page: Page) {
        // Tell the router, this also updates self.page
        if let Some(sender) = self.route_sender.as_ref() {
            sender.try_send(page).unwrap_or_default();
        }
    }
}
