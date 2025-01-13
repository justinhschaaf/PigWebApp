use crate::app::Page::Pigs;
use crate::data::{ClientDataHandler, Status};
use crate::modal::Modal;
use egui::{
    menu, widgets, Align, CentralPanel, Context, Label, Layout, ScrollArea, SelectableLabel, Sense, SidePanel,
    TextEdit, TopBottomPanel, Ui, ViewportCommand, Widget,
};
use egui_extras::{Column, TableBody};
use log::error;
use pigweb_common::Pig;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
enum Page {
    Pigs,
    Logs,
    Users,
    System,
}

// ( ͡° ͜ʖ ͡°)
enum DirtyAction {
    Create(String),
    Select(Pig),
    None,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct PigWebClient {
    // The currently open page, see above for options
    page: Page,

    // Handles sending and receiving API data
    #[serde(skip)]
    data: ClientDataHandler,

    // The current search query, opt out of serialization
    #[serde(skip)]
    query: String,

    // The current list of search results
    #[serde(skip)]
    query_results: Option<Vec<Pig>>,

    // The currently selected pig
    selection: Option<Pig>,

    // Whether we have unsaved changes
    dirty: bool,

    // Whether to show the modal to confirm deleting a pig
    #[serde(skip)]
    delete_modal: bool,

    // Modal which warns you when there's unsaved changes
    #[serde(skip)]
    dirty_modal: bool,

    #[serde(skip)]
    dirty_modal_action: DirtyAction,

    // Whether to show the modal error messages are displayed on
    #[serde(skip)]
    error_modal: bool,

    // The message to display on the error modal
    #[serde(skip)]
    error_modal_msg: Option<String>,
}

impl Default for PigWebClient {
    fn default() -> Self {
        Self {
            page: Pigs,
            data: ClientDataHandler::default(),
            query: String::default(),
            query_results: Some(Vec::new()),
            selection: None,
            dirty: false,
            delete_modal: false,
            dirty_modal: false,
            dirty_modal_action: DirtyAction::None,
            error_modal: false,
            error_modal_msg: None,
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

    fn process_promises(&mut self) {
        match self.data.resolve_pig_create() {
            Status::Received(pig) => {
                self.dirty = false;
                self.selection = Some(pig);
                self.do_query(); // Redo the search query so it includes the new pig
            }
            Status::Errored(err) => self.warn_generic_error(err.to_owned()),
            Status::Pending => {}
        }

        match self.data.resolve_pig_update() {
            Status::Received(_) => {
                self.dirty = false;
                self.do_query(); // Redo the search query so it includes any possible changes
            }
            Status::Errored(err) => self.warn_generic_error(err.to_owned()),
            Status::Pending => {}
        }

        match self.data.resolve_pig_delete() {
            Status::Received(_) => {
                self.dirty = false;
                self.selection = None;
                self.do_query(); // Redo the search query to exclude the deleted pig
            }
            Status::Errored(err) => self.warn_generic_error(err.to_owned()),
            Status::Pending => {}
        }

        match self.data.resolve_pig_fetch() {
            Status::Received(pigs) => self.query_results = Some(pigs),
            Status::Errored(err) => self.warn_generic_error(err.to_owned()),
            Status::Pending => {}
        }
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
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            // Show the quit button if somehow this gets run on desktop
            // (you shouldn't, dumbass)
            if !is_web && ui.button("🗙").clicked() {
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }

            // Logout
            if ui.button("⎆").clicked() {
                todo!("Implement when user accounts are completed.");
            }
        });
    }

    fn populate_sidebar(&mut self, ctx: &Context, ui: &mut Ui) {
        ui.add_space(8.0);
        ui.heading("The Pig List");
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            // Search bar, perform a search if it's been changed
            if ui.add(TextEdit::singleline(&mut self.query).hint_text("Search")).changed() {
                if !self.query.is_empty() {
                    self.do_query();
                } else {
                    // We already know there's gonna be no results if your query is blank
                    self.query_results = Some(Vec::new());
                    self.data.discard_pig_fetch();
                }
            }

            // Pig create button, it's only enabled when you have something in the search bar
            ui.add_enabled_ui(self.query.is_empty(), |ui| {
                if ui.button("+ Add").clicked() {
                    self.warn_if_dirty(DirtyAction::Create(self.query.to_owned()));
                }
            });
        });

        ui.add_space(4.0);

        // Only render the results table if we have results to show
        if self.query_results.as_ref().is_some_and(|pigs| !pigs.is_empty()) {
            // let Some(pigs) = self.query_results.as_mut()
            egui_extras::TableBuilder::new(ui)
                .striped(true)
                .resizable(false)
                .column(Column::remainder())
                .sense(Sense::click())
                .cell_layout(Layout::left_to_right(Align::Center))
                .body(|mut body| {
                    let pigs = self.query_results.as_ref().unwrap();
                    // This means we don't have to clone the list every frame
                    let mut clicked: Option<Pig> = None;
                    pigs.iter().for_each(|pig| {
                        body.row(18.0, |mut row| {
                            // idfk why this wants us to clone selection, otherwise self is supposedly moved
                            row.set_selected(self.selection.as_ref().is_some_and(|select| select.id == pig.id));

                            // Make sure we can't select the text or else we can't click the row behind
                            row.col(|ui| {
                                Label::new(&pig.name).selectable(false).truncate().ui(ui);
                            });

                            // On click, check if we have to change the selection before processing it
                            if row.response().clicked() && !self.selection.as_ref().is_some_and(|sel| sel.id == pig.id)
                            {
                                // warn about unsaved changes, else JUST DO IT
                                // ...and we clone the clone because of fucking course we do D:<
                                clicked = Some(pig.clone());
                            }
                        });
                    });

                    // Check if we have an action to do
                    if clicked.is_some() {
                        self.warn_if_dirty(DirtyAction::Select(clicked.unwrap()));
                    }
                });
        } else if self.query_results.is_none() {
            // Still waiting on results, this should only happen when waiting
            // since otherwise it'll be an empty vec

            // You spin me right 'round, baby, 'right round
            // Like a record, baby, right 'round, 'round, 'round
            ui.vertical_centered(|ui| ui.spinner());
        }
    }

    fn populate_center(&mut self, ctx: &Context, ui: &mut Ui) {
        ui.set_max_width(540.0);

        if self.selection.is_some() {
            // THIS IS REALLY FUCKING IMPORTANT, LETS US MODIFY THE VALUE INSIDE THE OPTION
            let pig = self.selection.as_mut().unwrap();

            // Title
            ui.add_space(8.0);
            ui.heading(pig.name.to_owned()); // convert to owned since we transfer a mut reference later
            ui.add_space(8.0);

            // Pig action buttons
            ui.vertical_centered_justified(|ui| {
                ui.add_enabled_ui(self.dirty, |ui| {
                    if ui.button("💾 Save").clicked() {
                        self.data.request_pig_update(pig);
                    }
                });

                if ui.button("🗑 Delete").clicked() {
                    self.delete_modal = true;
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
                        ui.code(pig.id.to_string());
                    });

                    add_pig_properties_row(&mut body, 80.0, "name", |ui| {
                        // yes, all this is necessary
                        // centered_and_justified makes the text box fill the value cell
                        // ScrollArea lets you scroll when it's too big
                        ui.centered_and_justified(|ui| {
                            ScrollArea::vertical().show(ui, |ui| {
                                if ui.text_edit_multiline(&mut pig.name).changed() {
                                    self.dirty = true; // TODO strip newlines, or just make it a singleline with wrap
                                }
                            });
                        });
                    });

                    if false {
                        // disabled until implemented
                        add_pig_properties_row(&mut body, 40.0, "created by", |ui| {
                            ui.code("TODO dropdown");
                        });
                    }

                    add_pig_properties_row(&mut body, 40.0, "created on", |ui| {
                        ui.label(pig.created.to_string()); // TODO chrono https://docs.rs/chrono/latest/chrono/
                    });
                });
        }
    }

    fn show_modals(&mut self, ctx: &Context) {
        if self.delete_modal {
            let modal = Modal::new_with_extras(
                ctx,
                "delete",
                "Confirm Deletion",
                "Are you sure you want to delete this pig? There's no going back after this!",
                |ui| {
                    if ui.button("✔ Yes").clicked() {
                        match self.selection.as_ref() {
                            Some(pig) => self.data.request_pig_delete(pig.id),
                            None => self.warn_generic_error(
                                "You tried to delete a pig without having one selected, how the fuck did you manage that?"
                                    .to_owned(),
                            ),
                        }
                        self.delete_modal = false;
                    }
                },
            );

            if modal.should_close() {
                self.delete_modal = false;
            }
        }

        if self.dirty_modal {
            let modal = Modal::new_with_extras(
                ctx,
                "dirty",
                "Discard Unsaved Changes",
                "Are you sure you want to continue and discard your current changes? There's no going back after this!",
                |ui| {
                    if ui.button("✔ Yes").clicked() {
                        self.do_dirty_action();
                        self.dirty_modal = false;
                    }
                },
            );

            if modal.should_close() {
                self.dirty_modal = false;
            }
        }

        if self.error_modal {
            if Modal::new(
                ctx,
                "Error",
                "Error",
                self.error_modal_msg.as_ref().unwrap_or(&mut "How did we get here?".to_owned()),
            )
            .should_close()
            {
                self.error_modal = false;
            }
        }
    }

    /// If the dirty var is true, warn the user with a modal before performing
    /// the given action; otherwise, just do it
    fn warn_if_dirty(&mut self, action: DirtyAction) {
        self.dirty_modal_action = action;

        if self.dirty {
            self.dirty_modal = true;
        } else {
            self.do_dirty_action();
        }
    }

    /// Sets the error modal's message, marks the modal to be shown, and logs
    /// the error
    fn warn_generic_error(&mut self, msg: String) {
        error!("{}", msg);
        self.error_modal = true;
        self.error_modal_msg = Some(msg);
    }

    /// Sends a fetch request for all results of the current query and clears
    /// the list of current results
    fn do_query(&mut self) {
        self.query_results = None;
        self.data.request_pig_fetch(&self.query);
    }

    fn do_dirty_action(&mut self) {
        match &self.dirty_modal_action {
            DirtyAction::Create(name) => self.data.request_pig_create(name),
            DirtyAction::Select(pig) => self.selection = Some(pig.to_owned()),
            DirtyAction::None => {}
        }
    }
}

impl eframe::App for PigWebClient {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Handle all the incoming data
        self.process_promises();

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

        self.show_modals(ctx);
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
fn add_pig_properties_row(body: &mut TableBody<'_>, height: f32, label: &str, add_value: impl FnOnce(&mut Ui)) {
    body.row(height, |mut row| {
        row.col(|ui| {
            ui.label(label);
        });

        row.col(add_value);
    });
}
