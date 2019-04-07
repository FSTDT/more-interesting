table! {
    comment_flags (user_id, comment_id) {
        user_id -> Int4,
        comment_id -> Int4,
        created_at -> Timestamp,
    }
}

table! {
    comment_stars (user_id, comment_id) {
        user_id -> Int4,
        comment_id -> Int4,
        created_at -> Timestamp,
    }
}

table! {
    comments (id) {
        id -> Int4,
        text -> Varchar,
        html -> Varchar,
        visible -> Bool,
        post_id -> Int4,
        created_at -> Timestamp,
        created_by -> Int4,
        updated_at -> Timestamp,
        rejected -> Bool,
    }
}

table! {
    flags (user_id, post_id) {
        user_id -> Int4,
        post_id -> Int4,
        created_at -> Timestamp,
    }
}

table! {
    invite_tokens (uuid) {
        uuid -> Int8,
        created_at -> Timestamp,
        invited_by -> Int4,
    }
}

table! {
    moderation (id) {
        id -> Int4,
        payload -> Jsonb,
        created_at -> Timestamp,
        created_by -> Int4,
    }
}

table! {
    post_tagging (post_id, tag_id) {
        post_id -> Int4,
        tag_id -> Int4,
    }
}

table! {
    posts (id) {
        id -> Int4,
        uuid -> Int8,
        title -> Varchar,
        url -> Nullable<Varchar>,
        visible -> Bool,
        initial_stellar_time -> Int4,
        score -> Int4,
        comment_count -> Int4,
        authored_by_submitter -> Bool,
        created_at -> Timestamp,
        submitted_by -> Int4,
        excerpt -> Nullable<Varchar>,
        excerpt_html -> Nullable<Varchar>,
        updated_at -> Timestamp,
        rejected -> Bool,
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
    tags (id) {
        id -> Int4,
        name -> Varchar,
        description -> Nullable<Varchar>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
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
        dark_mode -> Bool,
        big_mode -> Bool,
    }
}

joinable!(comment_flags -> comments (comment_id));
joinable!(comment_flags -> users (user_id));
joinable!(comment_stars -> comments (comment_id));
joinable!(comment_stars -> users (user_id));
joinable!(comments -> posts (post_id));
joinable!(comments -> users (created_by));
joinable!(flags -> posts (post_id));
joinable!(flags -> users (user_id));
joinable!(invite_tokens -> users (invited_by));
joinable!(moderation -> users (created_by));
joinable!(post_tagging -> posts (post_id));
joinable!(post_tagging -> tags (tag_id));
joinable!(posts -> users (submitted_by));
joinable!(stars -> posts (post_id));
joinable!(stars -> users (user_id));

allow_tables_to_appear_in_same_query!(
    comment_flags,
    comment_stars,
    comments,
    flags,
    invite_tokens,
    moderation,
    post_tagging,
    posts,
    stars,
    tags,
    users,
);
