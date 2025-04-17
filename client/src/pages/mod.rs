use crate::data::state::ClientState;
use crate::pages::pigpage::PigPageRender;
use egui::Ui;
use matchit::Params;

pub mod layout;
pub mod pigpage;

#[derive(Debug, PartialEq, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub enum Routes {
    Pigs,
}

impl Routes {
    pub fn get_renderer(&mut self) -> Box<dyn RenderPage> {
        match self {
            Self::Pigs => Box::new(PigPageRender::default()),
        }
    }
}

pub trait RenderPage {
    fn open(&mut self, state: &mut ClientState, params: Option<&Params>) {}

    fn ui(&mut self, ui: &mut Ui, state: &mut ClientState, params: Option<&Params>);
}
