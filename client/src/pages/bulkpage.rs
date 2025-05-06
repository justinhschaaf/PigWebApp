use crate::data::api::{ApiError, BulkApi, BulkFetchHandler, PigCreateHandler, PigFetchHandler};
use crate::data::state::ClientState;
use crate::pages::RenderPage;
use crate::ui::modal::Modal;
use crate::ui::style::{
    COLOR_ACCEPTED, COLOR_REJECTED, PANEL_WIDTH_MEDIUM, PANEL_WIDTH_SMALL, SPACE_MEDIUM, TABLE_ROW_HEIGHT_LARGE,
    TABLE_ROW_HEIGHT_SMALL, TIME_FMT,
};
use crate::ui::{add_properties_row, properties_list, selectable_list, spaced_heading, wrapped_singleline_layouter};
use crate::update_url_hash;
use chrono::Local;
use egui::{
    Align, Button, CentralPanel, Context, Label, Layout, OpenUrl, RichText, ScrollArea, Sense, SidePanel, TextEdit, Ui,
    Widget,
};
use egui_extras::{Column, TableBuilder};
use log::{debug, error};
use pigweb_common::bulk::{BulkImport, BulkPatch, BulkQuery, PatchAction};
use pigweb_common::pigs::{Pig, PigQuery};
use pigweb_common::users::Roles;
use urlable::ParsedURL;
use uuid::Uuid;

/// An action which should only be performed when there are no unsaved changes.
/// When this isn't [BulkPageDirtyAction::None], shows a modal with a warning
/// before performing the action and resetting itself to None.
// ( ͡° ͜ʖ ͡°)
#[derive(Debug)]
pub enum BulkPageDirtyAction {
    /// Select a different import
    SelectImport(Option<BulkImport>),

    /// Select a different pig
    SelectPig(Option<SelectedImportedPig>),

    /// No pending action, don't prompt the user for anything
    None,
}

/// A pig selected from the [`BulkImport`] list. Unifies selections from the
/// pending, accepted, and rejected lists into one. To render the unified list,
/// see [`BulkPageRender::selectable_mixed_list`].
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum SelectedImportedPig {
    Pending(String),
    Accepted(Pig),
    Rejected(String),
}

/// Persistent data storage for [`crate::pages::Routes::Bulk`].
#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct BulkPage {
    /// The currently selected import
    pub selected_import: Option<BulkImport>,

    /// The currently selected pig/name in the import
    pub selected_pig: Option<SelectedImportedPig>,

    /// When changing a pending name, save it here instead of modifying the
    /// [`selected_pig`] to prevent sync issues
    pub updated_name: String,

    /// Whether we have unsaved changes
    dirty: bool,
}

impl Default for BulkPage {
    fn default() -> Self {
        Self { selected_import: None, selected_pig: None, updated_name: String::default(), dirty: false }
    }
}

/// Responsible for rendering [`crate::pages::Routes::Bulk`]
pub struct BulkPageRender {
    /// Handles sending and receiving API data
    bulk_api: BulkApi,

    /// Handles API data specifically when getting the selection from the URL
    fetch_url_selection: BulkFetchHandler,

    /// Handles API data to load the full data for all accepted pigs in the
    /// [`BulkImport`]
    fetch_accepted_pigs: PigFetchHandler,

    /// Handles API data to load any duplicate pigs from the currently selected
    /// pending name
    fetch_duplicate_pigs: PigFetchHandler,

    /// Handles API data when creating a pig from a pending name
    create_pig: PigCreateHandler,

    /// All imports the user has access to see, shows up on the sidebar
    all_imports: Option<Vec<BulkImport>>,

    /// The full data for all accepted pigs in the [`BulkImport`]
    accepted_pigs: Option<Vec<Pig>>,

    /// All pigs similar to the selected pending name
    duplicate_pigs: Option<Vec<Pig>>,

    /// The selection pig from [duplicate_pigs]
    selected_duplicate: Option<Pig>,

    /// Modal which warns you when there's unsaved changes
    dirty_modal: BulkPageDirtyAction,

    /// The text box to paste the names you wish to import into
    raw_names: String,

    /// Whether to show the modal for a URL where no BulkImport exists
    not_found_modal: bool,
}

impl Default for BulkPageRender {
    fn default() -> Self {
        Self {
            bulk_api: BulkApi::default(),
            fetch_url_selection: BulkFetchHandler::default(),
            fetch_accepted_pigs: PigFetchHandler::default(),
            fetch_duplicate_pigs: PigFetchHandler::default(),
            create_pig: PigCreateHandler::default(),
            all_imports: None,
            accepted_pigs: None,
            duplicate_pigs: None,
            selected_duplicate: None,
            dirty_modal: BulkPageDirtyAction::None,
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
                    state
                        .pages
                        .layout
                        .display_error
                        .push(ApiError::new(err.to_string()).with_reason("Unable to parse UUID.".to_owned()));
                    update_url_hash(ctx, url, None);
                    error!("Unable to parse hash \"{:?}\", err: {:?}", &stripped_hash, err);
                }
            }
        } else if state.pages.bulk.selected_import.is_some() {
            // if we have a selection, update the hash to reflect it
            update_url_hash(ctx, url, state.pages.bulk.selected_import.as_ref().map(|sel| sel.id));
        }
    }

    fn open(&mut self, _ctx: &Context, state: &mut ClientState, _url: &ParsedURL) {
        self.query_imports();
        self.query_duplicates(state);
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

        // there's a different panel layout depending on the selected import and how done it is
        // hence creating the panels is handled in the function instead of here
        self.populate_center(ui, state, url);

        self.show_modals(ui.ctx(), state, url);
    }
}

impl BulkPageRender {
    /// Checks all APIs for data received from previously submitted requests
    fn process_promises(&mut self, ctx: &Context, state: &mut ClientState, url: &ParsedURL) {
        // import was created
        if let Some(import) = self.bulk_api.create.received(state) {
            state.pages.bulk.dirty = false;
            state.pages.bulk.selected_import = Some(import);
            self.raw_names = String::default();

            // refresh these things
            update_url_hash(ctx, url, Some(state.pages.bulk.selected_import.as_ref().unwrap().id));
            self.query_imports();
            self.update_accepted_pigs(state);
        }

        // did the submitted changes go through?
        if let Some(patch) = self.bulk_api.patch.received(state) {
            // update our lists to reflect the changes made by the patch
            if let Some(sel) = state.pages.bulk.selected_import.as_mut() {
                patch.update_import(sel);

                // if import is complete, auto refresh our selected import
                if sel.pending.len() == 0 {
                    self.fetch_url_selection.request(&BulkQuery::default().with_id(&sel.id));
                }

                // update our selected item in the list of all imports
                if let Some(imports) = self.all_imports.as_mut() {
                    let pos = imports.iter().position(|r| r.id.eq(&sel.id));
                    pos.and_then(|i| Some(imports[i] = sel.clone()));
                }
            } else {
                self.query_imports();
            }

            // reset the state
            self.update_accepted_pigs(state);
            self.duplicate_pigs = Some(Vec::new());
            self.selected_duplicate = None;
            state.pages.bulk.dirty = false;
            state.pages.bulk.selected_pig = None;
            state.pages.bulk.updated_name = String::default();

            // TODO automatically select next pending name?
        }

        // updates the left sidebar data
        if let Some(mut imports) = self.bulk_api.fetch.received(state) {
            imports.reverse(); // show newest first
            self.all_imports = Some(imports);
        }

        if let Some(mut imports) = self.fetch_url_selection.received(state) {
            // This request should have been made with limit = 1
            // therefore, the only pig is the one we want
            if let Some(sel) = imports.pop() {
                // this handler actually does both data from the url hash and data when the import
                // is finished since the core logic is the same. this here is only relevant in the
                // latter case.
                //
                // updates this item in the list of all imports with the fresh version
                if let Some(imports) = self.all_imports.as_mut() {
                    let pos = imports.iter().position(|r| r.id.eq(&sel.id));
                    pos.and_then(|i| Some(imports[i] = sel.clone()));
                }

                // change the selection
                self.warn_if_dirty(ctx, state, url, BulkPageDirtyAction::SelectImport(Some(sel)));
            } else {
                self.not_found_modal = true;
            }
        }

        if let Some(pigs) = self.fetch_accepted_pigs.received(state) {
            self.accepted_pigs = Some(pigs);
        }

        if let Some(pigs) = self.fetch_duplicate_pigs.received(state) {
            self.duplicate_pigs = Some(pigs);
        }

        // When a pig is created, submit a patch request to update the import
        if let Some(pig) = self.create_pig.received(state) {
            if let Some(import) = state.pages.bulk.selected_import.as_ref() {
                if let Some(sel) = state.pages.bulk.selected_pig.as_ref() {
                    match sel {
                        SelectedImportedPig::Pending(name) => {
                            let patch = BulkPatch::new(&import.id)
                                .pending(PatchAction::REMOVE(name.to_owned()))
                                .accepted(PatchAction::ADD(pig.id));
                            self.bulk_api.patch.request(patch);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// The sidebar listing all [`BulkImport`]s the user has access to
    fn populate_sidebar(&mut self, ui: &mut Ui, state: &mut ClientState, url: &ParsedURL) {
        ui.set_width(PANEL_WIDTH_SMALL);
        spaced_heading(ui, "Bulk Imports");

        // Only render the results table if we have results to show
        if self.all_imports.as_ref().is_some_and(|imports| !imports.is_empty()) {
            let clicked: Option<Option<BulkImport>> =
                selectable_list(ui, self.all_imports.as_ref().unwrap(), |row, import| {
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
                self.warn_if_dirty(ui.ctx(), state, url, BulkPageDirtyAction::SelectImport(clicked));
            }
        } else if self.all_imports.is_none() {
            // Still waiting on results, this should only happen when waiting
            // since otherwise it'll be an empty vec
            ui.vertical_centered(|ui| ui.spinner());
        }
    }

    /// Add the main content to the page, changes based on whether a
    /// [`BulkImport`] is selected and whether it's finished.
    fn populate_center(&mut self, ui: &mut Ui, state: &mut ClientState, url: &ParsedURL) {
        if let Some(import) = state.pages.bulk.selected_import.as_ref() {
            if import.finished.is_some() {
                // if we have a finished import selected, show the finished screen
                self.populate_center_finished(ui, state, url);
            } else {
                // if the selected import has pending names, show the editor
                self.populate_center_edit(ui, state, url);
            }
        } else {
            // show the create screen
            CentralPanel::default().show(ui.ctx(), |ui| {
                ui.vertical_centered(|ui| {
                    self.populate_center_create(ui, state);
                });
            });
        }
    }

    /// Shows the create screen in the center of the page
    fn populate_center_create(&mut self, ui: &mut Ui, state: &mut ClientState) {
        ui.set_max_width(PANEL_WIDTH_MEDIUM);
        state.colorix.draw_background(ui.ctx(), false);
        spaced_heading(ui, "Paste Names Below");

        // submit button
        let add_button = Button::new("+ Add All Pigs");
        if ui.add_enabled(!self.raw_names.is_empty(), add_button).clicked() {
            let names = self.raw_names.lines().map(|l: &str| l.to_string()).collect::<Vec<String>>();
            self.bulk_api.create.request(&names);
        }

        // text box to paste all names into
        ui.centered_and_justified(|ui| {
            ScrollArea::vertical().show(ui, |ui| {
                ui.text_edit_multiline(&mut self.raw_names);
            });
        });
    }

    /// Shows the edit screen in the center of the page
    fn populate_center_edit(&mut self, ui: &mut Ui, state: &mut ClientState, url: &ParsedURL) {
        // right sidepanel showing duplicates of the selected pending pig
        // this is added before the central panel because that must always come last
        SidePanel::right("duplicate_pigs").resizable(false).show(ui.ctx(), |ui| {
            ui.set_width(PANEL_WIDTH_SMALL);

            spaced_heading(ui, "Duplicates");

            // if we have anything in the name edit box and we have results to show
            if !state.pages.bulk.updated_name.is_empty()
                && self.duplicate_pigs.as_ref().is_some_and(|pigs| !pigs.is_empty())
            {
                let clicked: Option<Option<Pig>> =
                    selectable_list(ui, self.duplicate_pigs.as_ref().unwrap(), |row, pig| {
                        let selected = self.selected_duplicate.as_ref().is_some_and(|select| select.id == pig.id);
                        row.set_selected(selected);

                        // Make sure we can't select the text or else we can't click the row behind
                        row.col(|ui| {
                            Label::new(&pig.name).selectable(false).truncate().ui(ui);
                        });

                        selected
                    });

                // Check if we have an action to do
                if let Some(clicked) = clicked {
                    self.selected_duplicate = clicked;
                }
            } else if self.duplicate_pigs.is_none() {
                ui.vertical_centered(|ui| ui.spinner());
            }
        });

        // center panel with properties of the whole import and editor for the pending name
        CentralPanel::default().show(ui.ctx(), |ui| {
            ui.vertical_centered(|ui| {
                ui.set_max_width(PANEL_WIDTH_MEDIUM);
                state.colorix.draw_background(ui.ctx(), false);
                let is_admin = state.has_role(Roles::BulkAdmin);

                // show properties
                spaced_heading(ui, "In Progress");
                self.import_properties_list(ui, state, is_admin);

                // title for edit section
                spaced_heading(ui, "Add Names");

                // whether the currently selected pig to take action on is pending
                let selected_is_pending = state
                    .pages
                    .bulk
                    .selected_pig
                    .as_ref()
                    .is_some_and(|sel| matches!(sel, SelectedImportedPig::Pending(_)));

                // action buttons
                ui.horizontal(|ui| {
                    // Upon accepting the pig, submit a create request with what's in the edit box
                    let add_button = Button::new("+ Accept");
                    if ui
                        .add_enabled(selected_is_pending && !state.pages.bulk.updated_name.is_empty(), add_button)
                        .clicked()
                    {
                        self.create_pig.request(&state.pages.bulk.updated_name);
                    }

                    // Upon rejecting the name, submit a patch to remove it from the pending list and add it to the rejected list
                    let reject_button = Button::new("🗑 Reject");
                    if ui.add_enabled(selected_is_pending, reject_button).clicked() {
                        match state.pages.bulk.selected_pig.as_ref().unwrap() {
                            SelectedImportedPig::Pending(name) => {
                                let patch = BulkPatch::new(&state.pages.bulk.selected_import.as_ref().unwrap().id)
                                    .pending(PatchAction::REMOVE(name.to_owned()))
                                    .rejected(PatchAction::ADD(name.to_owned()));
                                self.bulk_api.patch.request(patch);
                            }
                            _ => {}
                        }
                    }

                    let open_duplicate = Button::new("⮩ Go To Duplicate");
                    if ui.add_enabled(self.selected_duplicate.is_some(), open_duplicate).clicked() {
                        ui.ctx().open_url(OpenUrl::same_tab(
                            "/pigs#".to_owned() + self.selected_duplicate.as_ref().unwrap().id.to_string().as_str(),
                        ))
                    }
                });

                ui.add_space(SPACE_MEDIUM);

                // edit text box
                let mut layouter = wrapped_singleline_layouter();
                let te = TextEdit::singleline(&mut state.pages.bulk.updated_name)
                    .desired_rows(4)
                    .layouter(&mut layouter)
                    .desired_width(PANEL_WIDTH_MEDIUM);
                if ui.add_enabled(selected_is_pending, te).changed() {
                    state.pages.bulk.dirty = true;
                    self.query_duplicates(state);
                }

                ui.add_space(SPACE_MEDIUM);

                // forces the second table to take on a new id. there's an id conflict without this
                // due to the two tables in the one vertical_centered ui
                ui.push_id(69, |ui| {
                    self.selectable_mixed_list(ui, state, url);
                });
            });
        });
    }

    /// Shows the import when there are no remaining names to add
    fn populate_center_finished(&mut self, ui: &mut Ui, state: &mut ClientState, url: &ParsedURL) {
        // center always comes last
        SidePanel::right("added_pigs").resizable(false).show(ui.ctx(), |ui| {
            ui.set_width(PANEL_WIDTH_SMALL);

            // show all names which were a part of this import
            self.selectable_mixed_list(ui, state, url);
        });

        CentralPanel::default().show(ui.ctx(), |ui| {
            ui.vertical_centered(|ui| {
                ui.set_max_width(PANEL_WIDTH_MEDIUM);
                state.colorix.draw_background(ui.ctx(), false);
                let is_admin = state.has_role(Roles::BulkAdmin);

                // Title
                spaced_heading(ui, "Import Complete");

                // navigates to the currently selected pig in the right sidebar, assuming it was added
                let go_to_selection = Button::new("⮩ Go To Pig");
                if let SelectedImportedPig::Accepted(pig) =
                    state.pages.bulk.selected_pig.as_ref().unwrap_or(&SelectedImportedPig::Rejected(String::default()))
                {
                    if ui.add(go_to_selection).clicked() {
                        ui.ctx().open_url(OpenUrl::same_tab("/pigs#".to_owned() + pig.id.to_string().as_str()))
                    }
                } else {
                    // there is either no pig selected or the name was rejected, disable the button
                    ui.add_enabled(false, go_to_selection);
                }

                // show the import properties
                self.import_properties_list(ui, state, is_admin);
            });
        });
    }

    /// Adds a table with the [`BulkImport`] properties to the ui. Hides fields
    /// which the user should not see depending on their permission level
    pub fn import_properties_list(&mut self, ui: &mut Ui, state: &mut ClientState, is_admin: bool) {
        if let Some(import) = state.pages.bulk.selected_import.as_mut() {
            properties_list(ui).body(|mut body| {
                add_properties_row(&mut body, TABLE_ROW_HEIGHT_LARGE, "id", |ui| {
                    ui.code(import.id.to_string());
                });

                // creator is only relevant if the user can see imports which aren't theirs
                if is_admin {
                    add_properties_row(&mut body, TABLE_ROW_HEIGHT_LARGE, "created by", |ui| {
                        // TODO actually bother fetching the user data
                        ui.code(import.creator.to_string());
                    });
                }

                add_properties_row(&mut body, TABLE_ROW_HEIGHT_LARGE, "started at", |ui| {
                    let start_time = import.started.and_utc().with_timezone(&Local);
                    ui.label(start_time.format(TIME_FMT).to_string());
                });

                // only show finished time if we have it
                if let Some(finished) = import.finished {
                    add_properties_row(&mut body, TABLE_ROW_HEIGHT_LARGE, "finished at", |ui| {
                        let finish_time = finished.and_utc().with_timezone(&Local);
                        ui.label(finish_time.format(TIME_FMT).to_string());
                    });
                }

                // only show pending amount if we have it
                let pending = import.pending.len();
                if pending > 0 {
                    add_properties_row(&mut body, TABLE_ROW_HEIGHT_LARGE, "pending", |ui| {
                        ui.label(pending.to_string());
                    });
                }

                add_properties_row(&mut body, TABLE_ROW_HEIGHT_LARGE, "accepted", |ui| {
                    ui.label(import.accepted.len().to_string());
                });

                add_properties_row(&mut body, TABLE_ROW_HEIGHT_LARGE, "rejected", |ui| {
                    ui.label(import.rejected.len().to_string());
                });
            });
        }
    }

    /// Add the mixed list of pending names, accepted pigs, and rejected names
    /// to the ui, with one item in the list being selectable at a time.
    pub fn selectable_mixed_list(&mut self, ui: &mut Ui, state: &mut ClientState, url: &ParsedURL) {
        // whether an item in the list was clicked, and if so, whether it was selected or deselected
        let mut clicked: Option<Option<SelectedImportedPig>> = None;

        // if we have an import
        if let Some(import) = state.pages.bulk.selected_import.as_ref() {
            TableBuilder::new(ui)
                .striped(true)
                .resizable(false)
                .column(Column::remainder())
                .sense(Sense::click())
                .cell_layout(Layout::left_to_right(Align::Center))
                .body(|mut body| {
                    // add the pending names
                    import.pending.iter().for_each(|e| {
                        body.row(TABLE_ROW_HEIGHT_SMALL, |mut row| {
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

                    // add the accepted pigs with green name color
                    if let Some(accepted) = self.accepted_pigs.as_ref() {
                        accepted.iter().for_each(|e| {
                            body.row(TABLE_ROW_HEIGHT_SMALL, |mut row| {
                                let selected = state.pages.bulk.selected_pig.as_ref().is_some_and(|sel| match sel {
                                    SelectedImportedPig::Accepted(pig) => pig.id == e.id,
                                    _ => false,
                                });

                                row.set_selected(selected);

                                // Make sure we can't select the text or else we can't click the row behind
                                row.col(|ui| {
                                    Label::new(RichText::new(&e.name).color(COLOR_ACCEPTED))
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

                    // add the rejected names with red text color
                    import.rejected.iter().for_each(|e| {
                        body.row(TABLE_ROW_HEIGHT_SMALL, |mut row| {
                            let selected = state.pages.bulk.selected_pig.as_ref().is_some_and(|sel| match sel {
                                SelectedImportedPig::Rejected(name) => name == e,
                                _ => false,
                            });

                            row.set_selected(selected);

                            // Make sure we can't select the text or else we can't click the row behind
                            row.col(|ui| {
                                Label::new(RichText::new(e).color(COLOR_REJECTED)).selectable(false).truncate().ui(ui);
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

            // Check if a name was selected or deselected and request an update to the selection if so
            if let Some(clicked) = clicked {
                self.warn_if_dirty(ui.ctx(), state, url, BulkPageDirtyAction::SelectPig(clicked));
            }
        }
    }

    /// Show any page-specific modals which should be visible
    fn show_modals(&mut self, ctx: &Context, state: &mut ClientState, url: &ParsedURL) {
        if !matches!(self.dirty_modal, BulkPageDirtyAction::None) {
            if let Some(do_action) = Modal::dirty(ctx) {
                if do_action {
                    self.do_dirty_action(ctx, state, url);
                } else {
                    self.dirty_modal = BulkPageDirtyAction::None;
                }
            }
        }

        if self.not_found_modal {
            if Modal::not_found(ctx) {
                // Close the modal
                self.not_found_modal = false;

                // Update the route
                update_url_hash(ctx, url, None);
            }
        }
    }

    /// Sends a fetch request for all [`BulkImport`]s the user can see and
    /// clears the list of current results
    fn query_imports(&mut self) {
        self.all_imports = None;
        self.bulk_api.fetch.request(&BulkQuery::default());
    }

    /// Sends a fetch request for all duplicates of the currently selected
    /// pending name and clears the list of current results
    fn query_duplicates(&mut self, state: &mut ClientState) {
        self.duplicate_pigs = None;
        self.fetch_duplicate_pigs.request(PigQuery::default().with_name(&state.pages.bulk.updated_name));
    }

    /// Clears the list of data for accepted pigs in this [`BulkImport`] and
    /// requests fresh data
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
    fn warn_if_dirty(&mut self, ctx: &Context, state: &mut ClientState, url: &ParsedURL, action: BulkPageDirtyAction) {
        self.dirty_modal = action;

        // If the state isn't dirty, execute the action right away
        // else if dirty_modal is not None, it will be shown
        if !state.pages.bulk.dirty {
            self.do_dirty_action(ctx, state, url);
        }
    }

    /// Performs the dirty action, resets all relevant variables, and refreshes
    /// all relevant data
    fn do_dirty_action(&mut self, ctx: &Context, state: &mut ClientState, url: &ParsedURL) {
        match &self.dirty_modal {
            BulkPageDirtyAction::SelectImport(selection) => {
                // Change the selection
                state.pages.bulk.selected_import = selection.clone();
                state.pages.bulk.selected_pig = None;
                state.pages.bulk.updated_name = String::default();
                update_url_hash(ctx, url, state.pages.bulk.selected_import.as_ref().and_then(|sel| Some(sel.id)));
                self.update_accepted_pigs(state);
            }
            BulkPageDirtyAction::SelectPig(selection) => {
                // Changes the edit text box if the pig is still pending
                state.pages.bulk.updated_name = if selection.is_some() {
                    match selection.as_ref().unwrap() {
                        SelectedImportedPig::Pending(name) => name.to_owned(),
                        _ => String::default(),
                    }
                } else {
                    String::default()
                };
                state.pages.bulk.selected_pig = selection.clone();
                self.query_duplicates(state);
            }
            BulkPageDirtyAction::None => {}
        }

        // Reset dirty state, how tf did i forget this?
        self.dirty_modal = BulkPageDirtyAction::None;
        state.pages.bulk.dirty = false;
    }
}
