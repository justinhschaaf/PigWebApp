use egui::Layout;
use egui::TextStyle::Button;
use egui::WidgetType::SelectableLabel;
use crate::app::Page::{Logs, Pigs, System, Users};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.

#[derive(Debug, PartialEq)]
#[derive(serde::Deserialize, serde::Serialize)]
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

}

impl Default for PigWebClient {
    fn default() -> Self {
        Self {
            page: Pigs,
            query: String::default(),
        }
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

    fn populate_menu(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {

        ui.add_space(2.0);

        egui::widgets::global_theme_preference_switch(ui);

        ui.separator();

        // TODO only show pages you have access to
        ui.selectable_value(&mut self.page, Pigs, " 🐖 Pigs ");
        ui.add_enabled(false, egui::SelectableLabel::new(false, " 📄 Logs "));
        ui.add_enabled(false, egui::SelectableLabel::new(false, " 😐 Users "));
        ui.add_enabled(false, egui::SelectableLabel::new(false, " ⛭ System "));

        // Show debug warning
        if cfg!(debug_assertions) {
            ui.separator();
            egui::warn_if_debug_build(ui);
        }

        // This right aligns it on the same row
        let is_web = cfg!(target_arch = "wasm32");
        ui.with_layout(Layout::right_to_left(egui::Align::RIGHT), |ui| {

            // Show the quit button if somehow this gets run on desktop
            // (you shouldn't, dumbass)
            if !is_web && ui.button("🗙").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }

            // Logout
            ui.button("⎆");

        });

    }

}

impl eframe::App for PigWebClient {

    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                self.populate_menu(ctx, ui);
            });
        });

        egui::SidePanel::left("left_panel").show(ctx, |ui| {

            // The central panel the region left after adding TopPanel's and SidePanel's

            ui.add_space(8.0);
            ui.heading("The Pig List");
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.query).hint_text("Search"));
                ui.button("+ Add");
            });

            ui.add_space(4.0);

            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .show(ui, |ui| {
                    egui::Grid::new("pig_list")
                        .num_columns(1)
                        .striped(true)
                        .show(ui, |ui| {
                            for i in 0..1000 {
                                ui.with_layout(Layout::top_down_justified(egui::Align::LEFT), |ui| {
                                    ui.label(format!("This is line {i}"));
                                });
                                ui.end_row();
                            }
                        });
                });

        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("hi :3");
        });

    }
}
