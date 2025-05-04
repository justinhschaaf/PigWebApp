use crate::{query_limit_offset, query_list, query_to_yuri, BULK_API_ROOT, DEFAULT_API_RESPONSE_LIMIT};
use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "server")]
use {crate::schema, diesel::*};

/// A list of pigs names imported at once. Names start in the [`pending`] list
/// before a pig is generated whose [`Uuid`] is [`accepted`] or the name is
/// [`rejected`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(
    feature = "server",
    derive(diesel::AsChangeset, diesel::Identifiable, diesel::Insertable, diesel::Queryable, diesel::Selectable)
)]
#[cfg_attr(feature = "server", diesel(table_name = crate::schema::bulk_imports))]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "server", diesel(treat_none_as_null = true))]
pub struct BulkImport {
    /// The unique id for this import
    pub id: Uuid,

    /// A human-friendly name for the import, usually the first valid name from
    /// the pending list when created.
    pub name: String,

    /// The id of the user who started importing these names
    pub creator: Uuid,

    /// When the import was created
    pub started: NaiveDateTime,

    /// When the last name was removed from the [`pending`] list, marking the
    /// import as complete. If this is [`None`], the import should be considered
    /// still in-progress.
    pub finished: Option<NaiveDateTime>,

    /// The list of names still waiting to be processed
    pub pending: Vec<String>,

    /// The ids of each pig created from this import
    pub accepted: Vec<Uuid>,

    /// The names from the import which were not added to the list
    pub rejected: Vec<String>,
}

impl BulkImport {
    /// Creates a new BulkImport from the given values with the current time as
    /// [`started`] and a [`finished`] time of [`None`].
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

/// A single modification to a BulkImport list.
#[derive(Debug, Serialize, Deserialize)]
pub enum PatchAction<T> {
    /// Adds the given value to the list
    ADD(T),

    /// Removes the given value from the list
    REMOVE(T),

    /// Replaces the first given value with the second
    UPDATE(T, T),
}

/// A request to modify a [`BulkImport`]. Patches are used instead of replacing
/// the object in-full to hopefully reduce the amount of data transmitted
/// between client and server.
#[derive(Debug, Serialize, Deserialize)]
pub struct BulkPatch {
    /// The id of the [`BulkImport`] to modify.
    pub id: Uuid,

    /// Changes to the [`BulkImport`] pending list
    pub pending: Option<Vec<PatchAction<String>>>,

    /// Changes to the [`BulkImport`] accepted list
    pub accepted: Option<Vec<PatchAction<Uuid>>>,

    /// Changes to the [`BulkImport`] rejected list
    pub rejected: Option<Vec<PatchAction<String>>>,
}

impl BulkPatch {
    /// Creates a new BulkPatch to apply to the [`BulkImport`] with the given id
    pub fn new(id: &Uuid) -> Self {
        Self { id: id.to_owned(), pending: None, accepted: None, rejected: None }
    }

    /// Adds a change to the [`BulkImport`] pending list
    pub fn pending(mut self, action: PatchAction<String>) -> Self {
        if self.pending.is_none() {
            self.pending = Some(Vec::new());
        }

        self.pending.as_mut().unwrap().push(action);

        self
    }

    /// Adds a change to the [`BulkImport`] accepted list
    pub fn accepted(mut self, action: PatchAction<Uuid>) -> Self {
        if self.accepted.is_none() {
            self.accepted = Some(Vec::new());
        }

        self.accepted.as_mut().unwrap().push(action);

        self
    }

    /// Adds a change to the [`BulkImport`] rejected list
    pub fn rejected(mut self, action: PatchAction<String>) -> Self {
        if self.rejected.is_none() {
            self.rejected = Some(Vec::new());
        }

        self.rejected.as_mut().unwrap().push(action);

        self
    }

    /// Applies the changes in this patch to the given BulkImport. This function
    /// is used by the server after all checks have passed and should be used
    /// by the client once the server confirms changes were successful.
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

    /// Applies each item in [`actions`] to the given [`vec`]
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

/// Represents all possible options in a query to fetch [`BulkImport`]s. Every
/// possible parameter is an [Option] so all of them aren't absolutely required.
#[derive(Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "server", derive(rocket::FromForm))]
pub struct BulkQuery {
    /// The server should only return [`BulkImport`]s with any of these ids
    pub id: Option<Vec<String>>,

    /// The server should only return [`BulkImport`]s with any of these creators
    pub creator: Option<Vec<String>>,

    /// The maximum number of items to return
    pub limit: Option<u32>,

    /// If the number of items which meet the query params exceeds [`limit`],
    /// start counting from here
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
        // Lets us actively build the query instead of being forced to use it immediately
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
