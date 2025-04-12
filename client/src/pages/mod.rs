use crate::data::state::ClientState;
use crate::pages::pigpage::PigPage;
use egui::Ui;
use matchit::Params;

pub mod layout;
pub mod pigpage;

pub enum Routes {
    Pigs,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub enum Pages {
    Pigs(PigPage),
    Logs,
    Users,
    System,
}

impl Pages {
    pub fn data(&mut self) -> Box<&mut dyn PageImpl> {
        match self {
            Pages::Pigs(page) => Box::new(page),
            _ => todo!(),
        }
    }
}

pub trait PageImpl {
    fn ui(&mut self, ui: &mut Ui, state: &mut ClientState, params: &Params);
}
