use crate::data::api::{ApiError, PigApi, PigFetchHandler};
use crate::data::state::ClientState;
use crate::pages::RenderPage;
use crate::ui::modal::Modal;
use crate::ui::style::{PANEL_WIDTH_MEDIUM, PANEL_WIDTH_SMALL, SPACE_SMALL, TABLE_ROW_HEIGHT_LARGE, TIME_FMT};
use crate::ui::{add_properties_row, properties_list, selectable_list, spaced_heading, wrapped_singleline_layouter};
use crate::update_url_hash;
use chrono::Local;
use egui::{Button, CentralPanel, Context, Label, ScrollArea, SidePanel, TextEdit, Ui, Widget};
use egui_flex::{item, Flex, FlexJustify};
use log::{debug, error};
use pigweb_common::pigs::{Pig, PigQuery};
use pigweb_common::users::Roles;
use urlable::ParsedURL;
use uuid::Uuid;

/// An action which should only be performed when there are no unsaved changes.
/// When this isn't [PigPageDirtyAction::None], shows a modal with a warning
/// before performing the action and resetting itself to None.
// ( ͡° ͜ʖ ͡°)
#[derive(Debug)]
enum PigPageDirtyAction {
    /// Create a new pig, navigating away from the current selection
    Create(String),

    /// Select a different pig
    Select(Option<Pig>),

    /// No pending action, don't prompt the user for anything
    None,
}

/// Persistent data storage for [`crate::pages::Routes::Pigs`].
// shit we care about saving
#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct PigPage {
    /// The current search query
    query: String,

    /// The currently selected pig
    selection: Option<Pig>,

    /// Whether we have unsaved changes
    dirty: bool,
}

impl Default for PigPage {
    fn default() -> Self {
        Self { query: String::default(), selection: None, dirty: false }
    }
}

/// Responsible for rendering [`crate::pages::Routes::Pigs`]
// shit we don't care about saving as it's actively in use
pub struct PigPageRender {
    /// Handles sending and receiving API data
    pig_api: PigApi,

    /// Handles API data specifically when getting the selection from the URL
    fetch_url_selection: PigFetchHandler,

    /// The current list of search results
    query_results: Option<Vec<Pig>>,

    /// Modal which warns you when there's unsaved changes
    dirty_modal: PigPageDirtyAction,

    /// Whether to show the modal to confirm deleting a pig
    delete_modal: bool,

    /// Whether to show the modal for a URL where no pig exists
    pig_not_found_modal: bool,
}

impl Default for PigPageRender {
    fn default() -> Self {
        Self {
            pig_api: PigApi::default(),
            fetch_url_selection: PigFetchHandler::default(),
            query_results: None,
            dirty_modal: PigPageDirtyAction::None,
            delete_modal: false,
            pig_not_found_modal: false,
        }
    }
}

impl RenderPage for PigPageRender {
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
                    if state.pages.pigs.selection.as_ref().is_none_or(|selected| uuid != selected.id) {
                        debug!(
                            "The selection has been updated via url! Previous Selection: {:?}",
                            state.pages.pigs.selection.as_ref()
                        );
                        self.fetch_url_selection.request(PigQuery::default().with_id(&uuid).with_limit(1));
                    }
                }
                Err(err) => {
                    state.pages.layout.display_error =
                        Some(ApiError::new(err.to_string()).with_reason("Unable to parse UUID.".to_owned()));
                    update_url_hash(ctx, url, None);
                    error!("Unable to parse hash \"{:?}\", err: {:?}", &stripped_hash, err);
                }
            }
        } else if state.pages.pigs.selection.is_some() {
            // if we have a pig selected, deselect it
            debug!("Hash is empty but selection is {:?}, selecting None!", state.pages.pigs.selection.as_ref());
            self.warn_if_dirty(ctx, state, url, PigPageDirtyAction::Select(None));
        }
    }

    fn open(&mut self, _ctx: &Context, state: &mut ClientState, _url: &ParsedURL) {
        self.do_query(state)
    }

    fn ui(&mut self, ui: &mut Ui, state: &mut ClientState, url: &ParsedURL) {
        if !state.has_role(Roles::PigViewer) {
            // TODO 403 Forbidden
            return;
        }

        self.process_promises(ui.ctx(), state, url);

        SidePanel::left("left_panel").resizable(false).show(ui.ctx(), |ui| {
            self.populate_sidebar(ui, state, url);
        });

        CentralPanel::default().show(ui.ctx(), |ui| {
            ui.vertical_centered(|ui| {
                self.populate_center(ui, state);
            });
        });

        self.show_modals(ui.ctx(), state, url);
    }
}

impl PigPageRender {
    /// Checks all APIs for data received from previously submitted requests
    fn process_promises(&mut self, ctx: &Context, state: &mut ClientState, url: &ParsedURL) {
        if let Some(pig) = self.pig_api.create.received(state) {
            state.pages.pigs.dirty = false;
            state.pages.pigs.selection = Some(pig);
            update_url_hash(ctx, url, Some(state.pages.pigs.selection.as_ref().unwrap().id));
            self.do_query(state); // Redo the search query so it includes the new pig
        }

        if self.pig_api.update.received(state).is_some() {
            state.pages.pigs.dirty = false;
            self.do_query(state); // Redo the search query so it includes any possible changes
        }

        if self.pig_api.delete.received(state).is_some() {
            state.pages.pigs.dirty = false;
            state.pages.pigs.selection = None;
            update_url_hash(ctx, url, None);
            self.do_query(state); // Redo the search query to exclude the deleted pig
        }

        if let Some(pigs) = self.pig_api.fetch.received(state) {
            self.query_results = Some(pigs);
        }

        if let Some(mut pigs) = self.fetch_url_selection.received(state) {
            // This request should have been made with limit = 1
            // therefore, the only pig is the one we want
            if let Some(pig) = pigs.pop() {
                self.warn_if_dirty(ctx, state, url, PigPageDirtyAction::Select(Some(pig)));
            } else {
                self.pig_not_found_modal = true;
            }
        }
    }

    /// The sidebar listing all pigs which match the current search query
    fn populate_sidebar(&mut self, ui: &mut Ui, state: &mut ClientState, url: &ParsedURL) {
        ui.set_width(PANEL_WIDTH_SMALL);
        spaced_heading(ui, "The Pig List");

        ui.horizontal(|ui| {
            // Search bar, perform a search if it's been changed
            if ui.add(TextEdit::singleline(&mut state.pages.pigs.query).hint_text("Search")).changed() {
                self.do_query(state);
            }

            // Pig create button, it's only enabled when you have something in
            // the search bar and when you have permissions
            let can_add = state.has_role(Roles::PigEditor) && !state.pages.pigs.query.is_empty();
            ui.add_enabled_ui(can_add, |ui| {
                if ui.button("+ Add").clicked() {
                    // We need to save the name here or else borrow check complains
                    let name = state.pages.pigs.query.to_owned();
                    self.warn_if_dirty(ui.ctx(), state, url, PigPageDirtyAction::Create(name));
                }
            });
        });

        ui.add_space(SPACE_SMALL);

        // Only render the results table if we have results to show
        // TODO add pagination
        if self.query_results.as_ref().is_some_and(|pigs| !pigs.is_empty()) {
            let clicked: Option<Option<Pig>> = selectable_list(ui, self.query_results.as_ref().unwrap(), |row, pig| {
                let selected = state.pages.pigs.selection.as_ref().is_some_and(|select| select.id == pig.id);
                row.set_selected(selected);

                // Make sure we can't select the text or else we can't click the row behind
                row.col(|ui| {
                    Label::new(&pig.name).selectable(false).truncate().ui(ui);
                });

                selected
            });

            // Check if we have an action to do
            if let Some(clicked) = clicked {
                self.warn_if_dirty(ui.ctx(), state, url, PigPageDirtyAction::Select(clicked));
            }
        } else if self.query_results.is_none() {
            // Still waiting on results, this should only happen when waiting
            // since otherwise it'll be an empty vec

            // You spin me right 'round, baby, 'right round
            // Like a record, baby, right 'round, 'round, 'round
            ui.vertical_centered(|ui| ui.spinner());
        }
    }

    /// Adds the pig details/editor to the center panel if a pig is selected
    fn populate_center(&mut self, ui: &mut Ui, state: &mut ClientState) {
        ui.set_max_width(PANEL_WIDTH_MEDIUM);
        state.colorix.draw_background(ui.ctx(), false);
        let can_edit = state.has_role(Roles::PigEditor);

        // THIS IS REALLY FUCKING IMPORTANT, LETS US MODIFY THE VALUE INSIDE THE OPTION
        if let Some(pig) = state.pages.pigs.selection.as_mut() {
            spaced_heading(ui, pig.name.to_owned()); // convert to owned since we transfer a mut reference later

            // Pig action buttons
            if can_edit {
                Flex::horizontal().w_full().justify(FlexJustify::SpaceBetween).show(ui, |flex| {
                    let save_button = Button::new("💾 Save");
                    let delete_button = Button::new("🗑 Delete");

                    // TODO set as disabled again when not dirty. we just have to live with this until https://github.com/lucasmerlin/hello_egui/pull/50 is done
                    if flex.add(item().grow(1.0), save_button).clicked() {
                        self.pig_api.update.request(pig);
                    }

                    if flex.add(item().grow(1.0), delete_button).clicked() {
                        self.delete_modal = true;
                    }
                });

                ui.add_space(SPACE_SMALL);
            }

            // Pig properties table
            properties_list(ui).body(|mut body| {
                add_properties_row(&mut body, TABLE_ROW_HEIGHT_LARGE, "id", |ui| {
                    ui.code(pig.id.to_string());
                });

                add_properties_row(&mut body, TABLE_ROW_HEIGHT_LARGE * 2.0, "name", |ui| {
                    // yes, all this is necessary
                    // centered_and_justified makes the text box fill the value cell
                    // ScrollArea lets you scroll when it's too big
                    // and we must define the layouter as a separate var or else borrow checker gets PISSED
                    ui.centered_and_justified(|ui| {
                        ScrollArea::vertical().show(ui, |ui| {
                            let mut layouter = wrapped_singleline_layouter();
                            let te = TextEdit::singleline(&mut pig.name).desired_rows(4).layouter(&mut layouter);
                            if ui.add_enabled(can_edit, te).changed() {
                                state.pages.pigs.dirty = true;
                            }
                        });
                    });
                });

                add_properties_row(&mut body, TABLE_ROW_HEIGHT_LARGE, "created by", |ui| {
                    // TODO actually bother fetching the user data
                    ui.code(pig.creator.to_string());
                });

                add_properties_row(&mut body, TABLE_ROW_HEIGHT_LARGE, "created at", |ui| {
                    let create_time = pig.created.and_utc().with_timezone(&Local);
                    ui.label(create_time.format(TIME_FMT).to_string());
                });
            });
        }
    }

    /// Show any page-specific modals which should be visible
    fn show_modals(&mut self, ctx: &Context, state: &mut ClientState, url: &ParsedURL) {
        if self.delete_modal {
            let modal = Modal::new("delete")
                .with_heading("Confirm Deletion")
                .with_body("Are you sure you want to delete this pig? There's no going back after this!")
                .show_with_extras(ctx, |ui| {
                    if ui.button("✔ Yes").clicked() {
                        match state.pages.pigs.selection.as_ref() {
                            Some(pig) => self.pig_api.delete.request(pig.id),
                            None => state.pages.layout.display_error = Some(ApiError::new("You tried to delete a pig without having one selected, how the fuck did you manage that?".to_owned())),
                        }
                        self.delete_modal = false;
                    }
                });

            if modal.should_close() {
                self.delete_modal = false;
            }
        }

        if !matches!(self.dirty_modal, PigPageDirtyAction::None) {
            if let Some(do_action) = Modal::dirty(ctx) {
                if do_action {
                    self.do_dirty_action(ctx, state, url);
                } else {
                    self.dirty_modal = PigPageDirtyAction::None;
                }
            }
        }

        if self.pig_not_found_modal {
            if Modal::not_found(ctx) {
                // Close the modal
                self.pig_not_found_modal = false;

                // Update the route
                update_url_hash(ctx, url, None);
            }
        }
    }

    /// Sends a fetch request for all results of the current query and clears
    /// the list of current results
    fn do_query(&mut self, state: &mut ClientState) {
        self.query_results = None;
        self.pig_api.fetch.request(PigQuery::default().with_name(&state.pages.pigs.query));
    }

    /// If the dirty var is true, warn the user with a modal before performing
    /// the given action; otherwise, just do it
    fn warn_if_dirty(&mut self, ctx: &Context, state: &mut ClientState, url: &ParsedURL, action: PigPageDirtyAction) {
        self.dirty_modal = action;

        // If the state isn't dirty, execute the action right away
        // else if dirty_modal is not None, it will be shown
        if !state.pages.pigs.dirty {
            self.do_dirty_action(ctx, state, url);
        }
    }

    /// Performs the dirty action, resets all relevant variables, and refreshes
    /// all relevant data
    fn do_dirty_action(&mut self, ctx: &Context, state: &mut ClientState, url: &ParsedURL) {
        match &self.dirty_modal {
            PigPageDirtyAction::Create(name) => self.pig_api.create.request(name),
            PigPageDirtyAction::Select(selection) => {
                // Change the selection
                state.pages.pigs.selection = selection.as_ref().and_then(|pig| Some(pig.to_owned()));
                update_url_hash(ctx, url, state.pages.pigs.selection.as_ref().and_then(|pig| Some(pig.id)))
            }
            PigPageDirtyAction::None => {}
        }
        // Reset dirty state, how tf did i forget this?
        self.dirty_modal = PigPageDirtyAction::None;
        state.pages.pigs.dirty = false;
    }
}
