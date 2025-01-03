use pigweb_common::Pig;
use rocket::form::validate::Contains;
use rocket::http::Status;
use rocket::response::status::Created;
use rocket::serde::json::Json;
use rocket::{Route, State};
use std::str::FromStr;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::{uuid, Uuid};

pub struct TempPigs {
    pigs: Vec<Pig>,
}

impl Default for TempPigs {
    // temp list of pigs to start on app logic
    // TODO replace with db
    fn default() -> Self {
        Self {
            pigs: vec![
                Pig {
                    id: uuid!("ab948298-4a8d-4166-b044-ff2a9fcbaf2d"),
                    name: "InfoWars.com".to_owned(),
                    created: 1734832007454,
                },
                Pig {
                    id: uuid!("1c87fa5d-da22-4a6e-85af-b92eaaec19b7"),
                    name: "Genocidal Oil Rig".to_owned(),
                    created: 1734832107454,
                },
                Pig {
                    id: uuid!("812822b4-2fa3-474f-83de-b4cf2fd3320e"),
                    name: "Mr. President".to_owned(),
                    created: 1734832207454,
                },
                Pig {
                    id: uuid!("b69ac7bf-adfa-48a2-8d7b-0f32aa55b0a0"),
                    name: "Megasota".to_owned(),
                    created: 1734832307454,
                },
                Pig {
                    id: uuid!("98aecf6e-fa9f-4ebb-a93e-1aa1e5218701"),
                    name: "Denial Code 286".to_owned(),
                    created: 1734832407454,
                },
                Pig {
                    id: uuid!("509371ee-b5fe-48a4-9958-8f3f6800970d"),
                    name: "Dodge Neon".to_owned(),
                    created: 1734832507454,
                },
                Pig {
                    id: uuid!("fe9484f6-4f9a-4d9e-805a-df83d114371d"),
                    name: "Before".to_owned(),
                    created: 1734832607454,
                },
                Pig {
                    id: uuid!("e3e69dd4-9c4e-4a56-9f74-cf2945509782"),
                    name: "After".to_owned(),
                    created: 1734832707454,
                },
                Pig {
                    id: uuid!("7db020ba-79e0-4f6b-bbfb-2b4a6a744bc7"),
                    name: "Brisket".to_owned(),
                    created: 1734832807454,
                },
                Pig {
                    id: uuid!("9a60441c-39be-4e5f-bb7e-6d5c5e58e6ee"),
                    name: "Bobby Moynihan".to_owned(),
                    created: 1734832907454,
                },
            ],
        }
    }
}

#[derive(Debug, PartialEq, FromForm)]
struct PigQuery {
    // Option is necessary to make it so both args aren't absolutely required
    id: Option<Vec<String>>,
    name: Option<String>,
}

pub fn get_pig_api_routes() -> Vec<Route> {
    routes![api_pig_create, api_pig_update, api_pig_delete, api_pig_fetch]
}

#[post("/create?<name>")]
async fn api_pig_create(
    temp_pigs_mut: &State<Mutex<TempPigs>>,
    name: &str,
) -> Result<Created<Json<Pig>>, (Status, &'static str)> {
    // Server should generate a UUID, determine the creating user and timestamp, save it to the DB, and return the final object
    // Deduplicating names should be your responsibility, dipshit
    // TODO deduplicate uuids?

    // Create the new pig
    let pig = Pig {
        id: Uuid::new_v4(),
        name: name.to_owned(),
        // https://www.cloudhadoop.com/rust-current-timestamp-millisecs-example#rust-current-time-in-milliseconds
        created: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64,
    };

    // We have to clone the pig for the json response because Json() wants ownership of it
    let res = Json(pig.clone());

    // Add the pig to the list
    let mut temp_pigs = temp_pigs_mut.lock().unwrap();
    temp_pigs.pigs.push(pig);

    // Respond with a path to the pig and the object itself, unfortunately the location path is mandatory
    // TODO does this HAVE to be a full URL, or is this fine?
    Ok(Created::new(format!("/api/pigs/fetch?id={}", res.id.to_string())).body(res))
}

#[put("/update", data = "<pig>")]
async fn api_pig_update(temp_pigs_mut: &State<Mutex<TempPigs>>, pig: Json<Pig>) -> (Status, &'static str) {
    // TODO add more checks to make sure read-only data isn't modified, we're testing rn so it's fine but later it won't be

    let uuid = pig.id;

    let mut temp_pigs = temp_pigs_mut.lock().unwrap();

    for (i, e) in temp_pigs.pigs.iter().enumerate() {
        if e.id == uuid {
            temp_pigs.pigs.remove(i);
            temp_pigs.pigs.insert(i, pig.into_inner());

            // there should only be one pig with this uuid, we need not continue
            return (Status::Ok, "Pig successfully updated");
        }
    }

    (Status::NotFound, "Unable to find pig")
}

#[delete("/delete?<id>")]
async fn api_pig_delete(temp_pigs_mut: &State<Mutex<TempPigs>>, id: &str) -> (Status, &'static str) {
    let uuid = match Uuid::from_str(id) {
        Ok(i) => i,
        Err(_) => return (Status::BadRequest, "Invalid UUID input"),
    };

    let mut temp_pigs = temp_pigs_mut.lock().unwrap();
    for (i, v) in temp_pigs.pigs.iter().enumerate() {
        if v.id == uuid {
            temp_pigs.pigs.remove(i);
            return (Status::NoContent, "Pig successfully deleted");
        }
    }

    (Status::NotFound, "Unable to find pig")
}

// the lifetimes here have to be named for whatever reason
// you may be able to tell i'm getting annoyed with these little shits being everywhere
#[get("/fetch?<query..>")]
async fn api_pig_fetch(
    temp_pigs_mut: &State<Mutex<TempPigs>>,
    query: PigQuery,
) -> Result<Json<Vec<Pig>>, (Status, &'static str)> {
    let mut ids: Option<Vec<Uuid>> = None;
    let mut res = Vec::new();

    // Convert IDs to UUIDs, if present
    // https://stackoverflow.com/a/16756324
    if query.id.is_some() {
        match query.id.unwrap_or_default().into_iter().map(|e| uuid::Uuid::from_str(e.as_str())).collect() {
            Ok(i) => ids = Some(i),
            Err(_) => return Err((Status::BadRequest, "Invalid UUID input")),
        }
    }

    let temp_pigs = temp_pigs_mut.lock().unwrap();
    let pigs: Vec<Pig> = temp_pigs.pigs.iter().cloned().collect();
    if ids.is_none() && query.name.is_none() {
        // Extend maintains the items in the original vec, append does not
        res.extend(pigs)
    } else {
        pigs.into_iter().for_each(|pig| {
            // Name should be a search with Tantivy, ID should fetch pigs by their ID
            // unified until all this shit gets implemented
            // Add a limit to the number of results? we don't wanna return the whole fucking database
            if ids.as_ref().is_some_and(|ids| ids.contains(pig.id))
                || query
                    .name
                    .as_ref()
                    .is_some_and(|name| pig.name.to_uppercase().contains(name.to_uppercase().as_str()))
            {
                res.push(pig);
            }
        });
    }

    Ok(Json(res))
}
