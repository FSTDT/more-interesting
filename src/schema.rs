table! {
    invite_tokens (uuid) {
        uuid -> Uuid,
        created_at -> Timestamp,
        invited_by -> Int4,
    }
}

table! {
    posts (id) {
        id -> Int4,
        uuid -> Uuid,
        title -> Varchar,
        url -> Nullable<Varchar>,
        visible -> Bool,
        initial_stellar_time -> Int4,
        score -> Int4,
        reply_count -> Int4,
        authored_by_submitter -> Bool,
        created_at -> Timestamp,
        submitted_by -> Int4,
    }
}

table! {
    replies (id) {
        id -> Int4,
        text -> Varchar,
        html -> Varchar,
        visible -> Bool,
        post_id -> Int4,
        created_at -> Timestamp,
        created_by -> Int4,
    }
}

table! {
    reply_stars (user_id, reply_id) {
        user_id -> Int4,
        reply_id -> Int4,
        created_at -> Timestamp,
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
        invited_by -> Nullable<Int4>,
    }
}

joinable!(invite_tokens -> users (invited_by));
joinable!(posts -> users (submitted_by));
joinable!(replies -> posts (post_id));
joinable!(replies -> users (created_by));
joinable!(reply_stars -> replies (reply_id));
joinable!(reply_stars -> users (user_id));
joinable!(stars -> posts (post_id));
joinable!(stars -> users (user_id));

allow_tables_to_appear_in_same_query!(
    invite_tokens,
    posts,
    replies,
    reply_stars,
    stars,
    users,
);
