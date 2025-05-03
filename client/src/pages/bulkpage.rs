use crate::data::api::{BulkApi, PigApi};
use crate::data::state::ClientState;
use crate::pages::RenderPage;
use crate::ui::modal::Modal;
use crate::ui::style::{THEME_ACCEPTED, THEME_REJECTED, TIME_FMT};
use crate::ui::{add_properties_row, properties_list, selectable_list};
use chrono::Local;
use eframe::emath::Align;
use egui::{Button, CentralPanel, Context, Label, Layout, RichText, ScrollArea, Sense, SidePanel, Ui, Widget};
use egui_extras::{Column, TableBuilder};
use pigweb_common::bulk::{BulkImport, BulkQuery};
use pigweb_common::pigs::{Pig, PigQuery};
use pigweb_common::users::Roles;
use std::mem;
use urlable::ParsedURL;

// ( ͡° ͜ʖ ͡°)
#[derive(Debug)]
pub enum DirtyAction {
    SelectImport(Option<BulkImport>),
    SelectPig(Option<SelectedImportedPig>),
    None,
}

impl PartialEq for DirtyAction {
    fn eq(&self, other: &Self) -> bool {
        mem::discriminant(self) == mem::discriminant(other)
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
enum SelectedImportedPig {
    Pending(String),
    Accepted(Pig),
    Rejected(String),
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct BulkPage {
    pub selected_import: Option<BulkImport>,

    pub selected_pig: Option<SelectedImportedPig>,

    /// Whether we have unsaved changes
    dirty: bool,
}

impl Default for BulkPage {
    fn default() -> Self {
        Self { selected_import: None, selected_pig: None, dirty: false }
    }
}

pub struct BulkPageRender {
    bulk_api: BulkApi,
    pig_api: PigApi,
    all_imports: Option<Vec<BulkImport>>,
    accepted_pigs: Option<Vec<Pig>>,
    dirty_modal: DirtyAction,
    raw_names: String,
}

impl Default for BulkPageRender {
    fn default() -> Self {
        Self {
            bulk_api: BulkApi::default(),
            pig_api: PigApi::default(),
            all_imports: None,
            accepted_pigs: None,
            dirty_modal: DirtyAction::None,
            raw_names: String::default(),
        }
    }
}

impl RenderPage for BulkPageRender {
    fn open(&mut self, _ctx: &Context, state: &mut ClientState, _url: &ParsedURL) {
        self.do_query();
        self.update_accepted_pigs(state);
    }

    fn ui(&mut self, ui: &mut Ui, state: &mut ClientState, url: &ParsedURL) {
        if !(state.has_role(Roles::BulkEditor) || state.has_role(Roles::BulkAdmin)) {
            // TODO 403 Forbidden
            return;
        }

        self.process_promises(ui.ctx(), state, url);

        SidePanel::left("left_panel").resizable(false).show(ui.ctx(), |ui| {
            self.populate_sidebar(ui, state, url);
        });

        self.populate_center(ui, state, url);

        self.show_modals(ui.ctx(), state, url);
    }
}

impl BulkPageRender {
    fn process_promises(&mut self, ctx: &Context, state: &mut ClientState, url: &ParsedURL) {
        if let Some(import) = self.bulk_api.create.received(state) {
            state.pages.bulk.dirty = false;
            state.pages.bulk.selected_import = Some(import);
            self.raw_names = String::default();
            self.do_query();
            self.update_accepted_pigs(state);
        }

        if self.bulk_api.patch.received(state).is_some() {
            state.pages.bulk.dirty = false;
            self.do_query();
        }

        if let Some(mut imports) = self.bulk_api.fetch.received(state) {
            imports.reverse(); // show newest first
            self.all_imports = Some(imports);
        }

        if let Some(pigs) = self.pig_api.fetch.received(state) {
            self.accepted_pigs = Some(pigs);
        }
    }

    fn populate_sidebar(&mut self, ui: &mut Ui, state: &mut ClientState, url: &ParsedURL) {
        ui.set_width(320.0);
        ui.add_space(8.0);
        ui.heading("Bulk Imports");
        ui.add_space(8.0);

        // Only render the results table if we have results to show
        if self.all_imports.as_ref().is_some_and(|imports| !imports.is_empty()) {
            let clicked: Option<Option<BulkImport>> =
                selectable_list(ui, self.all_imports.as_ref().unwrap(), |row, import| {
                    // idfk why this wants us to clone selection, otherwise page is supposedly moved
                    let selected =
                        state.pages.bulk.selected_import.as_ref().is_some_and(|select| select.id == import.id);
                    row.set_selected(selected);

                    // Make sure we can't select the text or else we can't click the row behind
                    row.col(|ui| {
                        let start_time = import.started.and_utc().with_timezone(&Local);
                        Label::new(start_time.format(TIME_FMT).to_string() + " " + import.name.as_str())
                            .selectable(false)
                            .truncate()
                            .ui(ui);
                    });

                    selected
                });

            // Check if we have an action to do
            if let Some(clicked) = clicked {
                self.warn_if_dirty(ui.ctx(), state, url, DirtyAction::SelectImport(clicked));
            }
        } else if self.all_imports.is_none() {
            // Still waiting on results, this should only happen when waiting
            // since otherwise it'll be an empty vec

            // You spin me right 'round, baby, 'right round
            // Like a record, baby, right 'round, 'round, 'round
            ui.vertical_centered(|ui| ui.spinner());
        }
    }

    fn populate_center(&mut self, ui: &mut Ui, state: &mut ClientState, url: &ParsedURL) {
        if let Some(import) = state.pages.bulk.selected_import.as_ref() {
            if import.finished.is_some() {
                self.populate_center_finished(ui, state, url);
            } else {
                self.populate_center_edit(ui, state, url);
            }
        } else {
            CentralPanel::default().show(ui.ctx(), |ui| {
                ui.vertical_centered(|ui| {
                    self.populate_center_create(ui, state, url);
                });
            });
        }
    }

    fn populate_center_create(&mut self, ui: &mut Ui, state: &mut ClientState, url: &ParsedURL) {
        ui.set_max_width(540.0);
        state.colorix.draw_background(ui.ctx(), false);

        // Title
        ui.add_space(8.0);
        ui.heading("Paste Names Below");
        ui.add_space(8.0);

        let add_button = Button::new("+ Add All Pigs");
        if ui.add_enabled(!self.raw_names.is_empty(), add_button).clicked() {
            let names = self.raw_names.lines().map(|l: &str| l.to_string()).collect::<Vec<String>>();
            self.bulk_api.create.request(&names);
        }

        ui.centered_and_justified(|ui| {
            ScrollArea::vertical().show(ui, |ui| {
                ui.text_edit_multiline(&mut self.raw_names);
            });
        });
    }

    fn populate_center_edit(&mut self, ui: &mut Ui, state: &mut ClientState, url: &ParsedURL) {
        // TODO
    }

    fn populate_center_finished(&mut self, ui: &mut Ui, state: &mut ClientState, url: &ParsedURL) {
        SidePanel::right("added_pigs").resizable(false).show(ui.ctx(), |ui| {
            ui.set_width(320.0);

            self.selectable_mixed_list(ui, state, url);
        });

        CentralPanel::default().show(ui.ctx(), |ui| {
            ui.vertical_centered(|ui| {
                ui.set_max_width(540.0);
                state.colorix.draw_background(ui.ctx(), false);
                let is_admin = state.has_role(Roles::BulkAdmin);

                // Title
                ui.add_space(8.0);
                ui.heading("Import Complete");
                ui.add_space(8.0);

                if let Some(import) = state.pages.bulk.selected_import.as_mut() {
                    properties_list(ui).body(|mut body| {
                        add_properties_row(&mut body, 40.0, "id", |ui| {
                            ui.code(import.id.to_string());
                        });

                        if is_admin {
                            add_properties_row(&mut body, 40.0, "created by", |ui| {
                                // TODO actually bother fetching the user data
                                ui.code(import.creator.to_string());
                            });
                        }

                        add_properties_row(&mut body, 40.0, "started at", |ui| {
                            let start_time = import.started.and_utc().with_timezone(&Local);
                            ui.label(start_time.format(TIME_FMT).to_string());
                        });

                        if let Some(finished) = import.finished {
                            add_properties_row(&mut body, 40.0, "finished at", |ui| {
                                let finish_time = finished.and_utc().with_timezone(&Local);
                                ui.label(finish_time.format(TIME_FMT).to_string());
                            });
                        }

                        add_properties_row(&mut body, 40.0, "accepted", |ui| {
                            ui.label(import.accepted.len().to_string());
                        });

                        add_properties_row(&mut body, 40.0, "rejected", |ui| {
                            ui.label(import.rejected.len().to_string());
                        });
                    });
                }
            });
        });
    }

    fn show_modals(&mut self, ctx: &Context, state: &mut ClientState, url: &ParsedURL) {
        if self.dirty_modal != DirtyAction::None {
            let modal = Modal::new("dirty")
                .with_heading("Discard Unsaved Changes")
                .with_body("Are you sure you want to continue and discard your current changes? There's no going back after this!")
                .show_with_extras(ctx, |ui| {
                    if ui.button("✔ Yes").clicked() {
                        self.do_dirty_action(ui.ctx(), state, url);
                    }
                });

            if modal.should_close() {
                self.dirty_modal = DirtyAction::None;
            }
        }
    }

    /// Sends a fetch request for all results of the current query and clears
    /// the list of current results
    fn do_query(&mut self) {
        self.all_imports = None;
        self.bulk_api.fetch.request(&BulkQuery::default());
    }

    fn update_accepted_pigs(&mut self, state: &mut ClientState) {
        self.accepted_pigs = None;
        if let Some(selected_import) = state.pages.bulk.selected_import.as_ref() {
            let len = selected_import.accepted.len();
            let query = PigQuery::default().with_ids(&selected_import.accepted).with_limit(len as u32);
            self.pig_api.fetch.request(query);
        }
    }

    /// If the dirty var is true, warn the user with a modal before performing
    /// the given action; otherwise, just do it
    fn warn_if_dirty(&mut self, ctx: &Context, state: &mut ClientState, url: &ParsedURL, action: DirtyAction) {
        self.dirty_modal = action;

        // If the state isn't dirty, execute the action right away
        // else if dirty_modal is not None, it will be shown
        if !state.pages.bulk.dirty {
            self.do_dirty_action(ctx, state, url);
        }
    }

    fn do_dirty_action(&mut self, ctx: &Context, state: &mut ClientState, url: &ParsedURL) {
        match &self.dirty_modal {
            DirtyAction::SelectImport(selection) => {
                // Change the selection
                state.pages.bulk.selected_import = selection.clone();
                if state.pages.bulk.selected_import.is_none() {
                    state.pages.bulk.selected_pig = None;
                }
                self.update_accepted_pigs(state);
            }
            DirtyAction::SelectPig(selection) => {
                state.pages.bulk.selected_pig = selection.clone();
            }
            DirtyAction::None => {}
        }

        // Reset dirty state, how tf did i forget this?
        self.dirty_modal = DirtyAction::None;
        state.pages.bulk.dirty = false;
    }

    pub fn selectable_mixed_list(&mut self, ui: &mut Ui, state: &mut ClientState, url: &ParsedURL) {
        let mut clicked: Option<Option<SelectedImportedPig>> = None;

        if let Some(import) = state.pages.bulk.selected_import.as_ref() {
            TableBuilder::new(ui)
                .striped(true)
                .resizable(false)
                .column(Column::remainder())
                .sense(Sense::click())
                .cell_layout(Layout::left_to_right(Align::Center))
                .body(|mut body| {
                    import.pending.iter().for_each(|e| {
                        body.row(18.0, |mut row| {
                            let selected = state.pages.bulk.selected_pig.as_ref().is_some_and(|sel| match sel {
                                SelectedImportedPig::Pending(name) => name == e,
                                _ => false,
                            });

                            row.set_selected(selected);

                            // Make sure we can't select the text or else we can't click the row behind
                            row.col(|ui| {
                                Label::new(e).selectable(false).truncate().ui(ui);
                            });

                            if row.response().clicked() {
                                if selected {
                                    clicked = Some(None);
                                } else {
                                    clicked = Some(Some(SelectedImportedPig::Pending(e.clone())));
                                }
                            }
                        });
                    });

                    if let Some(accepted) = self.accepted_pigs.as_ref() {
                        accepted.iter().for_each(|e| {
                            body.row(18.0, |mut row| {
                                let selected = state.pages.bulk.selected_pig.as_ref().is_some_and(|sel| match sel {
                                    SelectedImportedPig::Accepted(pig) => pig.id == e.id,
                                    _ => false,
                                });

                                row.set_selected(selected);

                                // Make sure we can't select the text or else we can't click the row behind
                                row.col(|ui| {
                                    Label::new(RichText::new(&e.name).color(THEME_ACCEPTED))
                                        .selectable(false)
                                        .truncate()
                                        .ui(ui);
                                });

                                if row.response().clicked() {
                                    if selected {
                                        clicked = Some(None);
                                    } else {
                                        clicked = Some(Some(SelectedImportedPig::Accepted(e.clone())));
                                    }
                                }
                            });
                        });
                    }

                    import.rejected.iter().for_each(|e| {
                        body.row(18.0, |mut row| {
                            let selected = state.pages.bulk.selected_pig.as_ref().is_some_and(|sel| match sel {
                                SelectedImportedPig::Rejected(name) => name == e,
                                _ => false,
                            });

                            row.set_selected(selected);

                            // Make sure we can't select the text or else we can't click the row behind
                            row.col(|ui| {
                                Label::new(RichText::new(e).color(THEME_REJECTED)).selectable(false).truncate().ui(ui);
                            });

                            if row.response().clicked() {
                                if selected {
                                    clicked = Some(None);
                                } else {
                                    clicked = Some(Some(SelectedImportedPig::Rejected(e.clone())));
                                }
                            }
                        });
                    });
                });

            // Check if we have an action to do
            if let Some(clicked) = clicked {
                self.warn_if_dirty(ui.ctx(), state, url, DirtyAction::SelectPig(clicked));
            }
        }
    }
}
