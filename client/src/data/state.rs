use crate::data::api::ApiError;
use crate::pages::layout::Layout;
use egui_colors::Colorix;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
// TODO figure out whether we really need to save any state or if it's better to just reset
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct ClientState {
    pub authenticated: bool,

    /// Global theme info
    #[serde(skip)]
    pub colorix: Colorix,

    /// The common layout shown on all pages
    pub layout: Layout,

    /// The error message currently on display, if any
    pub display_error: Option<ApiError>,
}

impl Default for ClientState {
    fn default() -> Self {
        Self { authenticated: false, colorix: Colorix::default(), layout: Layout::default(), display_error: None }
    }
}
