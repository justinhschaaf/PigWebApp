use crate::pages::pigpage::PigPageData;
use pigweb_common::yuri;

pub mod layout;
pub mod pigpage;

pub const APP_ROOT: &str = "/";
pub const PIG_PAGE: &str = "/pigs";

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub enum Page {
    Pigs(PigPageData),
    Logs,
    Users,
    System,
}

impl Page {
    pub fn get_route(&self) -> String {
        match self {
            Page::Pigs(data) => {
                if let Some(pig) = &data.selection {
                    let id_str = pig.id.to_string();
                    yuri!(PIG_PAGE, id_str)
                } else {
                    PIG_PAGE.to_owned()
                }
            }
            _ => APP_ROOT.to_string(),
        }
    }

    pub fn get_pig_page_data(&mut self) -> Option<&mut PigPageData> {
        match self {
            Page::Pigs(data) => Some(data),
            _ => None,
        }
    }
}
