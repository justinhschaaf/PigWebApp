// @generated automatically by Diesel CLI.

diesel::table! {
    pigs (id) {
        id -> Uuid,
        name -> Text,
        created -> Timestamp,
        creator -> Uuid,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        username -> Text,
        groups -> Nullable<Array<Nullable<Text>>>,
        created -> Timestamp,
        seen -> Timestamp,
        sso_subject -> Text,
        sso_issuer -> Text,
        session_exp -> Nullable<Timestamp>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    pigs,
    users,
);
