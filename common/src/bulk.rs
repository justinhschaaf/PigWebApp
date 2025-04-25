use crate::{query_limit_offset, query_list, query_to_yuri, BULK_API_ROOT, DEFAULT_API_RESPONSE_LIMIT};
use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "server")]
use {crate::schema, diesel::*};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(
    feature = "server",
    derive(diesel::AsChangeset, diesel::Identifiable, diesel::Insertable, diesel::Queryable, diesel::Selectable)
)]
#[cfg_attr(feature = "server", diesel(table_name = crate::schema::bulk_imports))]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "server", diesel(treat_none_as_null = true))]
pub struct BulkImport {
    pub id: Uuid,
    pub name: String,
    pub creator: Uuid,
    pub started: NaiveDateTime,
    pub finished: Option<NaiveDateTime>,
    pub pending: Vec<String>,
    pub accepted: Vec<Uuid>,
    pub rejected: Vec<String>,
}

impl BulkImport {
    pub fn new(
        name: &String,
        creator: &Uuid,
        pending: &Vec<String>,
        accepted: &Vec<Uuid>,
        rejected: &Vec<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_owned(),
            creator: creator.to_owned(),
            started: Utc::now().naive_utc(),
            finished: None,
            pending: pending.to_owned(),
            accepted: accepted.to_owned(),
            rejected: rejected.to_owned(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum PatchAction<T> {
    ADD(T),
    REMOVE(T),
    UPDATE(T, T),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BulkPatch {
    pub id: Uuid,
    pub pending: Option<Vec<PatchAction<String>>>,
    pub accepted: Option<Vec<PatchAction<Uuid>>>,
    pub rejected: Option<Vec<PatchAction<String>>>,
}

#[derive(Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "server", derive(rocket::FromForm))]
pub struct BulkQuery {
    pub id: Option<Vec<String>>,
    pub creator: Option<Vec<String>>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

impl Default for BulkQuery {
    fn default() -> Self {
        Self { id: None, creator: None, limit: Some(DEFAULT_API_RESPONSE_LIMIT), offset: Some(0) }
    }
}

impl BulkQuery {
    query_list!(id, Uuid);
    query_list!(creator, Uuid);
    query_limit_offset!();
    query_to_yuri!(BULK_API_ROOT);

    /// Converts query params to DB query
    #[cfg(feature = "server")]
    #[dsl::auto_type(no_type_alias)]
    pub fn to_db_select(&self) -> _ {
        let mut res: helper_types::IntoBoxed<schema::bulk_imports::table, pg::Pg> =
            schema::bulk_imports::table.into_boxed();

        // Filter by id, if specified
        if let Some(query_ids) = self.id.as_ref().and_then(|ids| crate::parse_uuids(ids).ok()) {
            res = res.filter(schema::bulk_imports::id.eq_any(query_ids));
        }

        // Filter by creator, if specified
        if let Some(query_creators) = self.creator.as_ref().and_then(|ids| crate::parse_uuids(ids).ok()) {
            res = res.filter(schema::bulk_imports::creator.eq_any(query_creators));
        }

        // Set the limit, if present
        res = res.limit(self.limit.unwrap_or_else(|| DEFAULT_API_RESPONSE_LIMIT) as i64);

        // Set the offset, if present
        if let Some(offset) = self.offset {
            if offset > 0 {
                res = res.offset(offset as i64);
            }
        }

        res
    }
}
