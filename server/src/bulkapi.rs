use crate::auth::AuthenticatedUser;
use crate::config::Config;
use chrono::Utc;
use diesel::{ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl, SelectableHelper};
use pigweb_common::bulk::{BulkImport, BulkPatch, BulkQuery};
use pigweb_common::pigs::{Pig, PigQuery};
use pigweb_common::schema;
use pigweb_common::users::Roles;
use rocket::http::Status;
use rocket::response::status::Created;
use rocket::serde::json::Json;
use rocket::{Route, State};
use std::ops::DerefMut;
use std::sync::Mutex;
use uuid::Uuid;

/// Returns a list of all bulk api routes
pub fn get_bulk_api_routes() -> Vec<Route> {
    routes![api_bulk_create, api_bulk_patch, api_bulk_fetch]
}

/// Starts a bulk import from the JSON list of pig names given in the request
/// body. Returns the BulkImport as JSON.
#[post("/create", data = "<names>")]
async fn api_bulk_create(
    auth_user: AuthenticatedUser,
    config: &State<Config>,
    db_connection: &State<Mutex<PgConnection>>,
    names: Json<Vec<String>>,
) -> Result<Created<Json<BulkImport>>, Status> {
    if !auth_user.has_role(config, Roles::BulkEditor) {
        return Err(Status::Forbidden);
    }

    let inputs = names.into_inner();
    let mut db_connection = db_connection.lock().unwrap();

    // Actual values for the BulkImport struct
    let mut import_name = None;
    let started = Utc::now().naive_utc();
    let mut finished = None;
    let mut pending = Vec::new();
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();

    // for each input name
    // TODO can we run this concurrently?
    for input in inputs {
        // Start with initial cleanup
        let mut name = String::new();
        input.trim().chars().for_each(|c| {
            name.push(match c {
                '“' | '”' => '"',
                '‘' | '’' => '\'',
                '‒' | '–' | '—' | '⸺' | '⸻' => '-',
                _ => c,
            })
        });

        // set the import name, if not set already
        if import_name.is_none() {
            import_name = Some(name.to_owned());
        }

        // if this name is a duplicate of an already pending pig, skip it entirely
        if pending.contains(&name) {
            continue;
        }

        // Search for duplicates
        let query = PigQuery::default().with_name(&name).with_limit(10);
        let duplicates_sql_query = query.to_db_select();
        let duplicates_sql_res = duplicates_sql_query.select(Pig::as_select()).load(db_connection.deref_mut());

        if let Ok(duplicates) = duplicates_sql_res {
            // if we have duplicates and the first one is an exact duplicate, reject it
            if duplicates.len() > 0 {
                if duplicates.get(1).is_some_and(|pig| pig.name.eq_ignore_ascii_case(name.as_str())) {
                    // we have an exact duplicate, add to rejected
                    rejected.push(name);
                } else {
                    // duplicate isn't exact, looking into it
                    pending.push(name);
                }
            } else {
                // we should only get to this case if we have no duplicates, in which case add the pig
                let pig = Pig::new(name.as_str(), auth_user.user.id.as_ref());
                let create_sql_res =
                    diesel::insert_into(schema::pigs::table).values(&pig).execute(db_connection.deref_mut());

                if create_sql_res.is_ok() {
                    // create went through successfully
                    accepted.push(pig.id);
                } else {
                    // the create request didn't go through, add to pending
                    pending.push(name);
                }
            }
        } else {
            pending.push(name);
        }
    }

    // if there are no pending pigs left we're done here
    if pending.len() == 0 {
        finished = Some(Utc::now().naive_utc());
    }

    // create the response struct
    let res = BulkImport {
        id: Uuid::new_v4(),
        name: import_name.unwrap_or_default(),
        creator: auth_user.user.id,
        started,
        finished,
        pending,
        accepted,
        rejected,
    };

    // Save it to the DB
    let sql_res = diesel::insert_into(schema::bulk_imports::table).values(&res).execute(db_connection.deref_mut());

    if sql_res.is_ok() {
        let params = BulkQuery::default().with_id(&res.id);
        Ok(Created::new(params.to_yuri()).body(Json(res)))
    } else {
        error!("Unable to save new bulk import {:?}: {:?}", res, sql_res.unwrap_err());
        Err(Status::InternalServerError)
    }
}

/// Updates a BulkImport with the actions in the request body. Returns HTTP
/// status code 200 if changes are successful.
#[patch("/patch", data = "<actions>")]
async fn api_bulk_patch(
    auth_user: AuthenticatedUser,
    config: &State<Config>,
    db_connection: &State<Mutex<PgConnection>>,
    actions: Json<BulkPatch>,
) -> Status {
    if !auth_user.has_role(config, Roles::BulkEditor) {
        return Status::Forbidden;
    }
    let actions = actions.into_inner();

    // Get object from the DB
    let mut db_connection = db_connection.lock().unwrap();
    let query = BulkQuery::default().with_id(&actions.id).with_limit(1);
    let sql_req_res = query.to_db_select().select(BulkImport::as_select()).load(db_connection.deref_mut());

    if let Ok(mut imports) = sql_req_res {
        if imports.len() != 1 {
            error!(
                "Found too many or too few BulkImports when updating! id: {:?}, matches: {:?}",
                actions.id,
                imports.len()
            );
            return Status::InternalServerError;
        }

        // Perform updates
        let mut import = imports.pop().unwrap();
        actions.update_import(&mut import);

        // if there are no pending pigs left we're done here
        if import.pending.len() == 0 {
            import.finished = Some(Utc::now().naive_utc());
        }

        // Save changes
        // we need to manually filter the id out here, whereas it just works when updating the pigs table (well, it did)
        // why did they break it? no fucking clue.
        let sql_res = diesel::update(schema::bulk_imports::table)
            .filter(schema::bulk_imports::id.eq(&import.id))
            .set(&import)
            .execute(db_connection.deref_mut());

        if sql_res.is_ok() {
            Status::Ok
        } else {
            error!("Unable to save BulkImport patch changes! err: {:?}", sql_res.unwrap_err());
            Status::InternalServerError
        }
    } else {
        error!(
            "Unable to load SQL result for BulkImport update! query: {:?}, err: {:?}",
            query,
            sql_req_res.unwrap_err()
        );
        Status::InternalServerError
    }
}

/// Returns a JSON list of BulkImports which match the given query.
#[get("/fetch?<query..>")]
async fn api_bulk_fetch(
    auth_user: AuthenticatedUser,
    config: &State<Config>,
    db_connection: &State<Mutex<PgConnection>>,
    query: BulkQuery,
) -> Result<Json<Vec<BulkImport>>, Status> {
    let mut query = query;
    let bulk_admin = auth_user.has_role(config, Roles::BulkAdmin);

    // If the user is not a BulkAdmin or BulkEditor, this is forbidden to them
    if !(bulk_admin || auth_user.has_role(config, Roles::BulkEditor)) {
        return Err(Status::Forbidden);
    }

    // If the user is not a BulkAdmin, only let them see their own
    if !bulk_admin {
        query = BulkQuery { creator: Some(vec![auth_user.user.id.to_string()]), ..query }
    }

    // Fetch from the DB
    let sql_query = query.to_db_select();
    let mut db_connection = db_connection.lock().unwrap();
    let sql_res = sql_query.select(BulkImport::as_select()).load(db_connection.deref_mut());

    if let Ok(imports) = sql_res {
        Ok(Json(imports))
    } else {
        error!("Unable to load SQL result for query {:?}: {:?}", query, sql_res.unwrap_err());
        Err(Status::InternalServerError)
    }
}
