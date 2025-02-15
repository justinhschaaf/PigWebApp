// @generated automatically by Diesel CLI.

diesel::table! {
    pigs (id) {
        id -> Uuid,
        name -> Text,
        created -> Timestamp,
    }
}
