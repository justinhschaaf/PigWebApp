use crate::data::api::UserApi;
use crate::data::state::ClientState;
use crate::pages::RenderPage;
use crate::style::TIME_FMT;
use chrono::{Local, Utc};
use eframe::emath::Align;
use egui::{Button, CentralPanel, Context, Layout, Sense, Ui};
use egui_extras::{Column, TableBody, TableBuilder};
use pigweb_common::users::{Roles, User, UserQuery};
use urlable::ParsedURL;

pub struct UserPageRender {
    user_api: UserApi,
    users: Option<Vec<User>>,
}

impl Default for UserPageRender {
    fn default() -> Self {
        Self { user_api: UserApi::default(), users: None }
    }
}

impl RenderPage for UserPageRender {
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
                        .body(|mut body| self.add_user_rows(&mut body, state));
                } else if self.users.is_none() {
                    ui.spinner();
                }
            });
        });
    }
}

impl UserPageRender {
    fn process_promises(&mut self, _ctx: &Context, state: &mut ClientState, _url: &ParsedURL) {
        if let Some(res) = self.user_api.fetch.received(state) {
            self.users = res.users;
        }

        // TODO we could just swap out the individual user in the list
        if let Some(_) = self.user_api.expire.received(state) {
            self.fetch_users();
        }
    }

    fn add_user_rows(&mut self, body: &mut TableBody, state: &mut ClientState) {
        for user in self.users.as_ref().unwrap() {
            body.row(20.0, |mut row| {
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
            });
        }
    }

    fn fetch_users(&mut self) {
        self.user_api.fetch.request(UserQuery::default())
    }
}
