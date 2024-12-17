use crate::app::Page::{Logs, Pigs, System, Users};
use egui::TextStyle::Button;
use egui::{
    menu, vec2, widgets, Align, CentralPanel, Context, Direction, Grid, Label, Layout, ScrollArea, SelectableLabel,
    Sense, SidePanel, TextEdit, TopBottomPanel, Ui, ViewportCommand, Widget,
};
use egui_extras::{Column, TableBody};

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

    fn populate_center(&mut self, ctx: &Context, ui: &mut Ui) {
        ui.set_max_width(540.0);
        // Title
        ui.add_space(8.0);
        ui.heading("Pig Name Here");
        ui.add_space(8.0);

        // Pig action buttons
        ui.vertical_centered_justified(|ui| {
            if ui.button("💾 Save").clicked() {
                println!("TODO"); // TODO implement me
            }

            if ui.button("🗑 Delete").clicked() {
                println!("TODO"); // TODO implement me
            }
        });

        ui.add_space(4.0);

        egui_extras::TableBuilder::new(ui)
            .striped(true)
            .resizable(false)
            .column(Column::initial(180.0))
            .column(Column::remainder())
            .cell_layout(Layout::left_to_right(Align::Center))
            .body(|mut body| {
                add_pig_properties_row(&mut body, 40.0, "id", |ui| {
                    ui.code("abcdefghijklmnopqrstuvwx");
                });

                add_pig_properties_row(&mut body, 80.0, "name", |ui| {
                    // yes, all this is necessary
                    // centered_and_justified makes the text box fill the value cell
                    // ScrollArea lets you scroll when it's too big
                    ui.centered_and_justified(|ui| {
                        ScrollArea::vertical().show(ui, |ui| {
                            ui.text_edit_multiline(&mut self.query);
                        });
                    });
                });

                add_pig_properties_row(&mut body, 40.0, "created by", |ui| {
                    ui.code("TODO dropdown");
                });

                add_pig_properties_row(&mut body, 40.0, "created on", |ui| {
                    ui.label("2024-12-24");
                });
            });
    }
}

impl eframe::App for PigWebClient {
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
            ui.vertical_centered(|ui| {
                self.populate_center(ctx, ui);
            });
        });
    }

    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        visuals.extreme_bg_color.to_normalized_gamma_f32()
    }
}

// This is out here because putting it in the struct causes a self-reference error
// it doesn't even need to use PigWebClient it's a fucking util method
fn add_pig_properties_row(body: &mut TableBody, height: f32, label: &str, add_value: impl FnOnce(&mut Ui)) {
    body.row(height, |mut row| {
        row.col(|ui| {
            ui.label(label);
        });

        row.col(add_value);
    });
}
