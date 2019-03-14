table! {
    posts (id) {
        id -> Int4,
        uuid -> Uuid,
        title -> Varchar,
        url -> Nullable<Varchar>,
        visible -> Bool,
        score -> Int4,
        created_at -> Timestamp,
        submitted_by -> Int4,
    }
}

table! {
    stars (user_id, post_id) {
        user_id -> Int4,
        post_id -> Int4,
        created_at -> Timestamp,
    }
}

table! {
    users (id) {
        id -> Int4,
        banned -> Bool,
        trust_level -> Int4,
        username -> Varchar,
        password_hash -> Bytea,
        created_at -> Timestamp,
    }
}

joinable!(posts -> users (submitted_by));
joinable!(stars -> posts (post_id));
joinable!(stars -> users (user_id));

allow_tables_to_appear_in_same_query!(
    posts,
    stars,
    users,
);
