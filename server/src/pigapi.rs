use crate::auth::AuthenticatedUser;
use crate::config::Config;
use diesel::{ExpressionMethods, PgConnection, PgTextExpressionMethods, QueryDsl, RunQueryDsl, SelectableHelper};
use diesel_full_text_search::{plainto_tsquery, to_tsvector, TsVectorExtensions};
use pigweb_common::pigs::{Pig, PigQuery};
use pigweb_common::users::Roles;
use pigweb_common::{parse_uuid, parse_uuids, schema, DEFAULT_API_RESPONSE_LIMIT};
use rocket::http::Status;
use rocket::response::status::Created;
use rocket::serde::json::Json;
use rocket::{Route, State};
use std::ops::DerefMut;
use std::sync::Mutex;
use uuid::Uuid;

pub fn get_pig_api_routes() -> Vec<Route> {
    routes![api_pig_create, api_pig_update, api_pig_delete, api_pig_fetch]
}

#[post("/create?<name>")]
async fn api_pig_create(
    auth_user: AuthenticatedUser,
    config: &State<Config>,
    db_connection: &State<Mutex<PgConnection>>,
    name: &str,
) -> Result<Created<Json<Pig>>, Status> {
    if !auth_user.has_role(config, Roles::PigEditor) {
        return Err(Status::Forbidden);
    }

    // Create the new pig
    // TODO deduplicate uuids and names
    let pig = Pig::new(name, auth_user.user.id.as_ref());

    // Save it to the DB
    let mut db_connection = db_connection.lock().unwrap();
    let sql_res = diesel::insert_into(schema::pigs::table).values(&pig).execute(db_connection.deref_mut());

    if sql_res.is_ok() {
        // Respond with a path to the pig and the object itself, unfortunately the location path is mandatory
        let params = PigQuery { id: Some(Vec::from([pig.id.to_string()])), ..Default::default() };
        Ok(Created::new(params.to_yuri()).body(Json(pig)))
    } else {
        error!("Unable to save new pig {:?}: {:?}", pig, sql_res.unwrap_err());
        Err(Status::InternalServerError)
    }
}

#[put("/update", data = "<pig>")]
async fn api_pig_update(
    auth_user: AuthenticatedUser,
    config: &State<Config>,
    db_connection: &State<Mutex<PgConnection>>,
    pig: Json<Pig>,
) -> Result<Json<Pig>, Status> {
    if !auth_user.has_role(config, Roles::PigEditor) {
        return Err(Status::Forbidden);
    }

    let pig = pig.into_inner();
    let mut db_connection = db_connection.lock().unwrap();

    // Because Pig derives Identifiable and AsChangeset it just kinda knows what needs to be updated
    let sql_res = diesel::update(schema::pigs::table).set(&pig).get_result(db_connection.deref_mut());

    if sql_res.is_ok() {
        // Return the updated pig
        Ok(Json(sql_res.unwrap()))
    } else {
        error!("Unable to update pig {:?}: {:?}", pig, sql_res.unwrap_err());
        Err(Status::InternalServerError)
    }
}

#[delete("/delete?<id>")]
async fn api_pig_delete(
    auth_user: AuthenticatedUser,
    config: &State<Config>,
    db_connection: &State<Mutex<PgConnection>>,
    id: &str,
) -> Result<Status, Status> {
    if !auth_user.has_role(config, Roles::PigEditor) {
        return Err(Status::Forbidden);
    }

    let uuid = parse_uuid(id)?;

    let mut db_connection = db_connection.lock().unwrap();
    let sql_res =
        diesel::delete(schema::pigs::table.filter(schema::pigs::id.eq(uuid))).execute(db_connection.deref_mut());

    if sql_res.is_ok() {
        Ok(Status::NoContent)
    } else {
        error!("Unable to delete pig {:?}: {:?}", id, sql_res.unwrap_err());
        Err(Status::InternalServerError)
    }
}

#[get("/fetch?<query..>")]
async fn api_pig_fetch(
    auth_user: AuthenticatedUser,
    config: &State<Config>,
    db_connection: &State<Mutex<PgConnection>>,
    query: PigQuery,
) -> Result<Json<Vec<Pig>>, Status> {
    if !auth_user.has_role(config, Roles::PigViewer) {
        return Err(Status::Forbidden);
    }

    // Convert IDs to UUIDs, if present
    let ids: Option<Vec<Uuid>> = query.id.as_ref().and_then(|ids| parse_uuids(&ids).ok());
    let mut limit = DEFAULT_API_RESPONSE_LIMIT;

    // Start constructing the SQL query
    let mut sql_query = schema::pigs::table.into_boxed();

    // Filter by name, if specified
    if let Some(ref query_name) = query.name {
        // This performs a full text search
        // https://www.slingacademy.com/article/implementing-fuzzy-search-with-postgresql-full-text-search/?#implementing-fuzzy-matching-with-fts
        sql_query = sql_query
            .filter(to_tsvector(schema::pigs::name).matches(plainto_tsquery(query_name)))
            .or_filter(schema::pigs::name.ilike(format!("%{}%", query_name)));
    }

    // Filter by id, if specified
    if let Some(query_ids) = ids {
        sql_query = sql_query.filter(schema::pigs::id.eq_any(query_ids));
    }

    // Set the limit, if present
    if let Some(l) = query.limit {
        limit = l;
    }

    // Set the offset, if present
    if let Some(offset) = query.offset {
        if offset > 0 {
            sql_query = sql_query.offset(offset as i64);
        }
    }

    // Set the limit and submit the query to the DB
    let mut db_connection = db_connection.lock().unwrap();
    let sql_res = sql_query.limit(limit as i64).select(Pig::as_select()).load(db_connection.deref_mut());

    if sql_res.is_ok() {
        Ok(Json(sql_res.unwrap()))
    } else {
        error!("Unable to load SQL result for query {:?}: {:?}", query, sql_res.unwrap_err());
        Err(Status::InternalServerError)
    }
}
