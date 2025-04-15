use crate::auth::AuthenticatedUser;
use crate::config::Config;
use chrono::Utc;
use diesel::{ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl, SelectableHelper};
use pigweb_common::users::{Roles, User, UserFetchResponse, UserQuery};
use pigweb_common::{parse_uuid, schema};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{Route, State};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::DerefMut;
use std::sync::Mutex;
use uuid::Uuid;

pub fn get_user_api_routes() -> Vec<Route> {
    routes![api_user_fetch, api_user_roles, api_user_expire]
}

#[get("/fetch?<query..>")]
async fn api_user_fetch(
    auth_user: AuthenticatedUser,
    config: &State<Config>,
    db_connection: &State<Mutex<PgConnection>>,
    query: UserQuery,
) -> Result<Json<UserFetchResponse>, Status> {
    // Fetch the users from the DB
    let sql_query = query.to_db_select();
    let mut db_connection = db_connection.lock().unwrap();
    let sql_res = sql_query.select(User::as_select()).load(db_connection.deref_mut());

    if let Ok(users) = sql_res {
        let mut ids_to_names: BTreeMap<Uuid, String> = BTreeMap::new();

        // Get the mapping of uuids to usernames
        for user in &users {
            ids_to_names.insert(user.id.to_owned(), user.username.to_owned());
        }

        let mut res = UserFetchResponse::default().with_usernames(ids_to_names);

        // add the actual users if requester has access
        if auth_user.has_role(config, Roles::UserViewer) {
            res = res.with_users(users);
        }

        Ok(Json(res))
    } else {
        error!("Unable to load SQL result for query {:?}: {:?}", query, sql_res.unwrap_err());
        Err(Status::InternalServerError)
    }
}

#[get("/roles?<query..>")]
async fn api_user_roles(
    auth_user: AuthenticatedUser,
    config: &State<Config>,
    db_connection: &State<Mutex<PgConnection>>,
    query: UserQuery,
) -> Result<Json<BTreeMap<Uuid, BTreeSet<Roles>>>, Status> {
    if !auth_user.has_role(config, Roles::UserViewer) {
        return Err(Status::Forbidden);
    }

    // Fetch the users from the DB
    let sql_query = query.to_db_select();
    let mut db_connection = db_connection.lock().unwrap();
    let sql_res = sql_query.select(User::as_select()).load(db_connection.deref_mut());

    if let Ok(users) = sql_res {
        let mut res: BTreeMap<Uuid, BTreeSet<Roles>> = BTreeMap::new();

        // Get the mapping of uuids to usernames
        for user in &users {
            let roles = get_user_roles(config, user);
            res.insert(user.id.to_owned(), roles);
        }

        Ok(Json(res))
    } else {
        error!("Unable to load SQL result for query {:?}: {:?}", query, sql_res.unwrap_err());
        Err(Status::InternalServerError)
    }
}

#[patch("/expire?<id>")]
async fn api_user_expire(
    auth_user: AuthenticatedUser,
    config: &State<Config>,
    db_connection: &State<Mutex<PgConnection>>,
    id: &str,
) -> Result<Json<User>, Status> {
    if !auth_user.has_role(config, Roles::UserAdmin) {
        return Err(Status::Forbidden);
    }

    let mut db_connection = db_connection.lock().unwrap();

    let uuid = parse_uuid(id)?;
    let now = Utc::now().naive_utc();

    let sql_res = diesel::update(schema::users::table)
        .filter(schema::users::columns::id.eq(uuid))
        .set(schema::users::columns::session_exp.eq(now))
        .get_result(db_connection.deref_mut());

    if sql_res.is_ok() {
        Ok(Json(sql_res.unwrap()))
    } else {
        error!("Unable to invalidate session for user {:?}: {:?}", uuid, sql_res.unwrap_err());
        Err(Status::InternalServerError)
    }
}

pub fn user_has_role(config: &Config, user: &User, role: Roles) -> bool {
    if config.oidc.is_none() || get_user_roles(config, user).contains(&role) {
        return true;
    }

    false
}

pub fn get_user_roles(config: &Config, user: &User) -> BTreeSet<Roles> {
    // If groups aren't configured, all users have all access
    if config.oidc.is_none() || config.groups.is_empty() {
        return Roles::values().collect::<BTreeSet<Roles>>();
    }

    let mut res = BTreeSet::new();

    // for each group the user has
    for group in &user.groups {
        // try to find the roles in that group
        if let Some(roles) = config.groups.get(group) {
            // add the group's roles to the response
            res.append(&mut roles.clone())
        }
    }

    res
}
