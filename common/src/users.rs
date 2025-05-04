use crate::{query_limit_offset, query_list, query_to_yuri, DEFAULT_API_RESPONSE_LIMIT, USER_API_ROOT};
use chrono::{NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use std::borrow::ToOwned;
use std::collections::BTreeMap;
use uuid::Uuid;

#[cfg(feature = "server")]
use {crate::schema, diesel::*, diesel_full_text_search::*};

/// A user. This is usually created upon first signing in with OIDC SSO.
///
/// While the app uses the [`id`] to uniquely identify users internally, upon
/// signing in with OIDC, only the [subject](sso_subject) and
/// [issuer](sso_issuer) can be used to uniquely identify a user (as per the
/// [spec](https://openid.net/specs/openid-connect-core-1_0.html#ClaimStability)).
///
/// As such, all fields besides these three are automatically updated to match
/// the OIDC provider upon login.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(
    feature = "server",
    derive(diesel::AsChangeset, diesel::Identifiable, diesel::Insertable, diesel::Queryable, diesel::Selectable)
)]
#[cfg_attr(feature = "server", diesel(table_name = crate::schema::users))]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct User {
    /// The unique id for this user
    pub id: Uuid,

    /// The name of this user
    pub username: String,

    /// A list of groups this user is a member of, as determined by the OIDC
    /// JWT. Groups recognized by the app can be set in the config.
    pub groups: Vec<String>,

    /// When this user first signed in to the app
    pub created: NaiveDateTime,

    /// The last time the user signed in to the app
    pub seen: NaiveDateTime,

    /// The subject identifier received from the OIDC provider (`sub` field from
    /// the JWT [ID Token](https://openid.net/specs/openid-connect-core-1_0.html#IDToken))
    pub sso_subject: String,

    /// The OIDC provider which issued the response to the server (`iss` field
    /// from the JWT [ID Token](https://openid.net/specs/openid-connect-core-1_0.html#IDToken))
    pub sso_issuer: String,

    /// When the user's current session will expire. The session should be
    /// considered expired if this is [`None`] or the timestamp is in the past.
    pub session_exp: Option<NaiveDateTime>,
}

impl User {
    /// Creates a new User from the given values with a random [`Uuid`] and the
    /// current time as [`created`].
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

    /// When OIDC and groups aren't properly setup, this returns a generic user
    /// to represent the performer of all actions instead. This isn't really
    /// tested, so setup OIDC!!!!!
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
    /// The server should only return [`User`]s with any of these ids
    pub id: Option<Vec<String>>,

    /// Performs a full-text search to only return [`User`]s with a similar name
    pub username: Option<String>,

    /// The maximum number of items to return
    pub limit: Option<u32>,

    /// If the number of items which meet the query params exceeds [`limit`],
    /// start counting from here
    pub offset: Option<u32>,
}

impl Default for UserQuery {
    fn default() -> Self {
        Self { id: None, username: None, limit: Some(DEFAULT_API_RESPONSE_LIMIT), offset: Some(0) }
    }
}

impl UserQuery {
    query_list!(id, Uuid);
    query_limit_offset!();
    query_to_yuri!(USER_API_ROOT);

    /// Filters the results to [`User`]s with a name similar to the given String
    pub fn with_username(mut self, username: &String) -> Self {
        self.username = Some(username.to_owned());
        self
    }

    /// Converts query params to DB query
    #[cfg(feature = "server")]
    #[dsl::auto_type(no_type_alias)]
    pub fn to_db_select(&self) -> _ {
        // Lets us actively build the query instead of being forced to use it immediately
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

/// A response to a user fetch request. If the requester has
/// [`Roles::UserViewer`], they will be sent the full data for each user.
/// Otherwise, only a mapping of ids to usernames will be returned.
#[derive(Debug, Serialize, Deserialize)]
pub struct UserFetchResponse {
    /// A mapping of ids to usernames containing each user who matches the query
    pub usernames: Option<BTreeMap<Uuid, String>>,

    /// A list of all users who match the query
    pub users: Option<Vec<User>>,
}

impl Default for UserFetchResponse {
    fn default() -> Self {
        Self { usernames: None, users: None }
    }
}

impl UserFetchResponse {
    /// Sets this response's mapping of ids to usernames to the given map.
    ///
    /// ***This overrides any previously provided data.***
    pub fn with_usernames(mut self, usernames: BTreeMap<Uuid, String>) -> Self {
        self.usernames = Some(usernames);
        self
    }

    /// Sets this response's list of users to the given Vec.
    ///
    /// ***This overrides any previously provided data.***
    pub fn with_users(mut self, users: Vec<User>) -> Self {
        self.users = Some(users);
        self
    }
}

/// Each action a user is allowed to take. The groups assigned to [`User`]s
/// directly are simply a list of roles which they grant the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Roles {
    /// Lets a user view the pig list
    PigViewer,

    /// Lets a user edit the pig list (create, update, delete pigs)
    PigEditor,

    /// Lets a user create and process [`crate::bulk::BulkImport`]s
    BulkEditor,

    /// Lets a user view and edit [`crate::bulk::BulkImport`]s created by other
    /// users
    BulkAdmin,

    /// Lets a user view detailed data on all other [`User`]s
    UserViewer,

    /// Lets a user invalidate user sessions
    UserAdmin,

    /// Lets a user view the audit log
    LogViewer,
}

impl Roles {
    /// Creates an iterator over all values in this enum
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
