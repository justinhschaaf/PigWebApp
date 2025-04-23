use crate::data::state::ClientState;
use crate::pages::errpage::ErrPageRender;
use crate::pages::pigpage::PigPageRender;
use crate::pages::userpage::UserPageRender;
use egui::{Context, Ui};
use urlable::ParsedURL;

mod errpage;
pub mod layout;
pub mod pigpage;
mod userpage;

#[derive(Debug, PartialEq, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub enum Routes {
    Pigs,
    Users,
    NotFound,
}

impl Routes {
    pub fn get_renderer(&mut self) -> Box<dyn RenderPage> {
        match self {
            Self::Pigs => Box::new(PigPageRender::default()),
            Self::Users => Box::new(UserPageRender::default()),
            Self::NotFound => Box::new(ErrPageRender::default()),
        }
    }
}

pub trait RenderPage {
    fn open(&mut self, ctx: &Context, state: &mut ClientState, url: &ParsedURL) {}

    fn ui(&mut self, ui: &mut Ui, state: &mut ClientState, url: &ParsedURL);
}
