use crate::data::api::{ApiError, PigApi, PigFetchHandler};
use crate::data::state::ClientState;
use crate::modal::Modal;
use crate::pages::RenderPage;
use chrono::Local;
use eframe::emath::Align;
use eframe::epaint::text::LayoutJob;
use egui::{
    Button, CentralPanel, Context, FontSelection, Label, Layout, OpenUrl, ScrollArea, Sense, SidePanel, TextEdit, Ui,
    Widget,
};
use egui_extras::{Column, TableBody};
use egui_flex::{item, Flex, FlexJustify};
use log::{debug, error};
use pigweb_common::pigs::{Pig, PigQuery};
use pigweb_common::users::Roles;
use std::cmp::PartialEq;
use std::mem;
use urlable::ParsedURL;
use uuid::Uuid;

// ( ͡° ͜ʖ ͡°)
#[derive(Debug)]
pub enum DirtyAction {
    Create(String),
    Select(Option<Pig>),
    None,
}

impl PartialEq for DirtyAction {
    fn eq(&self, other: &Self) -> bool {
        mem::discriminant(self) == mem::discriminant(other)
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct PigPage {
    /*
     * shit we care about saving
     */
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

pub struct PigPageRender {
    /*
     * shit we don't care about saving as it's actively in use
     */
    /// The last hash which was requested
    last_hash: String,

    /// Handles sending and receiving API data
    pig_api: PigApi,

    /// Handles API data specifically when getting the selection from the URL
    pig_fetch_from_url: PigFetchHandler,

    /// The current list of search results
    query_results: Option<Vec<Pig>>,

    /// Modal which warns you when there's unsaved changes
    dirty_modal: DirtyAction,

    /// Whether to show the modal to confirm deleting a pig
    delete_modal: bool,

    /// Whether to show the modal for a URL where no pig exists
    pig_not_found_modal: bool,
}

impl Default for PigPageRender {
    fn default() -> Self {
        Self {
            last_hash: String::new(),
            pig_api: PigApi::default(),
            pig_fetch_from_url: PigFetchHandler::default(),
            query_results: None,
            dirty_modal: DirtyAction::None,
            delete_modal: false,
            pig_not_found_modal: false,
        }
    }
}

impl RenderPage for PigPageRender {
    fn open(&mut self, ctx: &Context, state: &mut ClientState, url: &ParsedURL) {
        self.update_selection(ctx, state, url);
        self.do_query(state)
    }

    fn ui(&mut self, ui: &mut Ui, state: &mut ClientState, url: &ParsedURL) {
        if !state.has_role(Roles::PigViewer) {
            // TODO 403 Forbidden
            return;
        }

        // don't redo selection if hash hasn't changed
        if url.hash != self.last_hash {
            self.update_selection(ui.ctx(), state, url);
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
    fn update_selection(&mut self, ctx: &Context, state: &mut ClientState, url: &ParsedURL) {
        // remember that this was the last requested page
        self.last_hash = url.hash.to_string();

        // url.hash and self.last_hash must have the # character in it for previous checks to work
        // for the logic below, it depends on that character being gone
        let stripped_hash = self.last_hash.replacen('#', "", 1);
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
                        self.pig_fetch_from_url.request(PigQuery::default().with_id(&uuid).with_limit(1));
                    }
                }
                Err(err) => {
                    state.pages.layout.display_error =
                        Some(ApiError::new(err.to_string()).with_reason("Unable to parse UUID.".to_owned()));
                    Self::update_url_hash(ctx, url, None);
                    error!("Unable to parse hash \"{:?}\", err: {:?}", &stripped_hash, err);
                }
            }
        } else if state.pages.pigs.selection.is_some() {
            // if we have a pig selected, deselect it
            debug!("Hash is empty but selection is {:?}, selecting None!", state.pages.pigs.selection.as_ref());
            self.warn_if_dirty(ctx, state, url, DirtyAction::Select(None));
        }
    }

    fn process_promises(&mut self, ctx: &Context, state: &mut ClientState, url: &ParsedURL) {
        if let Some(pig) = self.pig_api.create.received(state) {
            state.pages.pigs.dirty = false;
            state.pages.pigs.selection = Some(pig);
            Self::update_url_hash(ctx, url, Some(state.pages.pigs.selection.as_ref().unwrap().id));
            self.do_query(state); // Redo the search query so it includes the new pig
        }

        if self.pig_api.update.received(state).is_some() {
            state.pages.pigs.dirty = false;
            self.do_query(state); // Redo the search query so it includes any possible changes
        }

        if self.pig_api.delete.received(state).is_some() {
            state.pages.pigs.dirty = false;
            state.pages.pigs.selection = None;
            Self::update_url_hash(ctx, url, None);
            self.do_query(state); // Redo the search query to exclude the deleted pig
        }

        if let Some(pigs) = self.pig_api.fetch.received(state) {
            self.query_results = Some(pigs);
        }

        if let Some(mut pigs) = self.pig_fetch_from_url.received(state) {
            // This request should have been made with limit = 1
            // therefore, the only pig is the one we want
            if let Some(pig) = pigs.pop() {
                self.warn_if_dirty(ctx, state, url, DirtyAction::Select(Some(pig)));
            } else {
                self.pig_not_found_modal = true;
            }
        }
    }

    fn populate_sidebar(&mut self, ui: &mut Ui, state: &mut ClientState, url: &ParsedURL) {
        ui.set_width(320.0);
        ui.add_space(8.0);
        ui.heading("The Pig List");
        ui.add_space(8.0);

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
                    self.warn_if_dirty(ui.ctx(), state, url, DirtyAction::Create(name));
                }
            });
        });

        ui.add_space(4.0);

        // Only render the results table if we have results to show
        // TODO add pagination
        if self.query_results.as_ref().is_some_and(|pigs| !pigs.is_empty()) {
            let mut clicked: Option<Pig> = None;

            egui_extras::TableBuilder::new(ui)
                .striped(true)
                .resizable(false)
                .column(Column::remainder())
                .sense(Sense::click())
                .cell_layout(Layout::left_to_right(Align::Center))
                .body(|mut body| {
                    let pigs = self.query_results.as_ref().unwrap();
                    // This means we don't have to clone the list every frame
                    pigs.iter().for_each(|pig| {
                        body.row(18.0, |mut row| {
                            // idfk why this wants us to clone selection, otherwise page is supposedly moved
                            row.set_selected(
                                state.pages.pigs.selection.as_ref().is_some_and(|select| select.id == pig.id),
                            );

                            // Make sure we can't select the text or else we can't click the row behind
                            row.col(|ui| {
                                Label::new(&pig.name).selectable(false).truncate().ui(ui);
                            });

                            // On click, check if we have to change the selection before processing it
                            if row.response().clicked()
                                && !state.pages.pigs.selection.as_ref().is_some_and(|sel| sel.id == pig.id)
                            {
                                // warn about unsaved changes, else JUST DO IT
                                // ...and we clone the clone because of fucking course we do D:<
                                clicked = Some(pig.clone());
                            }
                        });
                    });
                });

            // Check if we have an action to do
            if clicked.is_some() {
                self.warn_if_dirty(ui.ctx(), state, url, DirtyAction::Select(Some(clicked.unwrap())));
            }
        } else if self.query_results.is_none() {
            // Still waiting on results, this should only happen when waiting
            // since otherwise it'll be an empty vec

            // You spin me right 'round, baby, 'right round
            // Like a record, baby, right 'round, 'round, 'round
            ui.vertical_centered(|ui| ui.spinner());
        }
    }

    fn populate_center(&mut self, ui: &mut Ui, state: &mut ClientState) {
        ui.set_max_width(540.0);
        state.colorix.draw_background(ui.ctx(), false);
        let can_edit = state.has_role(Roles::PigEditor);

        // THIS IS REALLY FUCKING IMPORTANT, LETS US MODIFY THE VALUE INSIDE THE OPTION
        if let Some(pig) = state.pages.pigs.selection.as_mut() {
            // Title
            ui.add_space(8.0);
            ui.heading(pig.name.to_owned()); // convert to owned since we transfer a mut reference later
            ui.add_space(8.0);

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

                ui.add_space(4.0);
            }

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
                                // Adapted from https://github.com/emilk/egui/blob/0db56dc9f1a8459b5b9376159fab7d7048b19b65/crates/egui/src/widgets/text_edit/builder.rs#L521-L529
                                // We need to write a custom layouter for this so we can visually
                                // wrap the text while still treating it as a single line
                                let mut wrapped_singleline_layouter = |ui: &Ui, text: &str, wrap_width: f32| {
                                    let job = LayoutJob::simple(
                                        text.to_owned(),
                                        FontSelection::default().resolve(ui.style()),
                                        ui.visuals()
                                            .override_text_color
                                            .unwrap_or_else(|| ui.visuals().widgets.inactive.text_color()),
                                        wrap_width,
                                    );
                                    ui.fonts(|f| f.layout_job(job))
                                };

                                let te = TextEdit::singleline(&mut pig.name)
                                    .desired_rows(4)
                                    .layouter(&mut wrapped_singleline_layouter);

                                if ui.add_enabled(can_edit, te).changed() {
                                    state.pages.pigs.dirty = true;
                                }
                            });
                        });
                    });

                    add_pig_properties_row(&mut body, 40.0, "created by", |ui| {
                        // TODO actually bother fetching the user data
                        ui.code(pig.creator.to_string());
                    });

                    add_pig_properties_row(&mut body, 40.0, "created on", |ui| {
                        let create_time = pig.created.and_utc().with_timezone(&Local);
                        ui.label(create_time.format("%a, %b %e %Y %T").to_string());
                    });
                });
        }
    }

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

        if self.pig_not_found_modal {
            let modal = Modal::new("pig_not_found")
                .with_heading("Pig Not Found")
                .with_body("We couldn't find a pig with that id.")
                .show(ctx);

            if modal.should_close() {
                // Close the modal
                self.pig_not_found_modal = false;

                // Update the route
                Self::update_url_hash(ctx, url, None);
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
    fn warn_if_dirty(&mut self, ctx: &Context, state: &mut ClientState, url: &ParsedURL, action: DirtyAction) {
        self.dirty_modal = action;

        // If the state isn't dirty, execute the action right away
        // else if dirty_modal is not None, it will be shown
        if !state.pages.pigs.dirty {
            self.do_dirty_action(ctx, state, url);
        }
    }

    fn do_dirty_action(&mut self, ctx: &Context, state: &mut ClientState, url: &ParsedURL) {
        match &self.dirty_modal {
            DirtyAction::Create(name) => self.pig_api.create.request(name),
            DirtyAction::Select(selection) => {
                // Change the selection
                state.pages.pigs.selection = selection.as_ref().and_then(|pig| Some(pig.to_owned()));
                Self::update_url_hash(ctx, url, state.pages.pigs.selection.as_ref().and_then(|pig| Some(pig.id)))
            }
            DirtyAction::None => {}
        }
        // Reset dirty state, how tf did i forget this?
        self.dirty_modal = DirtyAction::None;
        state.pages.pigs.dirty = false;
    }

    /// Updates the hash on the URL to the given UUID if it is Some, else
    /// removes the hash from the URL. Then, asks egui to navigate to the new
    /// URL.
    fn update_url_hash(ctx: &Context, url: &ParsedURL, uuid: Option<Uuid>) {
        let mut dest = url.clone();
        dest.hash = uuid.map(|id| "#".to_owned() + id.to_string().as_str()).unwrap_or("".to_owned());
        ctx.open_url(OpenUrl::same_tab(dest.stringify()));
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
