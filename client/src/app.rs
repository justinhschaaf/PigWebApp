use crate::app::Page::{Logs, Pigs, System, Users};
use egui::TextStyle::Button;
use egui::{
    menu, widgets, Align, CentralPanel, Context, Label, Layout, SelectableLabel, Sense, SidePanel, TextEdit,
    TopBottomPanel, Ui, ViewportCommand, Widget,
};
use egui_extras::Column;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
enum Page {
    Pigs,
    Logs,
    Users,
    System,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct PigWebClient {
    // The currently open page, see above for options
    page: Page,

    // The current search query, opt out of serialization
    #[serde(skip)]
    query: String,

    // The currently selected row
    #[serde(skip)]
    row_selection: Option<usize>,
}

impl Default for PigWebClient {
    fn default() -> Self {
        Self { page: Pigs, query: String::default(), row_selection: None }
    }
}

impl PigWebClient {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }

    fn populate_menu(&mut self, ctx: &Context, ui: &mut Ui) {
        ui.add_space(2.0);

        widgets::global_theme_preference_switch(ui);

        ui.separator();

        // TODO only show pages you have access to
        ui.selectable_value(&mut self.page, Pigs, " 🐖 Pigs ");
        ui.add_enabled(false, SelectableLabel::new(false, " 📄 Logs "));
        ui.add_enabled(false, SelectableLabel::new(false, " 😐 Users "));
        ui.add_enabled(false, SelectableLabel::new(false, " ⛭ System "));

        // Show debug warning
        if cfg!(debug_assertions) {
            ui.separator();
            egui::warn_if_debug_build(ui);
        }

        // This right aligns it on the same row
        let is_web = cfg!(target_arch = "wasm32");
        ui.with_layout(Layout::right_to_left(Align::RIGHT), |ui| {
            // Show the quit button if somehow this gets run on desktop
            // (you shouldn't, dumbass)
            if !is_web && ui.button("🗙").clicked() {
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }

            // Logout
            if ui.button("⎆").clicked() {
                println!("TODO"); // TODO implement me
            }
        });
    }

    fn populate_sidebar(&mut self, ctx: &Context, ui: &mut Ui) {
        ui.add_space(8.0);
        ui.heading("The Pig List");
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.add(TextEdit::singleline(&mut self.query).hint_text("Search"));
            if ui.button("+ Add").clicked() {
                println!("TODO"); // TODO implement me
            }
        });

        ui.add_space(4.0);

        egui_extras::TableBuilder::new(ui)
            .striped(true)
            .resizable(false)
            .column(Column::remainder())
            .sense(Sense::click())
            .cell_layout(Layout::left_to_right(Align::Center))
            .body(|mut body| {
                body.rows(18.0, 1000, |mut row| {
                    let i = row.index();
                    row.set_selected(self.row_selection.is_some() && self.row_selection.unwrap() == i);

                    // Make sure we can't select the text or else we can't click the row behind
                    row.col(|ui| {
                        Label::new(format!("This is line {i}")).selectable(false).ui(ui);
                    });

                    if row.response().clicked() {
                        self.row_selection = Some(i);
                    }
                });
            });
    }
}

impl eframe::App for PigWebClient {
    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            menu::bar(ui, |ui| {
                self.populate_menu(ctx, ui);
            });
        });

        SidePanel::left("left_panel").show(ctx, |ui| {
            self.populate_sidebar(ctx, ui);
        });

        CentralPanel::default().show(ctx, |ui| {
            ui.label("hi :3");
        });
    }
}
