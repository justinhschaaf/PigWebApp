// @generated automatically by Diesel CLI.

diesel::table! {
    bulk_imports (id) {
        id -> Uuid,
        name -> Text,
        creator -> Uuid,
        started -> Timestamp,
        finished -> Nullable<Timestamp>,
        pending -> Array<Text>,
        accepted -> Array<Uuid>,
        rejected -> Array<Text>,
    }
}

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
        groups -> Array<Text>,
        created -> Timestamp,
        seen -> Timestamp,
        sso_subject -> Text,
        sso_issuer -> Text,
        session_exp -> Nullable<Timestamp>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(bulk_imports, pigs, users);
