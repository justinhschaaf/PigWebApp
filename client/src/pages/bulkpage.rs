use crate::data::api::{ApiError, BulkApi, BulkFetchHandler, PigApi, PigFetchHandler};
use crate::data::state::ClientState;
use crate::pages::RenderPage;
use crate::ui::modal::Modal;
use crate::ui::style::{THEME_ACCEPTED, THEME_REJECTED, TIME_FMT};
use crate::ui::{add_properties_row, properties_list, selectable_list};
use crate::update_url_hash;
use chrono::Local;
use eframe::emath::Align;
use egui::{Button, CentralPanel, Context, Label, Layout, OpenUrl, RichText, ScrollArea, Sense, SidePanel, Ui, Widget};
use egui_extras::{Column, TableBuilder};
use log::{debug, error};
use pigweb_common::bulk::{BulkImport, BulkQuery};
use pigweb_common::pigs::{Pig, PigQuery};
use pigweb_common::users::Roles;
use std::mem;
use urlable::ParsedURL;
use uuid::Uuid;

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
    fetch_url_selection: BulkFetchHandler,
    fetch_accepted_pigs: PigFetchHandler,
    all_imports: Option<Vec<BulkImport>>,
    accepted_pigs: Option<Vec<Pig>>,
    dirty_modal: DirtyAction,
    raw_names: String,
    not_found_modal: bool,
}

impl Default for BulkPageRender {
    fn default() -> Self {
        Self {
            bulk_api: BulkApi::default(),
            fetch_url_selection: BulkFetchHandler::default(),
            fetch_accepted_pigs: PigFetchHandler::default(),
            all_imports: None,
            accepted_pigs: None,
            dirty_modal: DirtyAction::None,
            raw_names: String::default(),
            not_found_modal: false,
        }
    }
}

impl RenderPage for BulkPageRender {
    fn on_url_update(&mut self, ctx: &Context, state: &mut ClientState, url: &ParsedURL) {
        // url.hash and self.last_hash must have the # character in it for previous checks to work
        // for the logic below, it depends on that character being gone
        let stripped_hash = url.hash.replacen('#', "", 1);
        if !stripped_hash.is_empty() {
            // convert slug to uuid
            match Uuid::try_parse(stripped_hash.as_str()) {
                Ok(uuid) => {
                    // If we don't have a selection or the slug doesn't equal the
                    // current selection, fetch the data of the desired pig
                    if state.pages.bulk.selected_import.as_ref().is_none_or(|selected| uuid != selected.id) {
                        debug!(
                            "The selection has been updated via url! Previous Selection: {:?}",
                            state.pages.bulk.selected_import.as_ref()
                        );
                        self.fetch_url_selection.request(&BulkQuery::default().with_id(&uuid).with_limit(1));
                    }
                }
                Err(err) => {
                    state.pages.layout.display_error =
                        Some(ApiError::new(err.to_string()).with_reason("Unable to parse UUID.".to_owned()));
                    update_url_hash(ctx, url, None);
                    error!("Unable to parse hash \"{:?}\", err: {:?}", &stripped_hash, err);
                }
            }
        } else if state.pages.bulk.selected_import.is_some() {
            // if we have a pig selected, deselect it
            debug!("Hash is empty but selection is {:?}, selecting None!", state.pages.bulk.selected_import.as_ref());
            self.warn_if_dirty(ctx, state, url, DirtyAction::SelectImport(None));
        }
    }

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
            update_url_hash(ctx, url, Some(state.pages.bulk.selected_import.as_ref().unwrap().id));
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

        if let Some(mut imports) = self.fetch_url_selection.received(state) {
            // This request should have been made with limit = 1
            // therefore, the only pig is the one we want
            if let Some(sel) = imports.pop() {
                self.warn_if_dirty(ctx, state, url, DirtyAction::SelectImport(Some(sel)));
            } else {
                self.not_found_modal = true;
            }
        }

        if let Some(pigs) = self.fetch_accepted_pigs.received(state) {
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
        SidePanel::right("duplicate_pigs").resizable(false).show(ui.ctx(), |ui| {
            ui.set_width(320.0);

            ui.add_space(8.0);
            ui.heading("Duplicates");
            ui.add_space(8.0);

            // TODO add duplicates, query should equal edit box
        });

        CentralPanel::default().show(ui.ctx(), |ui| {
            ui.vertical_centered(|ui| {
                ui.set_max_width(540.0);
                state.colorix.draw_background(ui.ctx(), false);
                let is_admin = state.has_role(Roles::BulkAdmin);

                // Title
                ui.add_space(8.0);
                ui.heading("In Progress");
                ui.add_space(8.0);

                self.import_properties_list(ui, state, is_admin);

                ui.add_space(8.0);
                ui.heading("Add Names");
                ui.add_space(8.0);

                // TODO add edit box, add button, and open selected duplicate

                // forces the second table to take on a new id. there's an id conflict without this
                // due to the two tables in the one vertical_centered ui
                ui.push_id(69, |ui| {
                    self.selectable_mixed_list(ui, state, url);
                });
            });
        });
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

                let go_to_selection = Button::new("⮩ Go To Pig");
                if let SelectedImportedPig::Accepted(pig) =
                    state.pages.bulk.selected_pig.as_ref().unwrap_or(&SelectedImportedPig::Rejected("".to_owned()))
                {
                    if ui.add(go_to_selection).clicked() {
                        ui.ctx().open_url(OpenUrl::same_tab("/pigs#".to_owned() + pig.id.to_string().as_str()))
                    }
                } else {
                    ui.add_enabled(false, go_to_selection);
                }

                self.import_properties_list(ui, state, is_admin);
            });
        });
    }

    pub fn import_properties_list(&mut self, ui: &mut Ui, state: &mut ClientState, is_admin: bool) {
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

                let pending = import.pending.len();
                if pending > 0 {
                    add_properties_row(&mut body, 40.0, "pending", |ui| {
                        ui.label(pending.to_string());
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

        if self.not_found_modal {
            let modal = Modal::new("import_not_found")
                .with_heading("Import Not Found")
                .with_body("We couldn't find an import with that id.")
                .show(ctx);

            if modal.should_close() {
                // Close the modal
                self.not_found_modal = false;

                // Update the route
                update_url_hash(ctx, url, None);
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
            self.fetch_accepted_pigs.request(query);
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
                state.pages.bulk.selected_pig = None;
                update_url_hash(ctx, url, state.pages.bulk.selected_import.as_ref().and_then(|sel| Some(sel.id)));
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
}
