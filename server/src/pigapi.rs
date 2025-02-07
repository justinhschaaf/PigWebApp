use pigweb_common::{query, yuri, Pig, PigFetchQuery, PIG_API_ROOT};
use rocket::form::validate::Contains;
use rocket::http::Status;
use rocket::response::status::Created;
use rocket::serde::json::Json;
use rocket::{Route, State};
use std::str::FromStr;
use std::sync::Mutex;
use uuid::Uuid;

pub struct TempPigs {
    pigs: Vec<Pig>,
}

impl Default for TempPigs {
    // temp list of pigs to start on app logic
    // TODO replace with db
    fn default() -> Self {
        let names = vec![
            "Ozempic",
            "The C Programming Language, 2nd Edition",
            "Mr. Clean's Trench Eraser",
            "Jerry's Nugget Casino",
            "The couch in the women's restroom",
            "Shock and Awe",
            "Shock and Aww",
            "\"Nanomachines, son.\"",
            "Buckyballs",
            "The 1,200 government-owned goats that fight wildfires in Colorado",
            "Brisket",
            "Bobby Moynihan",
            "Caffe Trilussa",
            "Microsoft's meth lab in Sweden",
            "Southwest Airlines running their servers on Windows 3.1",
            "Johnny Manchild",
            "Fold yourself 12 times",
            "\"Next to him!\"",
            "Before",
            "After",
            "Dodge Neon",
            "Hong Xiuquan",
            "Patty",
            "Selma",
            "James Dean's Jacket",
            "Jimmy's Mom",
            "Eminem wearing Frank Sinatra's tuxedo",
            "\"Better is a poor man who walks in his integrity than a rich man who is crooked in his ways.\" - Proverbs 28:6",
            "Breloom",
            "Denial Code 286",
            "John Brown",
            "Parson Brown",
            "Slow Mobius",
            "\"He killed my wife!\"",
            "Larry Craig",
            "Quango",
            "Emperor Popeatine",
            "A bomb",
            "$8 for a side of bacon",
            "Euler",
            "Rhombic Dodecahedron",
            "Genocidal Oil Rig",
            "A byzantine financial struggle to own InfoWars.com",
            "Megasota",
            "The sea in storm",
            "A night with no moon",
            "The anger of a gentle man",
            "The President",
            "Mr. President",
            "A big Mack",
            "\"I need to go iron my dog.\"",
            "Gymnophobia",
            "Springtime for Hitler",
            "Headpats",
            "Entscheidungsproblem",
            "\"I am the plot, and you need the armor.\"",
            "The NX-5 Planet Remover, brought to you by Xamamax in partnership with Wrangler Jeans",
            "People sit on chairs",
            "Uranus Fudge Factory & General Store",
            "FRONT TOWARD ENEMY",
            "Lightsaber Gatling Gun",
            "Semper Fi",
            "Porkbun",
            "Toyotathon",
            "Bacardi 151",
            "5 unopened jars of dill pickles",
            "6 bottles of Mrs. Butterworth's Syrup",
            "Ambassador",
            "Isbassador",
            "\"What walks the most?\"",
            "The World's Largest Parking Lot",
            "15-minute city",
            "15-hour city",
            "Malice in Wonderland",
            "Clarke's Third Law",
            "Bechdel Test",
            "Complexity Addiction",
            "Shoney's",
            "Purpose Robot",
            "A notary",
            "Dropping a lightsaber perfectly vertical",
            "Totally not a robot",
            "Bringing your pet lobster on an airplane",
            "Previous Leon",
            "Miss Lead",
            "Flash Back",
            "Connie TinuityError",
            "Mr. Twist",
            "Protago Nick",
            "Rhett Caan",
            "Pat Gelsinger",
            "Sun SPARC",
            "\"It killed me to lie to you, Morty, but it would've literally killed me not to lie to you.\"",
            "The Polycrisis",
            "Japan",
            "Maria Montessori",
            "Industry Americus Collins",
            "Verbal Backspace",
            "Implausible Deniability",
            "Plausible Deniability",
            "Previously on Jesus Christ",
            "King",
            "The Presidential Black Hawk",
            "American McGee",
            "Area 51",
            "Bowdlerisation",
            "The Microsoft Excel 95 Hall of Tortured Souls",
            "Morphine",
            "The Weave",
            "The smartest man in the universe using a pickaxe to break wood",
            "Asmongold's Lair",
            "Having Elvis officiate your wedding",
            "John Helldiver",
            "\"Can you make this?\"",
            "Neurospicy",
            "The EU fines the EU for violating the GDPR",
            "Zohan",
            "\"Because I can.\"",
            "Having a username so long you don't need a password for it",
            "The Law of Conservation of Corporate Generosity",
            "Big \"They\"",
            "Loss",
        ];

        let mut pigs: Vec<Pig> = Vec::new();

        for name in names {
            pigs.push(Pig { id: Uuid::new_v4(), name: name.to_owned(), created: 1734832007454 });
        }

        Self { pigs }
    }
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
    let pig = Pig::create(name);

    // We have to clone the pig for the json response because Json() wants ownership of it
    let res = Json(pig.clone());

    // Add the pig to the list
    let mut temp_pigs = temp_pigs_mut.lock().unwrap();
    temp_pigs.pigs.push(pig);

    // Respond with a path to the pig and the object itself, unfortunately the location path is mandatory
    let params = PigFetchQuery { id: Some(Vec::from([res.id.to_string()])), name: None };
    let loc = yuri!(PIG_API_ROOT, "fetch" ;? query!(params));
    Ok(Created::new(loc).body(res))
}

#[put("/update", data = "<pig>")]
async fn api_pig_update(temp_pigs_mut: &State<Mutex<TempPigs>>, pig: Json<Pig>) -> (Status, &'static str) {
    let uuid = pig.id;

    let mut temp_pigs = temp_pigs_mut.lock().unwrap();

    for (i, e) in temp_pigs.pigs.iter().enumerate() {
        if e.id == uuid {
            // use merge to protect read-only data
            // TODO should it return the correct pig object in case the data has changed? Yes, yes it should.
            let merged = temp_pigs.pigs.remove(i).merge(&pig.into_inner());
            temp_pigs.pigs.insert(i, merged);

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
    query: PigFetchQuery,
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
