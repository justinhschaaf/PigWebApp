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

#[derive(Debug, PartialEq, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub enum Routes {
    Pigs,
    Bulk,
    Users,
    NotFound,
}

impl Routes {
    pub fn get_renderer(&mut self) -> Box<dyn RenderPage> {
        match self {
            Self::Pigs => Box::new(PigPageRender::default()),
            Self::Bulk => Box::new(BulkPageRender::default()),
            Self::Users => Box::new(UserPageRender::default()),
            Self::NotFound => Box::new(ErrPageRender::default()),
        }
    }
}

#[allow(unused_variables)]
pub trait RenderPage {
    fn on_url_update(&mut self, ctx: &Context, state: &mut ClientState, url: &ParsedURL) {}

    fn open(&mut self, ctx: &Context, state: &mut ClientState, url: &ParsedURL) {}

    fn ui(&mut self, ui: &mut Ui, state: &mut ClientState, url: &ParsedURL);
}
