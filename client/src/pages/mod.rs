use crate::data::state::ClientState;
use egui::Ui;
use uuid::Uuid;

pub(crate) mod layout;
pub(crate) mod pigpage;

#[derive(Debug, PartialEq, Clone, serde::Deserialize, serde::Serialize)]
pub(crate) enum Page {
    Pigs(Option<Uuid>),
    Logs,
    Users,
    System,
}

impl Page {
    pub fn get_route(&self) -> String {
        match self {
            Page::Pigs(opt_id) => {
                if let Some(id) = opt_id {
                    format!("/pigs/{:?}", id)
                } else {
                    "/pigs".to_owned()
                }
            }
            _ => "/".to_string(),
        }
    }
}

pub trait PageImpl {
    fn new() -> Self;
    fn ui(ui: &mut Ui, state: &mut ClientState);
}
