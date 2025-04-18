use crate::data::state::ClientState;
use crate::pages::errpage::ErrPageRender;
use crate::pages::pigpage::PigPageRender;
use egui::Ui;
use urlable::ParsedURL;

mod errpage;
pub mod layout;
pub mod pigpage;

#[derive(Debug, PartialEq, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub enum Routes {
    Pigs,
    NotFound,
}

impl Routes {
    pub fn get_renderer(&mut self) -> Box<dyn RenderPage> {
        match self {
            Self::Pigs => Box::new(PigPageRender::default()),
            Self::NotFound => Box::new(ErrPageRender::default()),
        }
    }
}

pub trait RenderPage {
    fn open(&mut self, state: &mut ClientState, url: &ParsedURL) {}

    fn ui(&mut self, ui: &mut Ui, state: &mut ClientState, url: &ParsedURL);
}
