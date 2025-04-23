use crate::data::api::{ApiError, UserApi, UserFetchHandler};
use crate::data::state::ClientState;
use crate::pages::RenderPage;
use crate::style::TIME_FMT;
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

pub struct UserPageRender {
    user_api: UserApi,
    user_fetch_selection: UserFetchHandler,
    users: Option<Vec<User>>,
    selection: Option<User>,
    roles: Option<BTreeSet<Roles>>,
}

impl Default for UserPageRender {
    fn default() -> Self {
        Self {
            user_api: UserApi::default(),
            user_fetch_selection: UserFetchHandler::default(),
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
                        self.user_fetch_selection.request(UserQuery::default().with_id(&uuid).with_limit(1));
                    }
                }
                Err(err) => {
                    state.pages.layout.display_error =
                        Some(ApiError::new(err.to_string()).with_reason("Unable to parse UUID.".to_owned()));
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

        CentralPanel::default().show(ui.ctx(), |ui| {
            state.colorix.draw_background(ui.ctx(), false);
            ui.vertical_centered(|ui| {
                ui.set_max_width(960.0);
                ui.add_space(8.0);

                if self.users.as_ref().is_some_and(|users| !users.is_empty()) {
                    TableBuilder::new(ui)
                        .striped(true)
                        .sense(Sense::click())
                        .cell_layout(Layout::left_to_right(Align::Center))
                        .column(Column::initial(280.0))
                        .column(Column::initial(280.0))
                        .column(Column::initial(200.0))
                        .column(Column::initial(200.0))
                        .header(20.0, |mut header| {
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
                    ui.spinner();
                }
            });
        });
    }
}

impl UserPageRender {
    fn process_promises(&mut self, ctx: &Context, state: &mut ClientState, url: &ParsedURL) {
        if let Some(res) = self.user_api.fetch.received(state) {
            self.users = res.users;
        }

        if let Some(roles) = self.user_api.roles.received(state) {
            if let Some(sel) = self.selection.as_ref() {
                self.roles = roles.get(&sel.id).cloned();
            }
        }

        // TODO we could just swap out the individual user in the list
        if let Some(_) = self.user_api.expire.received(state) {
            self.fetch_users();
        }

        if let Some(mut users) = self.user_fetch_selection.received(state).and_then(|res| res.users) {
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

    fn add_user_rows(&mut self, body: &mut TableBody, state: &mut ClientState, url: &ParsedURL) {
        for user in self.users.as_ref().unwrap() {
            let selected = self.selection.as_ref().is_some_and(|sel| sel.id == user.id);

            body.row(20.0, |mut row| {
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

                if row.response().clicked() {
                    let ctx = &row.response().ctx;
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

    fn fetch_users(&mut self) {
        self.user_api.fetch.request(UserQuery::default())
    }
}
