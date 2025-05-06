use crate::data::api::{ApiError, UserApi, UserFetchHandler};
use crate::data::state::ClientState;
use crate::pages::RenderPage;
use crate::ui::style::{
    PANEL_WIDTH_LARGE, SPACE_MEDIUM, TABLE_COLUMN_WIDTH_MEDIUM, TABLE_COLUMN_WIDTH_SMALL, TABLE_ROW_HEIGHT_LARGE,
    TABLE_ROW_HEIGHT_SMALL, TIME_FMT,
};
use crate::update_url_hash;
use chrono::{Local, Utc};
use eframe::emath::Align;
use egui::{Button, CentralPanel, Context, Layout, Sense, Ui};
use egui_extras::{Column, TableBody, TableBuilder};
use log::{debug, error};
use pigweb_common::users::{Roles, User, UserQuery};
use std::collections::BTreeSet;
use urlable::ParsedURL;
use uuid::Uuid;

/// Responsible for rendering [`crate::pages::Routes::Users`]
///
/// Admittedly, this page is somewhat rushed and is meant to get the minimum
/// function in (expiring user sessions). Maybe one day I'll come back and
/// actually add in UI for showing each user's details and rows. For now, it's
/// good enough.
pub struct UserPageRender {
    /// Handles sending and receiving API data
    user_api: UserApi,

    /// Handles API data specifically when getting the selection from the URL
    fetch_url_selection: UserFetchHandler,

    /// The full list of users registered in the app
    users: Option<Vec<User>>,

    /// The currently selected user
    selection: Option<User>,

    /// The roles the currently selected user has access to
    roles: Option<BTreeSet<Roles>>,
}

impl Default for UserPageRender {
    fn default() -> Self {
        Self {
            user_api: UserApi::default(),
            fetch_url_selection: UserFetchHandler::default(),
            users: None,
            selection: None,
            roles: None,
        }
    }
}

impl RenderPage for UserPageRender {
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
                    if self.selection.as_ref().is_none_or(|selected| uuid != selected.id) {
                        debug!(
                            "The selection has been updated via url! Previous Selection: {:?}",
                            self.selection.as_ref()
                        );
                        self.fetch_url_selection.request(UserQuery::default().with_id(&uuid).with_limit(1));
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
        } else if self.selection.is_some() {
            // if we have a pig selected, deselect it
            debug!("Hash is empty but selection is {:?}, selecting None!", self.selection.as_ref());
            self.selection = None;
            self.roles = None;
            self.user_api.roles.discard();
        }
    }

    fn open(&mut self, _ctx: &Context, _state: &mut ClientState, _url: &ParsedURL) {
        self.fetch_users();
    }

    fn ui(&mut self, ui: &mut Ui, state: &mut ClientState, url: &ParsedURL) {
        if !state.has_role(Roles::UserViewer) {
            // TODO 403 Forbidden
            return;
        }

        self.process_promises(ui.ctx(), state, url);

        // Draw the CentralPanel and the user table here because that's all this page is
        // Use the helper function to populate the table body
        CentralPanel::default().show(ui.ctx(), |ui| {
            state.colorix.draw_background(ui.ctx(), false);
            ui.vertical_centered(|ui| {
                ui.set_max_width(PANEL_WIDTH_LARGE);
                ui.add_space(SPACE_MEDIUM);

                // Only add the table if we have users loaded
                if self.users.as_ref().is_some_and(|users| !users.is_empty()) {
                    TableBuilder::new(ui)
                        .striped(true)
                        .sense(Sense::click())
                        .cell_layout(Layout::left_to_right(Align::Center))
                        .column(Column::initial(TABLE_COLUMN_WIDTH_MEDIUM))
                        .column(Column::initial(TABLE_COLUMN_WIDTH_MEDIUM))
                        .column(Column::initial(TABLE_COLUMN_WIDTH_SMALL))
                        .column(Column::initial(TABLE_COLUMN_WIDTH_SMALL))
                        .header(TABLE_ROW_HEIGHT_LARGE, |mut header| {
                            header.col(|ui| {
                                ui.heading("id");
                            });
                            header.col(|ui| {
                                ui.heading("username");
                            });
                            header.col(|ui| {
                                ui.heading("last seen");
                            });
                            header.col(|ui| {
                                ui.heading("session");
                            });
                        })
                        .body(|mut body| self.add_user_rows(&mut body, state, url));
                } else if self.users.is_none() {
                    // you spin me...
                    ui.spinner();
                }
            });
        });
    }
}

impl UserPageRender {
    /// Checks all APIs for data received from previously submitted requests
    fn process_promises(&mut self, ctx: &Context, state: &mut ClientState, url: &ParsedURL) {
        if let Some(res) = self.user_api.fetch.received(state) {
            self.users = res.users;
        }

        if let Some(roles) = self.user_api.roles.received(state) {
            if let Some(sel) = self.selection.as_ref() {
                self.roles = roles.get(&sel.id).cloned();
            }
        }

        if let Some(user) = self.user_api.expire.received(state) {
            // update the user in the list of all users rather than refreshing everything
            if let Some(users) = self.users.as_mut() {
                let pos = users.iter().position(|e| e.id.eq(&user.id));
                pos.and_then(|i| Some(users[i] = user.clone()));
            }

            // if we have this user selected, update the data
            if self.selection.as_ref().is_some_and(|sel| sel.id.eq(&user.id)) {
                self.selection = Some(user);
            }
        }

        if let Some(mut users) = self.fetch_url_selection.received(state).and_then(|res| res.users) {
            // This request should have been made with limit = 1
            // therefore, the only user is the one we want
            if let Some(user) = users.pop() {
                self.user_api.roles.request(UserQuery::default().with_id(&user.id));
                self.selection = Some(user);
            } else {
                // else there isn't a user and i'm not implementing a message for it rn
                update_url_hash(ctx, url, None)
            }
        }
    }

    /// Populates the given table body with the loaded users. We have to do it
    /// all in one shot rather than having a function per user or else borrow
    /// checker complains
    fn add_user_rows(&mut self, body: &mut TableBody, state: &mut ClientState, url: &ParsedURL) {
        for user in self.users.as_ref().unwrap() {
            let selected = self.selection.as_ref().is_some_and(|sel| sel.id == user.id);

            body.row(TABLE_ROW_HEIGHT_SMALL, |mut row| {
                row.set_selected(selected);

                row.col(|ui| {
                    ui.code(user.id.to_string());
                });

                row.col(|ui| {
                    ui.label(user.username.as_str());
                });

                row.col(|ui| {
                    let time = user.seen.and_utc().with_timezone(&Local);
                    ui.label(time.format(TIME_FMT).to_string());
                });

                row.col(|ui| {
                    if ui
                        .add_enabled(
                            user.session_exp
                                .is_some_and(|time| state.has_role(Roles::UserAdmin) && time >= Utc::now().naive_utc()),
                            Button::new("⌛ Expire"),
                        )
                        .clicked()
                    {
                        self.user_api.expire.request(user.id);
                    }
                });

                // Update the selection if the row is clicked. Logic can go here directly since there's
                // no other way to select a user and since there's no dirty state to worry about
                if row.response().clicked() {
                    let ctx = &row.response().ctx;
                    // If the row is selected, unset the selection, else update the selection to this row
                    if selected {
                        self.selection = None;
                        self.roles = None;
                        self.user_api.roles.discard();
                        update_url_hash(ctx, url, None);
                    } else {
                        self.user_api.roles.request(UserQuery::default().with_id(&user.id));
                        self.selection = Some(user.clone());
                        update_url_hash(ctx, url, self.selection.as_ref().map(|user| user.id));
                    }
                }
            });
        }
    }

    /// Sends a fetch request for all [`User`]s in the system
    fn fetch_users(&mut self) {
        self.user_api.fetch.request(UserQuery::default())
    }
}
