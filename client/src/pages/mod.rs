use crate::data::state::ClientState;
use crate::pages::bulkpage::BulkPageRender;
use crate::pages::errpage::ErrPageRender;
use crate::pages::pigpage::PigPageRender;
use crate::pages::userpage::UserPageRender;
use egui::{Context, Ui};
use urlable::ParsedURL;

pub mod bulkpage;
pub mod errpage;
pub mod layout;
pub mod pigpage;
pub mod userpage;

/// The unique page routes users can navigate to
#[derive(Debug, PartialEq, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub enum Routes {
    /// Manage all pigs on the list
    Pigs,

    /// Import multiple names at once
    Bulk,

    /// Manage app users
    Users,

    /// 404 page
    NotFound,
}

impl Routes {
    /// Creates a new renderer responsible for the route
    pub fn get_renderer(&self) -> Box<dyn RenderPage> {
        match self {
            Self::Pigs => Box::new(PigPageRender::default()),
            Self::Bulk => Box::new(BulkPageRender::default()),
            Self::Users => Box::new(UserPageRender::default()),
            Self::NotFound => Box::new(ErrPageRender::default()),
        }
    }
}

/// Anything responsible for actually rendering a route. You should not expect
/// any data stored in this struct to persist. For persistent data, create a
/// separate struct and add it to [ClientState].
#[allow(unused_variables)]
pub trait RenderPage {
    /// Runs when the web browser URL updates, both when the route changes and
    /// stays the same.
    fn on_url_update(&mut self, ctx: &Context, state: &mut ClientState, url: &ParsedURL) {}

    /// Runs when navigating to this page from a different route.
    fn open(&mut self, ctx: &Context, state: &mut ClientState, url: &ParsedURL) {}

    /// Runs every frame to render the UI.
    fn ui(&mut self, ui: &mut Ui, state: &mut ClientState, url: &ParsedURL);
}
