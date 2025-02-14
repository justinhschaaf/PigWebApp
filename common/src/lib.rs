mod yuri;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The relative base URL for all Pig API routes
pub const PIG_API_ROOT: &str = "/api/pigs/";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pig {
    pub id: Uuid,
    // never, never, never, never, never, never, NEVER change this to a str or else it will FUCK EVERYTHING
    pub name: String,
    pub created: i64,
}

impl Pig {
    /// Creates a new pig with a random UUID and the given name at the current
    /// timestamp.
    pub fn create(name: &str) -> Pig {
        Pig { id: Uuid::new_v4(), name: name.to_owned(), created: Utc::now().timestamp_millis() }
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
    // TODO add limit on number of results here? maybe upper and lower bound? idfk
    // TODO add better functions for declaration, e.g. with_id(), with_ids(), with_name()
    pub id: Option<Vec<String>>,
    pub name: Option<String>,
}

impl Default for PigFetchQuery {
    fn default() -> Self {
        Self { id: None, name: None }
    }
}
