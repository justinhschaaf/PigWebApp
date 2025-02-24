use crate::auth::AuthenticatedUser;
use diesel::{ExpressionMethods, PgConnection, PgTextExpressionMethods, QueryDsl, RunQueryDsl, SelectableHelper};
use diesel_full_text_search::{plainto_tsquery, to_tsvector, TsVectorExtensions};
use pigweb_common::pigs::{Pig, PigFetchQuery};
use pigweb_common::schema;
use rocket::http::Status;
use rocket::response::status::Created;
use rocket::serde::json::Json;
use rocket::{Route, State};
use std::ops::DerefMut;
use std::str::FromStr;
use std::sync::Mutex;
use uuid::Uuid;

pub fn get_pig_api_routes() -> Vec<Route> {
    routes![api_pig_create, api_pig_update, api_pig_delete, api_pig_fetch]
}

#[post("/create?<name>")]
async fn api_pig_create(
    auth_user: &AuthenticatedUser,
    db_connection: &State<Mutex<PgConnection>>,
    name: &str,
) -> Result<Created<Json<Pig>>, (Status, &'static str)> {
    // TODO deduplicate uuids and names

    // Create the new pig
    let pig = Pig::new(name, auth_user.user.id.as_ref());

    // Save it to the DB
    let mut db_connection = db_connection.lock().unwrap();
    let sql_res = diesel::insert_into(schema::pigs::table).values(&pig).execute(db_connection.deref_mut());

    if sql_res.is_ok() {
        // Respond with a path to the pig and the object itself, unfortunately the location path is mandatory
        let params = PigFetchQuery { id: Some(Vec::from([pig.id.to_string()])), ..Default::default() };
        Ok(Created::new(params.to_yuri()).body(Json(pig)))
    } else {
        error!("Unable to save new pig {:?}: {:?}", pig, sql_res.unwrap_err());
        Err((Status::InternalServerError, "Error saving new pig"))
    }
}

#[put("/update", data = "<pig>")]
async fn api_pig_update(
    _auth_user: &AuthenticatedUser,
    db_connection: &State<Mutex<PgConnection>>,
    pig: Json<Pig>,
) -> Result<Json<Pig>, (Status, &'static str)> {
    let pig = pig.into_inner();
    let mut db_connection = db_connection.lock().unwrap();

    // Because Pig derives Identifiable and AsChangeset it just kinda knows what needs to be updated
    let sql_res = diesel::update(schema::pigs::table).set(&pig).get_result(db_connection.deref_mut());

    if sql_res.is_ok() {
        // Return the updated pig
        Ok(Json(sql_res.unwrap()))
    } else {
        error!("Unable to update pig {:?}: {:?}", pig, sql_res.unwrap_err());
        Err((Status::InternalServerError, "Unable to update pig."))
    }
}

#[delete("/delete?<id>")]
async fn api_pig_delete(
    _auth_user: &AuthenticatedUser,
    db_connection: &State<Mutex<PgConnection>>,
    id: &str,
) -> (Status, &'static str) {
    let uuid = match Uuid::from_str(id) {
        Ok(i) => i,
        Err(_) => return (Status::BadRequest, "Invalid UUID input"),
    };

    let mut db_connection = db_connection.lock().unwrap();
    let sql_res =
        diesel::delete(schema::pigs::table.filter(schema::pigs::id.eq(uuid))).execute(db_connection.deref_mut());

    if sql_res.is_ok() {
        (Status::NoContent, "Pig successfully deleted")
    } else {
        error!("Unable to delete pig {:?}: {:?}", id, sql_res.unwrap_err());
        (Status::InternalServerError, "Unable to delete pig.")
    }
}

#[get("/fetch?<query..>")]
async fn api_pig_fetch(
    _auth_user: &AuthenticatedUser,
    db_connection: &State<Mutex<PgConnection>>,
    query: PigFetchQuery,
) -> Result<Json<Vec<Pig>>, (Status, &'static str)> {
    let mut ids: Option<Vec<Uuid>> = None;
    let mut limit = PigFetchQuery::get_default_limit();

    // Convert IDs to UUIDs, if present
    // https://stackoverflow.com/a/16756324
    if let Some(ref id) = query.id {
        match id.iter().map(|e| uuid::Uuid::from_str(e.as_str())).collect() {
            Ok(i) => ids = Some(i),
            Err(_) => return Err((Status::BadRequest, "Invalid UUID input")),
        }
    }

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
        Err((Status::InternalServerError, "Unable to load requested data."))
    }
}
