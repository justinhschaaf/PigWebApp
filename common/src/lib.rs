use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
pub struct Pig<'r> {
    id: Uuid,
    name: &'r str,
    created: u64,
}
