use crate::{query, yuri, PIG_API_ROOT};
use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "server")]
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};

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

    pub creator: Uuid,
}

impl Pig {
    /// Creates a new pig with a random UUID and the given name at the current
    /// timestamp.
    pub fn new(name: &str, creator: &Uuid) -> Pig {
        Pig { id: Uuid::new_v4(), name: name.to_owned(), created: Utc::now().naive_utc(), creator: creator.to_owned() }
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

    pub fn with_id(self, id: &Uuid) -> Self {
        self.with_ids(vec![id.to_owned()])
    }

    pub fn with_id_string(self, id: &String) -> Self {
        self.with_ids_string(vec![id.to_owned()])
    }

    pub fn with_ids(self, ids: Vec<Uuid>) -> Self {
        self.with_ids_string(ids.iter().map(|e| e.to_string()).collect())
    }

    pub fn with_ids_string(mut self, ids: Vec<String>) -> Self {
        self.id = Some(ids);
        self
    }

    pub fn with_name(mut self, name: &String) -> Self {
        self.name = Some(name.to_owned());
        self
    }

    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_offset(mut self, offset: u32) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn to_yuri(&self) -> String {
        yuri!(PIG_API_ROOT, "fetch" ;? query!(self))
    }
}
