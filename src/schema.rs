table! {
    use crate::sql_types::*;

    blocked_regexes (id) {
        id -> Int4,
        regex -> Varchar,
    }
}

table! {
    use crate::sql_types::*;

    comment_flags (user_id, comment_id) {
        user_id -> Int4,
        comment_id -> Int4,
        created_at -> Timestamp,
    }
}

table! {
    use crate::sql_types::*;

    comment_hides (comment_id, user_id) {
        comment_id -> Int4,
        user_id -> Int4,
    }
}

table! {
    use crate::sql_types::*;

    comment_readpoints (user_id, post_id) {
        user_id -> Int4,
        post_id -> Int4,
        comment_readpoint -> Int4,
    }
}

table! {
    use crate::sql_types::*;

    comment_stars (user_id, comment_id) {
        user_id -> Int4,
        comment_id -> Int4,
        created_at -> Timestamp,
    }
}

table! {
    use crate::sql_types::*;

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
    use crate::sql_types::*;

    domain_restrictions (domain_id) {
        domain_id -> Int4,
        restriction_level -> Int4,
    }
}

table! {
    use crate::sql_types::*;

    domain_synonyms (from_hostname) {
        from_hostname -> Varchar,
        to_domain_id -> Int4,
    }
}

table! {
    use crate::sql_types::*;

    domains (id) {
        id -> Int4,
        banned -> Bool,
        hostname -> Varchar,
        is_www -> Bool,
        is_https -> Bool,
    }
}

table! {
    use crate::sql_types::*;

    flags (user_id, post_id) {
        user_id -> Int4,
        post_id -> Int4,
        created_at -> Timestamp,
    }
}

table! {
    use crate::sql_types::*;

    invite_tokens (uuid) {
        uuid -> Int8,
        created_at -> Timestamp,
        invited_by -> Int4,
    }
}

table! {
    use crate::sql_types::*;

    legacy_comments (id) {
        id -> Int4,
        post_id -> Int4,
        author -> Varchar,
        text -> Varchar,
        html -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    use crate::sql_types::*;

    moderation (id) {
        id -> Int4,
        payload -> Jsonb,
        created_at -> Timestamp,
        created_by -> Int4,
    }
}

table! {
    use crate::sql_types::*;

    notifications (id) {
        id -> Int4,
        user_id -> Int4,
        post_id -> Int4,
        created_at -> Timestamp,
        created_by -> Int4,
    }
}

table! {
    use crate::sql_types::*;

    poll_choices (id) {
        id -> Int4,
        poll_id -> Int4,
        title -> Text,
        created_at -> Timestamp,
        created_by -> Int4,
    }
}

table! {
    use crate::sql_types::*;

    poll_votes (id) {
        id -> Int4,
        user_id -> Int4,
        choice_id -> Int4,
        score -> Int4,
        created_at -> Timestamp,
    }
}

table! {
    use crate::sql_types::*;

    polls (id) {
        id -> Int4,
        post_id -> Int4,
        title -> Text,
        open -> Bool,
        created_at -> Timestamp,
        created_by -> Int4,
    }
}

table! {
    use crate::sql_types::*;

    post_hides (post_id, user_id) {
        post_id -> Int4,
        user_id -> Int4,
    }
}

table! {
    use crate::sql_types::*;

    post_search_index (post_id) {
        post_id -> Int4,
        search_index -> Tsvector,
    }
}

table! {
    use crate::sql_types::*;

    post_tagging (post_id, tag_id) {
        post_id -> Int4,
        tag_id -> Int4,
    }
}

table! {
    use crate::sql_types::*;

    post_word_freq (word) {
        word -> Varchar,
        num -> Nullable<Int4>,
    }
}

table! {
    use crate::sql_types::*;

    posts (id) {
        id -> Int4,
        uuid -> Int8,
        title -> Varchar,
        url -> Nullable<Varchar>,
        visible -> Bool,
        initial_stellar_time -> Int4,
        score -> Int4,
        comment_count -> Int4,
        created_at -> Timestamp,
        submitted_by -> Int4,
        excerpt -> Nullable<Varchar>,
        excerpt_html -> Nullable<Varchar>,
        updated_at -> Timestamp,
        rejected -> Bool,
        domain_id -> Nullable<Int4>,
        banner_title -> Nullable<Varchar>,
        banner_desc -> Nullable<Varchar>,
        private -> Bool,
        title_html -> Nullable<Varchar>,
        blog_post -> Bool,
        noindex -> Bool,
        locked -> Bool,
        anon -> Bool,
    }
}

table! {
    use crate::sql_types::*;

    site_customization (name) {
        name -> Varchar,
        value -> Varchar,
    }
}

table! {
    use crate::sql_types::*;

    stars (user_id, post_id) {
        user_id -> Int4,
        post_id -> Int4,
        created_at -> Timestamp,
    }
}

table! {
    use crate::sql_types::*;

    subscriptions (id) {
        id -> Int4,
        user_id -> Int4,
        post_id -> Int4,
        created_at -> Timestamp,
        created_by -> Int4,
    }
}

table! {
    use crate::sql_types::*;

    tags (id) {
        id -> Int4,
        name -> Varchar,
        description -> Nullable<Varchar>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    use crate::sql_types::*;

    user_sessions (uuid) {
        uuid -> Int8,
        created_at -> Timestamp,
        user_agent -> Text,
        user_id -> Int4,
        last_seen_at -> Timestamp,
    }
}

table! {
    use crate::sql_types::*;

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
        identicon -> Int4,
    }
}

joinable!(comment_flags -> comments (comment_id));
joinable!(comment_flags -> users (user_id));
joinable!(comment_hides -> comments (comment_id));
joinable!(comment_hides -> users (user_id));
joinable!(comment_readpoints -> posts (post_id));
joinable!(comment_readpoints -> users (user_id));
joinable!(comment_stars -> comments (comment_id));
joinable!(comment_stars -> users (user_id));
joinable!(comments -> posts (post_id));
joinable!(comments -> users (created_by));
joinable!(domain_restrictions -> domains (domain_id));
joinable!(domain_synonyms -> domains (to_domain_id));
joinable!(flags -> posts (post_id));
joinable!(flags -> users (user_id));
joinable!(invite_tokens -> users (invited_by));
joinable!(legacy_comments -> posts (post_id));
joinable!(moderation -> users (created_by));
joinable!(notifications -> posts (post_id));
joinable!(poll_choices -> polls (poll_id));
joinable!(poll_choices -> users (created_by));
joinable!(poll_votes -> poll_choices (choice_id));
joinable!(poll_votes -> users (user_id));
joinable!(polls -> posts (post_id));
joinable!(polls -> users (created_by));
joinable!(post_hides -> posts (post_id));
joinable!(post_hides -> users (user_id));
joinable!(post_search_index -> posts (post_id));
joinable!(post_tagging -> posts (post_id));
joinable!(post_tagging -> tags (tag_id));
joinable!(posts -> domains (domain_id));
joinable!(posts -> users (submitted_by));
joinable!(stars -> posts (post_id));
joinable!(stars -> users (user_id));
joinable!(subscriptions -> posts (post_id));
joinable!(user_sessions -> users (user_id));

allow_tables_to_appear_in_same_query!(
    blocked_regexes,
    comment_flags,
    comment_hides,
    comment_readpoints,
    comment_stars,
    comments,
    domain_restrictions,
    domain_synonyms,
    domains,
    flags,
    invite_tokens,
    legacy_comments,
    moderation,
    notifications,
    poll_choices,
    poll_votes,
    polls,
    post_hides,
    post_search_index,
    post_tagging,
    post_word_freq,
    posts,
    site_customization,
    stars,
    subscriptions,
    tags,
    user_sessions,
    users,
);
