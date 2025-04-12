use chrono::{NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use std::borrow::ToOwned;
use uuid::Uuid;

#[cfg(feature = "server")]
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(AsChangeset, Identifiable, Insertable, Queryable, Selectable))]
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
