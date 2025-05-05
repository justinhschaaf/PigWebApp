use crate::pages::bulkpage::BulkPage;
use crate::pages::layout::Layout;
use crate::pages::pigpage::PigPage;
use crate::pages::Routes;
use egui_colors::Colorix;
use pigweb_common::users::Roles;
use std::collections::BTreeSet;

/// Persistent data stored on the user's device by the client. This should be
/// used for data the user is actively working with where changes may be lost
/// without persistence. Session cookies are handled by the server.
// Derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct ClientState {
    /// List of roles the user has. None if the user isn't authenticated
    pub authorized: Option<BTreeSet<Roles>>,

    /// Global theme info
    #[serde(skip)]
    pub colorix: Colorix,

    /// The current route
    pub route: Routes,

    /// Data storage for individual pages
    pub pages: PageData,
}

impl Default for ClientState {
    fn default() -> Self {
        Self { authorized: None, colorix: Colorix::default(), route: Routes::Pigs, pages: PageData::default() }
    }
}

impl ClientState {
    /// Whether the authenticated user has the given role. Returns `false` if
    /// the user isn't authenticated or doesn't have access
    pub fn has_role(&self, role: Roles) -> bool {
        self.authorized.as_ref().is_some_and(|roles| roles.contains(&role))
    }
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct PageData {
    /// The common layout shown on all pages
    pub layout: Layout,

    /// Main page for managing the pig list
    pub pigs: PigPage,

    /// Page for managing bulk imports
    pub bulk: BulkPage,
}
