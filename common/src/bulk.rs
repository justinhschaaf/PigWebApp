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

impl BulkPatch {
    pub fn new(id: &Uuid) -> Self {
        Self { id: id.to_owned(), pending: None, accepted: None, rejected: None }
    }

    pub fn pending(mut self, action: PatchAction<String>) -> Self {
        if self.pending.is_none() {
            self.pending = Some(Vec::new());
        }

        self.pending.as_mut().unwrap().push(action);

        self
    }

    pub fn accepted(mut self, action: PatchAction<Uuid>) -> Self {
        if self.accepted.is_none() {
            self.accepted = Some(Vec::new());
        }

        self.accepted.as_mut().unwrap().push(action);

        self
    }

    pub fn rejected(mut self, action: PatchAction<String>) -> Self {
        if self.rejected.is_none() {
            self.rejected = Some(Vec::new());
        }

        self.rejected.as_mut().unwrap().push(action);

        self
    }

    pub fn update_import(&self, import: &mut BulkImport) {
        if let Some(pending_actions) = self.pending.as_ref() {
            Self::perform_actions(pending_actions, &mut import.pending);
        }

        if let Some(accepted_actions) = self.accepted.as_ref() {
            Self::perform_actions(accepted_actions, &mut import.accepted);
        }

        if let Some(rejected_actions) = self.rejected.as_ref() {
            Self::perform_actions(rejected_actions, &mut import.rejected);
        }
    }

    pub fn perform_actions<T: PartialEq + Clone>(actions: &Vec<PatchAction<T>>, vec: &mut Vec<T>) {
        for action in actions {
            match action {
                PatchAction::ADD(e) => vec.push(e.clone()),
                PatchAction::REMOVE(e) => {
                    // .and_then expects the lambda to return an Option, but we don't care about it
                    let pos = vec.iter().position(|r| r.eq(e));
                    pos.and_then(|i| Some(vec.remove(i)));
                }
                PatchAction::UPDATE(old, new) => {
                    let pos = vec.iter().position(|r| r.eq(old));
                    pos.and_then(|i| Some(vec[i] = new.clone()));
                }
            }
        }
    }
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
