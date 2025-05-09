use crate::data::state::ClientState;
use crate::pages::RenderPage;
use crate::ui::spaced_heading;
use crate::ui::style::{PANEL_WIDTH_SMALL, SPACE_LARGE, SPACE_MEDIUM};
use egui::{Button, CentralPanel, OpenUrl, Ui};
use egui_flex::{item, Flex, FlexJustify};
use urlable::ParsedURL;

/// Responsible for rendering [`crate::pages::Routes::NotFound`] (and possibly
/// other errors)
pub struct ErrPageRender {
    /// The title/headline of the error
    head: String,

    /// A further explanation of the error
    body: String,
}

impl Default for ErrPageRender {
    fn default() -> Self {
        Self::not_found()
    }
}

impl RenderPage for ErrPageRender {
    fn ui(&mut self, ui: &mut Ui, state: &mut ClientState, _url: &ParsedURL) {
        CentralPanel::default().show(ui.ctx(), |ui| {
            state.colorix.draw_background(ui.ctx(), false);
            ui.vertical_centered(|ui| {
                ui.set_width(PANEL_WIDTH_SMALL);
                ui.add_space(SPACE_LARGE);
                spaced_heading(ui, self.head.as_str());

                ui.label(self.body.as_str());
                ui.add_space(SPACE_MEDIUM);

                ui.separator();
                let mut go_home = false;

                Flex::horizontal().w_full().justify(FlexJustify::SpaceBetween).show(ui, |flex| {
                    let btn = Button::new("Go home?");
                    if flex.add(item().grow(1.0), btn).clicked() {
                        go_home = true;
                    }
                });

                if go_home {
                    ui.ctx().open_url(OpenUrl::same_tab("/"))
                }
            });
        });
    }
}

impl ErrPageRender {
    /// Creates a renderer for 404 not found errors
    fn not_found() -> Self {
        Self { head: "Page Not Found".to_owned(), body: "That pig is in another castle!".to_owned() }
    }
}
