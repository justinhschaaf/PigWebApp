use crate::data::state::ClientState;
use crate::pages::pigpage::PigPage;
use egui::Ui;

pub mod layout;
pub mod pigpage;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub enum Page {
    Pigs(PigPage),
    Logs,
    Users,
    System,
}

pub trait PageImpl {
    fn ui(&mut self, ui: &mut Ui, state: &mut ClientState);
}
