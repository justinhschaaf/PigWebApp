mod app;
mod data;
mod modal;
mod pages;
mod style;

pub use app::PigWebClient;
use egui::{Context, OpenUrl};
use urlable::ParsedURL;
use uuid::Uuid;

/// Updates the hash on the URL to the given UUID if it is Some, else
/// removes the hash from the URL. Then, asks egui to navigate to the new
/// URL.
pub fn update_url_hash(ctx: &Context, url: &ParsedURL, uuid: Option<Uuid>) {
    let mut dest = url.clone();
    dest.hash = "#".to_owned() + uuid.map(|id| id.to_string()).unwrap_or("".to_owned()).as_str();
    ctx.open_url(OpenUrl::same_tab(dest.stringify()));
}
