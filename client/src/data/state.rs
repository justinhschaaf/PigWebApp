use crate::data::api::ApiError;
use crate::pages::layout::Layout;
use egui_colors::Colorix;
use pigweb_common::users::Roles;
use std::collections::BTreeSet;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
// TODO figure out whether we really need to save any state or if it's better to just reset
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct ClientState {
    pub authorized: Option<BTreeSet<Roles>>,

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
        Self { authorized: None, colorix: Colorix::default(), layout: Layout::default(), display_error: None }
    }
}

impl ClientState {
    pub fn has_role(&self, role: Roles) -> bool {
        self.authorized.as_ref().is_some_and(|roles| roles.contains(&role))
    }
}
