use chrono::{TimeZone, Utc};
use diesel::query_builder::QueryBuilder;
use diesel::{sql_query, ExpressionMethods, Insertable, PgConnection, QueryDsl, RunQueryDsl, SelectableHelper};
use diesel_full_text_search::{to_tsquery, to_tsvector};
use pigweb_common::{query, schema, Pig, PigFetchQuery};
use rocket::form::validate::Contains;
use rocket::http::Status;
use rocket::response::status::Created;
use rocket::serde::json::Json;
use rocket::{Route, State};
use std::error::Error;
use std::str::FromStr;
use std::sync::Mutex;
use uuid::Uuid;

pub fn get_pig_api_routes() -> Vec<Route> {
    routes![api_pig_create, api_pig_update, api_pig_delete, api_pig_fetch]
}

#[post("/create?<name>")]
async fn api_pig_create(
    db_connection: State<Mutex<PgConnection>>,
    name: &str,
) -> Result<Created<Json<Pig>>, (Status, &'static str)> {
    // TODO deduplicate uuids and names

    // Create the new pig
    let pig = Pig::create(name);

    // Save it to the DB
    let mut db_connection = db_connection.lock().unwrap();
    let sql_res = diesel::insert_into(schema::pigs::table).values(&pig).execute(db_connection.into());

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
    db_connection: State<Mutex<PgConnection>>,
    pig: Json<Pig>,
) -> Result<Json<Pig>, (Status, &'static str)> {
    let pig = pig.into_inner();
    let mut db_connection = db_connection.lock().unwrap();

    // Because Pig derives Identifiable and AsChangeset it just kinda knows what needs to be updated
    let sql_res = diesel::update(schema::pigs::table).set(&pig).get_result(db_connection.into());

    if sql_res.is_ok() {
        // Return the updated pig
        Ok(Json(sql_res.unwrap()))
    } else {
        error!("Unable to update pig {:?}: {:?}", pig, sql_res.unwrap_err());
        Err((Status::InternalServerError, "Unable to update pig."))
    }
}

#[delete("/delete?<id>")]
async fn api_pig_delete(db_connection: &State<Mutex<PgConnection>>, id: &str) -> (Status, &'static str) {
    let uuid = match Uuid::from_str(id) {
        Ok(i) => i,
        Err(_) => return (Status::BadRequest, "Invalid UUID input"),
    };

    let mut db_connection = db_connection.lock().unwrap();
    let sql_res = diesel::delete(schema::pigs::table.filter(schema::pigs::id.eq(uuid))).execute(db_connection.into());

    if sql_res.is_ok() {
        (Status::NoContent, "Pig successfully deleted")
    } else {
        error!("Unable to delete pig {:?}: {:?}", id, sql_res.unwrap_err());
        (Status::InternalServerError, "Unable to delete pig.")
    }
}

#[get("/fetch?<query..>")]
async fn api_pig_fetch(
    db_connection: &State<Mutex<PgConnection>>,
    query: PigFetchQuery,
) -> Result<Json<Vec<Pig>>, (Status, &'static str)> {
    let mut ids: Option<Vec<Uuid>> = None;

    // Convert IDs to UUIDs, if present
    // https://stackoverflow.com/a/16756324
    if query.id.is_some() {
        match query.id.unwrap_or_default().into_iter().map(|e| uuid::Uuid::from_str(e.as_str())).collect() {
            Ok(i) => ids = Some(i),
            Err(_) => return Err((Status::BadRequest, "Invalid UUID input")),
        }
    }

    // Start constructing the SQL query
    let mut sql_query: dyn QueryBuilder<_> = schema::pigs::table;

    // Filter by name, if specified
    if let Some(query_name) = query.name {
        // This performs a full text search
        sql_query = sql_query.filter(to_tsvector(schema::pigs::name).matches(to_tsquery(query_name)));
    }

    // Filter by id, if specified
    if let Some(query_ids) = ids {
        sql_query = sql_query.filter(schema::pigs::id.eq_any(query_ids));
    }

    // Set the offset, if present
    if query.offset > 0 {
        sql_query = sql_query.offset(query.offset as i64);
    }

    // Set the limit and submit the query to the DB
    let db_connection = db_connection.lock().unwrap();
    let sql_res = sql_query.limit(query.limit as i64).select(Pig::as_select()).load(db_connection);

    if sql_res.is_ok() {
        Ok(Json(sql_res.unwrap()))
    } else {
        error!("Unable to load SQL result for query {:?}: {:?}", query, sql_res.unwrap_err());
        Err((Status::InternalServerError, "Unable to load requested data."))
    }
}
