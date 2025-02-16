pub mod yuri;

use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "server")]
pub mod schema;

#[cfg(feature = "server")]
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};

/// The relative base URL for all Pig API routes
pub const PIG_API_ROOT: &str = "/api/pigs/";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(AsChangeset, Identifiable, Insertable, Queryable, Selectable))]
#[cfg_attr(feature = "server", diesel(table_name = crate::schema::pigs))]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "server", diesel(treat_none_as_null = true))]
pub struct Pig {
    // as this is the key in the db it won't be changed, no extra work needed
    pub id: Uuid,

    // never, never, never, never, never, never, NEVER change this to a str or else it will FUCK EVERYTHING
    pub name: String,

    // skip updating this field in the db as we don't want it to change
    // TODO enable this in diesel 2.3.0
    // https://github.com/diesel-rs/diesel/pull/4364
    //#[cfg_attr(feature = "server", diesel(skip_update))]
    pub created: NaiveDateTime,
}

impl Pig {
    /// Creates a new pig with a random UUID and the given name at the current
    /// timestamp.
    pub fn create(name: &str) -> Pig {
        Pig { id: Uuid::new_v4(), name: name.to_owned(), created: Utc::now().naive_utc() }
    }

    /// Merges this pig and the given one together, using the current pig as a
    /// base and only taking the values from the other Pig that can be changed.
    ///
    /// It's possible to have Pig objects always be immutable and have interior
    /// mutability using Cell to wrap them, but for the time being that would
    /// be more complex to use than I would like. The current method of having
    /// the server double check everything before committing is fine for now.
    /// https://stackoverflow.com/a/47748296
    /// https://doc.rust-lang.org/std/cell/struct.Cell.html#examples
    pub fn merge(&self, other: &Pig) -> Pig {
        Pig { name: other.name.to_owned(), ..*self }
    }
}

/// Represents all possible options in a query to fetch pigs. Every possible
/// parameter is an [Option] so all of them aren't absolutely required.
// https://stackoverflow.com/a/42551386
#[derive(Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "server", derive(rocket::FromForm))]
pub struct PigFetchQuery {
    // NOTE: all of these MUST be options or else Rocket won't recognize the query params
    // TODO add better functions for declaration, e.g. with_id(), with_ids(), with_name()
    pub id: Option<Vec<String>>,
    pub name: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

impl Default for PigFetchQuery {
    fn default() -> Self {
        Self { id: None, name: None, limit: Some(Self::get_default_limit()), offset: Some(0) }
    }
}

impl PigFetchQuery {
    pub fn get_default_limit() -> u32 {
        100
    }

    pub fn to_yuri(&self) -> String {
        yuri!(PIG_API_ROOT, "fetch" ;? query!(self))
    }
}
