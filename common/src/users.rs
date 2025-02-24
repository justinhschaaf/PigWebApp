use chrono::{NaiveDate, NaiveDateTime};
use std::borrow::ToOwned;
use uuid::Uuid;

pub const SYSTEM_USER: User = User {
    id: Uuid::default(),
    username: "admin".to_owned(),
    groups: vec![],
    created: NaiveDateTime::default(),
    seen: NaiveDateTime::default(),
    sso_subject: String::default(),
    sso_issuer: "https://self-issued.me".to_owned(),
    session_exp: Some(NaiveDate::from_ymd_opt(9999, 12, 31).unwrap_or_default().and_hms_opt(23, 59, 59).unwrap()),
};

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
