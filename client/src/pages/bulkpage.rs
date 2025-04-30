use crate::data::api::BulkApi;
use crate::data::state::ClientState;
use crate::modal::Modal;
use crate::pages::RenderPage;
use crate::selectable_list::SelectableList;
use crate::style::TIME_FMT;
use crate::DirtyAction;
use chrono::Local;
use egui::{Button, CentralPanel, Context, Label, SidePanel, Ui, Widget};
use pigweb_common::bulk::{BulkImport, BulkQuery};
use pigweb_common::users::Roles;
use urlable::ParsedURL;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct BulkPage {
    pub selected_import: Option<BulkImport>,

    /// Whether we have unsaved changes
    dirty: bool,
}

impl Default for BulkPage {
    fn default() -> Self {
        Self { selected_import: None, dirty: false }
    }
}

pub struct BulkPageRender {
    bulk_api: BulkApi,
    all_imports: Option<Vec<BulkImport>>,
    dirty_modal: DirtyAction<Vec<String>, BulkImport>,
    raw_names: String,
}

impl Default for BulkPageRender {
    fn default() -> Self {
        Self {
            bulk_api: BulkApi::default(),
            all_imports: None,
            dirty_modal: DirtyAction::None,
            raw_names: String::default(),
        }
    }
}

impl RenderPage for BulkPageRender {
    fn open(&mut self, _ctx: &Context, _state: &mut ClientState, _url: &ParsedURL) {
        self.do_query();
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

        CentralPanel::default().show(ui.ctx(), |ui| {
            ui.vertical_centered(|ui| {
                self.populate_center(ui, state, url);
            });
        });

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
        }

        if self.bulk_api.patch.received(state).is_some() {
            state.pages.bulk.dirty = false;
            self.do_query();
        }

        if let Some(imports) = self.bulk_api.fetch.received(state) {
            self.all_imports = Some(imports);
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
                SelectableList::new().show(ui, self.all_imports.as_ref().unwrap(), |row, import| {
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
                self.warn_if_dirty(ui.ctx(), state, url, DirtyAction::Select(clicked));
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
        state.colorix.draw_background(ui.ctx(), false);

        if let Some(import) = state.pages.bulk.selected_import.as_ref() {
            if import.finished.is_some() {
                self.populate_center_finished(ui, state);
            } else {
                self.populate_center_edit(ui, state);
            }
        } else {
            self.populate_center_create(ui, state, url);
        }
    }

    fn populate_center_create(&mut self, ui: &mut Ui, state: &mut ClientState, url: &ParsedURL) {
        ui.set_max_width(540.0);

        // Title
        ui.add_space(8.0);
        ui.heading("Start Import"); // convert to owned since we transfer a mut reference later
        ui.add_space(8.0);

        let add_button = Button::new("+ Add All Pigs");
        if ui.add_enabled(!self.raw_names.is_empty(), add_button).clicked() {
            let names = self.raw_names.lines().map(|l: &str| l.to_string()).collect::<Vec<String>>();
            self.warn_if_dirty(ui.ctx(), state, url, DirtyAction::Create(names));
        }

        ui.text_edit_multiline(&mut self.raw_names);
    }

    fn populate_center_edit(&mut self, ui: &mut Ui, state: &mut ClientState) {
        // TODO
    }

    fn populate_center_finished(&mut self, ui: &mut Ui, state: &mut ClientState) {
        // TODO
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

    /// If the dirty var is true, warn the user with a modal before performing
    /// the given action; otherwise, just do it
    fn warn_if_dirty(
        &mut self,
        ctx: &Context,
        state: &mut ClientState,
        url: &ParsedURL,
        action: DirtyAction<Vec<String>, BulkImport>,
    ) {
        self.dirty_modal = action;

        // If the state isn't dirty, execute the action right away
        // else if dirty_modal is not None, it will be shown
        if !state.pages.bulk.dirty {
            self.do_dirty_action(ctx, state, url);
        }
    }

    fn do_dirty_action(&mut self, ctx: &Context, state: &mut ClientState, url: &ParsedURL) {
        match &self.dirty_modal {
            DirtyAction::Create(names) => self.bulk_api.create.request(names),
            DirtyAction::Select(selection) => {
                // Change the selection
                state.pages.bulk.selected_import = selection.as_ref().and_then(|import| Some(import.to_owned()));
            }
            DirtyAction::None => {}
        }

        // Reset dirty state, how tf did i forget this?
        self.dirty_modal = DirtyAction::None;
        state.pages.bulk.dirty = false;
    }
}
