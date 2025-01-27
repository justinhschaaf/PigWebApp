use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub const PIG_API_ROOT: &str = "/api/pigs/";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Pig {
    pub id: Uuid,
    // never, never, never, never, never, never, NEVER change this to a str or else it will FUCK EVERYTHING
    pub name: String,
    pub created: u64,
}

impl Pig {
    /// Creates a new pig with a random UUID and the given name at the current
    /// timestamp.
    pub fn create(name: &str) -> Pig {
        Pig {
            id: Uuid::new_v4(),
            name: name.to_owned(),
            // https://www.cloudhadoop.com/rust-current-timestamp-millisecs-example#rust-current-time-in-milliseconds
            created: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64,
        }
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
