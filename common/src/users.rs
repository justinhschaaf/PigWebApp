use crate::{query, yuri, DEFAULT_API_RESPONSE_LIMIT, USER_API_ROOT};
use chrono::{NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use std::borrow::ToOwned;
use std::collections::BTreeMap;
use uuid::Uuid;

#[cfg(feature = "server")]
use {crate::schema, diesel::*, diesel_full_text_search::*};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(
    feature = "server",
    derive(diesel::AsChangeset, diesel::Identifiable, diesel::Insertable, diesel::Queryable, diesel::Selectable)
)]
#[cfg_attr(feature = "server", diesel(table_name = crate::schema::users))]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub groups: Vec<String>,
    pub created: NaiveDateTime,
    pub seen: NaiveDateTime,
    pub sso_subject: String,
    pub sso_issuer: String,
    pub session_exp: Option<NaiveDateTime>,
}

impl User {
    pub fn new(
        username: String,
        groups: Vec<String>,
        sso_subject: String,
        sso_issuer: String,
        session_exp: Option<NaiveDateTime>,
    ) -> User {
        let now = Utc::now().naive_utc();
        User { id: Uuid::new_v4(), username, groups, created: now, seen: now, sso_subject, sso_issuer, session_exp }
    }

    pub fn get_system_user() -> User {
        User {
            id: Uuid::default(),
            username: "admin".to_owned(),
            groups: vec![],
            created: NaiveDateTime::default(),
            seen: NaiveDateTime::default(),
            sso_subject: String::default(),
            sso_issuer: "https://self-issued.me".to_owned(),
            session_exp: Some(
                NaiveDate::from_ymd_opt(9999, 12, 31).unwrap_or_default().and_hms_opt(23, 59, 59).unwrap(),
            ),
        }
    }
}

#[derive(Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "server", derive(rocket::FromForm))]
pub struct UserQuery {
    pub id: Option<Vec<String>>,
    pub username: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

impl Default for UserQuery {
    fn default() -> Self {
        Self { id: None, username: None, limit: Some(DEFAULT_API_RESPONSE_LIMIT), offset: Some(0) }
    }
}

impl UserQuery {
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

    pub fn with_username(mut self, username: &String) -> Self {
        self.username = Some(username.to_owned());
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
        yuri!(USER_API_ROOT, "fetch" ;? query!(self))
    }

    /// Converts user query params to DB query
    #[cfg(feature = "server")]
    #[dsl::auto_type(no_type_alias)]
    pub fn to_db_select(&self) -> _ {
        let mut res: helper_types::IntoBoxed<schema::users::table, pg::Pg> = schema::users::table.into_boxed();

        // Filter by name, if specified
        if let Some(ref username) = self.username {
            // This performs a full text search
            // https://www.slingacademy.com/article/implementing-fuzzy-search-with-postgresql-full-text-search/?#implementing-fuzzy-matching-with-fts
            res = res
                .filter(to_tsvector(schema::users::username).matches(plainto_tsquery(username)))
                .or_filter(schema::users::username.ilike(format!("%{}%", username)));
        }

        // Filter by id, if specified
        if let Some(query_ids) = self.id.as_ref().and_then(|ids| crate::parse_uuids(ids).ok()) {
            res = res.filter(schema::users::id.eq_any(query_ids));
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

#[derive(Debug, Serialize, Deserialize)]
pub struct UserFetchResponse {
    pub usernames: Option<BTreeMap<Uuid, String>>,
    pub users: Option<Vec<User>>,
}

impl Default for UserFetchResponse {
    fn default() -> Self {
        Self { usernames: None, users: None }
    }
}

impl UserFetchResponse {
    pub fn with_usernames(mut self, usernames: BTreeMap<Uuid, String>) -> Self {
        self.usernames = Some(usernames);
        self
    }

    pub fn with_users(mut self, users: Vec<User>) -> Self {
        self.users = Some(users);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Roles {
    PigViewer,
    PigEditor,
    BulkEditor,
    BulkAdmin,
    UserViewer,
    UserAdmin,
    LogViewer,
}

impl Roles {
    // https://stackoverflow.com/a/21376984
    pub fn values() -> impl Iterator<Item = Roles> {
        [
            Self::PigViewer,
            Self::PigEditor,
            Self::BulkEditor,
            Self::BulkAdmin,
            Self::UserViewer,
            Self::UserAdmin,
            Self::LogViewer,
        ]
        .iter()
        .copied()
    }
}
