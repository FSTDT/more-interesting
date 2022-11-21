use bigdecimal::{BigDecimal, ToPrimitive};
use rocket_sync_db_pools::diesel::PgConnection;
use diesel::prelude::*;
use diesel::sql_types;
use diesel::result::Error as DieselError;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Utc, Duration};
use crate::schema::{site_customization, users, user_sessions, posts, stars, invite_tokens, comments, comment_stars, tags, post_tagging, moderation, flags, comment_flags, domains, legacy_comments, domain_synonyms, notifications, subscriptions, post_hides, comment_hides, post_word_freq, comment_readpoints, domain_restrictions, polls, poll_votes, poll_choices, blocked_regexes};
use crate::password::{password_hash, password_verify, PasswordResult};
use serde::{Deserialize, Serialize};
use more_interesting_base32::Base32;
use std::cmp::max;
use std::cmp::Ordering;
use std::mem;
use ordered_float::OrderedFloat;
use std::collections::{HashMap, HashSet};
use crate::prettify::{self, prettify_title};
use serde_json::{self as json, json};
use url::Url;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use std::convert::TryInto;

sql_function!(fn coalesce(x: sql_types::Nullable<sql_types::VarChar>, y: sql_types::VarChar) -> sql_types::VarChar);
no_arg_sql_function!(random, sql_types::BigInt, "Random number");

const FLAG_INVISIBLE_THRESHOLD: i64 = 3;

#[derive(Debug)]
pub enum CreateCommentError {
    DieselError(DieselError),
    TooManyComments,
}

impl From<DieselError> for CreateCommentError {
    fn from(e: DieselError) -> CreateCommentError {
        CreateCommentError::DieselError(e)
    }
}

#[derive(Debug)]
pub enum CreatePostError {
    DieselError(DieselError),
    RequireTag,
    TooManyPosts,
    TooManyPostsDomain,
    TooManyPostsDomainUser,
    TooLong
}

impl From<DieselError> for CreatePostError {
    fn from(e: DieselError) -> CreatePostError {
        CreatePostError::DieselError(e)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BodyFormat {
    #[serde(alias = "plain")]
    Plain,
    #[serde(alias = "bbcode")]
    BBCode,
}

impl Default for BodyFormat {
    fn default() -> BodyFormat {
        BodyFormat::Plain
    }
}

#[derive(Queryable, Serialize)]
pub struct Moderation {
    pub id: i32,
    pub payload: json::Value,
    pub created_at: NaiveDateTime,
    pub created_by: i32,
}

#[derive(Serialize)]
pub struct ModerationInfo {
    pub id: i32,
    pub payload: json::Value,
    pub created_at: NaiveDateTime,
    pub created_by: i32,
    pub created_by_username: String,
}

#[derive(Queryable, QueryableByName, Serialize)]
#[table_name="polls"]
pub struct Poll {
    pub id: i32,
    pub post_id: i32,
    pub title: String,
    pub open: bool,
    pub created_at: NaiveDateTime,
    pub created_by: i32,
}

#[derive(Queryable, QueryableByName, Serialize)]
#[table_name="poll_choices"]
pub struct PollChoice {
    pub id: i32,
    pub poll_id: i32,
    pub title: String,
    pub created_at: NaiveDateTime,
    pub created_by: i32,
}

#[derive(Queryable, QueryableByName, Serialize)]
#[table_name="poll_votes"]
pub struct PollVote {
    pub id: i32,
    pub user_id: i32,
    pub choice_id: i32,
    pub score: i32,
    pub created_at: NaiveDateTime,
}

#[derive(Serialize)]
pub struct PollInfo {
    pub title: String,
    pub poll_id: i32,
    pub open: bool,
    pub choices: Vec<PollInfoChoice>,
}

#[derive(Serialize)]
pub struct PollInfoChoice {
    pub choice_id: i32,
    pub title: String,
    pub score: i32,
    pub count: i32,
    pub average: f64,
}

#[derive(Queryable, QueryableByName, Serialize)]
#[table_name="posts"]
pub struct Post {
    pub id: i32,
    pub uuid: Base32,
    pub title: String,
    pub url: Option<String>,
    pub visible: bool,
    pub initial_stellar_time: i32,
    pub score: i32,
    pub comment_count: i32,
    pub created_at: NaiveDateTime,
    pub submitted_by: i32,
    pub excerpt: Option<String>,
    pub excerpt_html: Option<String>,
    pub updated_at: NaiveDateTime,
    pub rejected: bool,
    pub domain_id: Option<i32>,
    pub banner_title: Option<String>,
    pub banner_desc: Option<String>,
    pub private: bool,
    pub title_html: Option<String>,
    pub blog_post: bool,
    pub noindex: bool,
    pub locked: bool,
    pub anon: bool,
}

#[derive(Clone, Queryable, Serialize)]
pub struct UserSession {
    pub uuid: Base32,
    pub created_at: NaiveDateTime,
    pub user_agent: String,
    pub user_id: i32,
    pub last_seen_at: NaiveDateTime,
}

#[derive(Clone, Queryable, Serialize)]
pub struct User {
    pub id: i32,
    pub banned: bool,
    pub trust_level: i32,
    pub username: String,
    pub password_hash: Vec<u8>,
    pub created_at: NaiveDateTime,
    pub invited_by: Option<i32>,
    pub dark_mode: bool,
    pub big_mode: bool,
    pub identicon: i32,
}

#[derive(Queryable, Serialize)]
pub struct LegacyComment {
    pub id: i32,
    pub post_id: i32,
    pub author: String,
    pub text: String,
    pub html: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Serialize)]
pub struct LegacyCommentInfo {
    pub id: i32,
    pub post_id: i32,
    pub author: String,
    pub text: String,
    pub html: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub created_at_relative: String,
}

impl Default for User {
    fn default() -> Self {
        User {
            id: 0,
            banned: false,
            trust_level: -2,
            username: "".to_string(),
            password_hash: vec![],
            created_at: NaiveDateTime::from_timestamp(0, 0),
            invited_by: None,
            dark_mode: false,
            big_mode: false,
            identicon: 0,
        }
    }
}

impl Default for UserSession {
    fn default() -> Self {
        UserSession {
            uuid: Base32::zero(),
            user_id: 0,
            user_agent: String::new(),
            created_at: NaiveDateTime::from_timestamp(0, 0),
            last_seen_at: NaiveDateTime::from_timestamp(0, 0),
        }
    }
}

#[derive(Insertable, Queryable, QueryableByName, Serialize)]
#[table_name="site_customization"]
pub struct SiteCustomization {
    pub name: String,
    pub value: String,
}

pub struct NewPost {
    pub title: String,
    pub url: Option<String>,
    pub excerpt: Option<String>,
    pub submitted_by: i32,
    pub visible: bool,
    pub private: bool,
    pub blog_post: bool,
    pub anon: bool,
}

#[derive(Serialize)]
pub struct PostInfo {
    pub id: i32,
    pub uuid: Base32,
    pub title: String,
    pub title_html: String,
    pub url: Option<String>,
    pub visible: bool,
    pub private: bool,
    pub hotness: f64,
    pub score: i32,
    pub comment_count: i32,
    pub blog_post: bool,
    pub created_at: NaiveDateTime,
    pub created_at_relative: String,
    pub submitted_by: i32,
    pub submitted_by_username: String,
    pub submitted_by_username_urlencode: String,
    pub starred_by_me: bool,
    pub flagged_by_me: bool,
    pub hidden_by_me: bool,
    pub comment_readpoint: Option<i32>,
    pub excerpt_html: Option<String>,
    pub banner_title: Option<String>,
    pub banner_desc: Option<String>,
    pub noindex: bool,
    pub locked: bool,
    pub anon: bool,
}

#[derive(Serialize)]
pub struct NotificationInfo {
    pub post_uuid: Base32,
    pub post_title: String,
    pub comment_count: i32,
    pub from_username: String,
}

#[derive(Queryable, Serialize)]
pub struct Domain {
    pub id: i32,
    pub banned: bool,
    pub hostname: String,
    pub is_www: bool,
    pub is_https: bool,
}

#[derive(Queryable, Serialize)]
pub struct BlockedRegex {
    pub id: i32,
    pub regex: String,
}

#[derive(Insertable)]
#[table_name="blocked_regexes"]
pub struct NewBlockedRegex {
    pub regex: String,
}

#[derive(Insertable)]
#[table_name="domains"]
pub struct NewDomain {
    pub hostname: String,
    pub is_www: bool,
    pub is_https: bool,
}

#[derive(Insertable, Queryable, Serialize)]
#[table_name="domain_synonyms"]
pub struct DomainSynonym {
    pub from_hostname: String,
    pub to_domain_id: i32,
}

#[derive(Insertable, Queryable, Serialize)]
#[table_name="domain_restrictions"]
pub struct DomainRestriction {
    pub domain_id: i32,
    pub restriction_level: i32,
}

#[derive(Queryable, Serialize)]
pub struct DomainSynonymInfo {
    pub from_hostname: String,
    pub to_hostname: String,
}

#[derive(Queryable, Serialize)]
pub struct Comment {
    pub id: i32,
    pub text: String,
    pub html: String,
    pub visible: bool,
    pub post_id: i32,
    pub created_at: NaiveDateTime,
    pub created_by: i32,
    pub updated_at: NaiveDateTime,
    pub rejected: bool,
}

#[derive(Clone)]
pub struct NewComment {
    pub post_id: i32,
    pub text: String,
    pub created_by: i32,
    pub visible: bool,
}

#[derive(Serialize)]
pub struct CommentInfo {
    pub id: i32,
    pub text: String,
    pub html: String,
    pub visible: bool,
    pub post_id: i32,
    pub created_at: NaiveDateTime,
    pub created_at_relative: String,
    pub created_by: i32,
    pub created_by_username: String,
    pub created_by_username_urlencode: String,
    pub created_by_identicon: Base32,
    pub starred_by_me: bool,
    pub flagged_by_me: bool,
    pub hidden_by_me: bool,
    pub starred_by: Vec<String>,
}

#[derive(Serialize)]
pub struct CommentSearchResult {
    pub id: i32,
    pub html: String,
    pub post_id: i32,
    pub post_uuid: Base32,
    pub post_title: String,
    pub post_locked: bool,
    pub created_at: NaiveDateTime,
    pub created_at_relative: String,
    pub created_by: i32,
    pub created_by_username: String,
    pub created_by_identicon: Base32,
    pub starred_by_me: bool,
    pub flagged_by_me: bool,
    pub starred_by: Vec<String>,
}

#[derive(Queryable)]
pub struct InviteToken {
    pub uuid: Base32,
    pub created_at: NaiveDateTime,
    pub invited_by: i32,
}

#[derive(Clone, Insertable)]
#[table_name="stars"]
pub struct NewStar {
    pub user_id: i32,
    pub post_id: i32,
}

#[derive(Clone, Insertable)]
#[table_name="flags"]
pub struct NewFlag {
    pub user_id: i32,
    pub post_id: i32,
}

#[derive(Clone, Insertable)]
#[table_name="post_hides"]
pub struct NewHide {
    pub user_id: i32,
    pub post_id: i32,
}

#[derive(Clone, Insertable)]
#[table_name="comment_stars"]
pub struct NewStarComment {
    pub user_id: i32,
    pub comment_id: i32,
}

#[derive(Clone, Insertable)]
#[table_name="comment_flags"]
pub struct NewFlagComment {
    pub user_id: i32,
    pub comment_id: i32,
}

#[derive(Clone, Insertable)]
#[table_name="comment_hides"]
pub struct NewHideComment {
    pub user_id: i32,
    pub comment_id: i32,
}

#[derive(Clone)]
pub struct UserAuth {
    pub username: String,
    pub password: String,
}

#[derive(Clone)]
pub struct NewUser {
    pub username: String,
    pub password: String,
    pub invited_by: Option<i32>,
}

#[derive(Clone)]
pub struct NewTag {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Queryable, Serialize)]
pub struct Tag {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[table_name="post_tagging"]
struct CreatePostTagging {
    post_id: i32,
    tag_id: i32,
}

#[derive(Insertable)]
#[table_name="moderation"]
struct CreateModeration {
    pub payload: json::Value,
    pub created_by: i32,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum PostSearchOrderBy {
    Hottest,
    Newest,
    Latest,
    Top,
    Random,
}

#[derive(Insertable, Queryable, QueryableByName, Serialize)]
#[table_name="comment_readpoints"]
pub struct ReadPoint {
    pub user_id: i32,
    pub post_id: i32,
    pub comment_readpoint: i32,
}

#[derive(Queryable, QueryableByName, Serialize)]
#[table_name="notifications"]
pub struct Notification {
    pub id: i32,
    pub user_id: i32,
    pub post_id: i32,
    pub created_at: NaiveDateTime,
    pub created_by: i32,
}

#[derive(Insertable)]
#[table_name="notifications"]
pub struct NewNotification {
    pub user_id: i32,
    pub post_id: i32,
    pub created_by: i32,
}

#[derive(Queryable, QueryableByName, Serialize)]
#[table_name="subscriptions"]
pub struct Subscription {
    pub id: i32,
    pub user_id: i32,
    pub post_id: i32,
    pub created_at: NaiveDateTime,
    pub created_by: i32,
}

#[derive(Clone, Insertable)]
#[table_name="subscriptions"]
pub struct NewSubscription {
    pub user_id: i32,
    pub post_id: i32,
    pub created_by: i32,
}

#[derive(Clone)]
pub struct PostSearch {
    pub my_user_id: i32,
    pub for_user_id: i32,
    pub order_by: PostSearchOrderBy,
    pub keywords: String,
    pub title: String,
    pub or_tags: Vec<i32>,
    pub and_tags: Vec<i32>,
    pub hide_tags: Vec<i32>,
    pub or_domains: Vec<i32>,
    pub after_post_id: i32,
    pub search_page: i32,
    pub subscriptions: bool,
    pub blog_post: Option<bool>,
    pub before_date: Option<NaiveDate>,
    pub after_date: Option<NaiveDate>,
    pub limit: i32,
}

#[derive(Queryable, Serialize)]
pub struct PostFlagInfo {
    pub uuid: Base32,
    pub title: String,
    pub created_by_username: String,
}

#[derive(Queryable, Serialize)]
pub struct CommentFlagInfo {
    pub uuid: Base32,
    pub id: i32,
    pub title: String,
    pub created_by_username: String,
}

impl PostSearch {
    pub fn with_my_user_id(my_user_id: i32) -> PostSearch {
        PostSearch {
            my_user_id,
            for_user_id: 0,
            order_by: PostSearchOrderBy::Hottest,
            keywords: String::new(),
            title: String::new(),
            or_tags: Vec::new(),
            and_tags: Vec::new(),
            hide_tags: Vec::new(),
            or_domains: Vec::new(),
            after_post_id: 0,
            search_page: 0,
            subscriptions: false,
            blog_post: None,
            before_date: None,
            after_date: None,
            limit: 50,
        }
    }
}

#[database("more_interesting")]
pub struct MoreInterestingConn(PgConnection);

impl MoreInterestingConn {
    pub async fn prettify_title(&self, post_id: i32, title: &str, url: &str, blog_post: bool) -> String {
        let title = title.to_owned();
        let url = url.to_owned();
        self.run(move |conn| {
            let mut data = PrettifyData::new(conn, post_id);
            crate::prettify::prettify_title(&title, &url, &mut data, blog_post).string
        }).await
    }
    pub async fn prettify_body(&self, post_id: i32, excerpt: &str, body_format: BodyFormat) -> String {
        let excerpt = excerpt.to_owned();
        self.run(move |conn| {
            let mut data = PrettifyData::new(conn, post_id);
            let body = match body_format {
                BodyFormat::Plain => crate::prettify::prettify_body(&excerpt, &mut data),
                BodyFormat::BBCode => crate::prettify::prettify_body_bbcode(&excerpt, &mut data),
            };
            body.string
        }).await
    }
    pub async fn get_recent_post_flags(&self) -> Vec<PostFlagInfo> {
        self.run(move |conn| Self::get_recent_post_flags_(conn)).await
    }
    fn get_recent_post_flags_(conn: &PgConnection) -> Vec<PostFlagInfo> {
        use self::flags::dsl as f;
        use self::posts::dsl::{*, self as p};
        use self::users::dsl::{*, self as u};
        let all: Vec<PostFlagInfo> = f::flags
            .inner_join(users.on(u::id.eq(f::user_id)))
            .inner_join(posts)
            .select((
                p::uuid,
                p::title,
                u::username,
            ))
            .filter(p::visible.eq(true))
            .order_by(f::created_at.desc())
            .limit(200)
            .get_results::<PostFlagInfo>(conn)
            .unwrap_or(Vec::new())
            .into_iter()
            .collect();
        all
    }
    pub async fn get_recent_comment_flags(&self) -> Vec<CommentFlagInfo> {
        self.run(|conn| Self::get_recent_comment_flags_(conn)).await
    }
    fn get_recent_comment_flags_(conn: &PgConnection) -> Vec<CommentFlagInfo> {
        use self::comment_flags::dsl as f;
        use self::posts::dsl::{*, self as p};
        use self::comments::dsl::{*, self as c};
        use self::users::dsl::{*, self as u};
        let all: Vec<CommentFlagInfo> = f::comment_flags
            .inner_join(users.on(u::id.eq(f::user_id)))
            .inner_join(comments)
            .inner_join(posts.on(p::id.eq(c::post_id)))
            .select((
                p::uuid,
                c::id,
                p::title,
                u::username,
            ))
            .filter(c::visible.eq(true))
            .order_by(f::created_at.desc())
            .limit(200)
            .get_results::<CommentFlagInfo>(conn)
            .unwrap_or(Vec::new())
            .into_iter()
            .collect();
        all
    }
    pub async fn set_customization(&self, new: SiteCustomization) -> Result<(), DieselError> {
        self.run(|conn| Self::set_customization_(conn, new)).await
    }
    fn set_customization_(conn: &PgConnection, new: SiteCustomization) -> Result<(), DieselError> {
        use self::site_customization::dsl::*;
        let affected_rows = if Self::get_customization_value_(conn, &new.name).is_some() {
            diesel::update(site_customization.find(&new.name))
                .set(value.eq(&new.value))
                .execute(conn)?
        } else {
            diesel::insert_into(site_customization)
                .values(new)
                .execute(conn)?
        };
        assert_eq!(affected_rows, 1);
        Ok(())
    }
    pub async fn get_customizations(&self) -> Result<Vec<SiteCustomization>, DieselError> {
        self.run(move |conn| Self::get_customizations_(conn)).await
    }
    fn get_customizations_(conn: &PgConnection) -> Result<Vec<SiteCustomization>, DieselError> {
        use self::site_customization::dsl::*;
        site_customization.get_results::<SiteCustomization>(conn)
    }
    pub async fn get_customization_value(&self, name_param: &str) -> Option<String> {
        let name_param = name_param.to_owned();
        self.run(move |conn| Self::get_customization_value_(conn, &name_param)).await
    }
    fn get_customization_value_(conn: &PgConnection, name_param: &str) -> Option<String> {
        use self::site_customization::dsl::*;
        site_customization
            .select(value)
            .filter(name.eq(name_param))
            .get_result::<String>(conn)
            .ok()
    }
    pub async fn create_notification(&self, new: NewNotification) -> Result<(), DieselError> {
        self.run(move |conn| Self::create_notification_(conn, new)).await
    }
    fn create_notification_(conn: &PgConnection, new: NewNotification) -> Result<(), DieselError> {
        diesel::insert_into(notifications::table)
            .values(new)
            .execute(conn)?;
        Ok(())
    }
    pub async fn use_notification(&self, post_id_value: i32, user_id_value: i32) -> Result<(), DieselError> {
        self.run(move |conn| Self::use_notification_(conn, post_id_value, user_id_value)).await
    }
    fn use_notification_(conn: &PgConnection, post_id_value: i32, user_id_value: i32) -> Result<(), DieselError> {
        use self::notifications::dsl::*;
        diesel::delete(notifications.filter(user_id.eq(user_id_value)).filter(post_id.eq(post_id_value)))
            .execute(conn)?;
        Ok(())
    }
    pub async fn set_readpoint(&self, post_id_value: i32, user_id_value: i32, comment_id_value: i32) -> Result<(), DieselError> {
        self.run(move |conn| Self::set_readpoint_(conn, post_id_value, user_id_value, comment_id_value)).await
    }
    fn set_readpoint_(conn: &PgConnection, post_id_value: i32, user_id_value: i32, comment_id_value: i32) -> Result<(), DieselError> {
        use self::comment_readpoints::dsl::*;
        let r = diesel::update(
                comment_readpoints
                .filter(user_id.eq(user_id_value))
                .filter(post_id.eq(post_id_value))
            )
            .set(comment_readpoint.eq(comment_id_value))
            .get_result::<ReadPoint>(conn);
        if let Err(diesel::result::Error::NotFound) = r {
            diesel::insert_into(comment_readpoints)
                .values(ReadPoint {
                    user_id: user_id_value,
                    post_id: post_id_value,
                    comment_readpoint: comment_id_value,
                })
                .execute(conn)?;
        } else if let Err(e) = r {
            return Err(e);
        }
        Ok(())
    }
    pub async fn list_notifications(&self, user_id_value: i32) -> Result<Vec<NotificationInfo>, DieselError> {
        if user_id_value == 0 {
            return Ok(Vec::new());
        }
        self.run(move |conn| Self::list_notifications_(conn, user_id_value)).await
    }
    fn list_notifications_(conn: &PgConnection, user_id_value: i32) -> Result<Vec<NotificationInfo>, DieselError> {
        use self::posts::dsl::*;
        use self::users::dsl::*;
        use self::notifications::dsl::*;
        if user_id_value == 0 {
            return Ok(Vec::new());
        }
        let all: Vec<NotificationInfo> = notifications
            .inner_join(users.on(self::users::dsl::id.eq(self::notifications::dsl::created_by)))
            .inner_join(posts)
            .select((
                self::posts::dsl::uuid,
                self::posts::dsl::title,
                self::posts::dsl::comment_count,
                self::users::dsl::username,
            ))
            .filter(visible.eq(true))
            .filter(self::notifications::dsl::user_id.eq(user_id_value))
            .order_by(self::notifications::dsl::created_at.asc())
            .limit(50)
            .get_results::<(Base32, String, i32, String)>(conn)?
            .into_iter()
            .map(|t| tuple_to_notification_info(t))
            .collect();
        Ok(all)
    }
    pub async fn is_subscribed(&self, post_id_value: i32, user_id_value: i32) -> Result<bool, DieselError> {
        if user_id_value == 0 {
            return Ok(false);
        }
        self.run(move |conn| Self::is_subscribed_(conn, post_id_value, user_id_value)).await
    }
    fn is_subscribed_(conn: &PgConnection, post_id_value: i32, user_id_value: i32) -> Result<bool, DieselError> {
        use self::subscriptions::dsl::*;
        use diesel::{select, dsl::exists};
        if user_id_value == 0 {
            return Ok(false);
        }
        Ok(select(exists(subscriptions
            .filter(post_id.eq(post_id_value))
            .filter(user_id.eq(user_id_value))))
            .get_result::<bool>(conn)?)
    }
    pub async fn create_subscription(&self, new: NewSubscription) -> Result<(), DieselError> {
        self.run(move |conn| Self::create_subscription_(conn, new)).await
    }
    fn create_subscription_(conn: &PgConnection, new: NewSubscription) -> Result<(), DieselError> {
        diesel::insert_into(subscriptions::table)
            .values(new)
            .execute(conn)?;
        Ok(())
    }
    pub async fn drop_subscription(&self, new: NewSubscription) -> Result<(), DieselError> {
        self.run(move |conn| Self::drop_subscription_(conn, new)).await
    }
    fn drop_subscription_(conn: &PgConnection, new: NewSubscription) -> Result<(), DieselError> {
        use self::subscriptions::dsl::*;
        diesel::delete(subscriptions.filter(user_id.eq(new.user_id)).filter(post_id.eq(new.post_id)))
            .execute(conn)?;
        Ok(())
    }
    pub async fn list_subscribed_users(&self, post_id_value: i32) -> Result<Vec<i32>, DieselError> {
        self.run(move |conn| Self::list_subscribed_users_(conn, post_id_value)).await
    }
    fn list_subscribed_users_(conn: &PgConnection, post_id_value: i32) -> Result<Vec<i32>, DieselError> {
        use self::subscriptions::dsl::*;
        Ok(subscriptions
            .select(user_id)
            .filter(post_id.eq(post_id_value))
            .get_results::<i32>(conn)?)
    }
    pub async fn search_posts(&self, search: &PostSearch) -> Result<Vec<PostInfo>, DieselError> {
        let search = search.clone();
        self.run(move |conn| Self::search_posts_(conn, &search)).await
    }
    fn search_posts_(conn: &PgConnection, search: &PostSearch) -> Result<Vec<PostInfo>, DieselError> {
        use self::posts::dsl::{self as p, *};
        use self::stars::dsl::{self as s, *};
        use self::flags::dsl::{self as f, *};
        use self::post_hides::dsl::{self as ph, *};
        use self::users::dsl::{self as u, *};
        use self::subscriptions::dsl::{self as sb, *};
        use self::post_tagging::dsl::{self as pt, *};
        use self::comment_readpoints::dsl::{self as cr, *};
        use crate::schema::post_search_index::dsl::*;
        use diesel_full_text_search::{plainto_tsquery, TsVectorExtensions, ts_rank_cd};
        let query = posts.filter(visible.eq(true));
        let mut query = match search.order_by {
            PostSearchOrderBy::Hottest if search.keywords == "" => query.order_by((initial_stellar_time.desc(), p::created_at.desc())).into_boxed(),
            PostSearchOrderBy::Hottest => query.into_boxed(),
            PostSearchOrderBy::Top => query.order_by((score.desc(), p::created_at.desc())).into_boxed(),
            PostSearchOrderBy::Newest => query.order_by((initial_stellar_time.desc(), p::created_at.desc())).into_boxed(),
            PostSearchOrderBy::Latest => query.order_by(p::updated_at.desc()).into_boxed(),
            PostSearchOrderBy::Random => query.order_by(random).into_boxed(),
        };
        if !search.or_domains.is_empty() {
            query = query.filter(domain_id.eq_any(&search.or_domains))
        }
        if !search.or_tags.is_empty() {
            let ids = post_tagging
                .filter(pt::tag_id.eq_any(&search.or_tags))
                .select(pt::post_id);
            query = query.filter(p::id.eq_any(ids));
        }
        if !search.and_tags.is_empty() {
            for &tag_id_ in &search.and_tags {
                let ids = post_tagging
                    .filter(pt::tag_id.eq(tag_id_))
                    .select(pt::post_id);
                query = query.filter(p::id.eq_any(ids));
            }
        }
        if !search.hide_tags.is_empty() {
            let ids = post_tagging
                .filter(pt::tag_id.eq_any(&search.hide_tags))
                .select(pt::post_id);
            query = query.filter(diesel::dsl::not(p::id.eq_any(ids)));
        }
        if search.subscriptions {
            let ids = subscriptions
                .filter(sb::user_id.eq(search.my_user_id))
                .select(sb::post_id);
            query = query.filter(p::id.eq_any(ids));
        } else {
            query = query.filter(private.eq(false));
        }
        if let Some(blog_post_value) = search.blog_post {
            query = query.filter(p::blog_post.eq(blog_post_value));
        }
        let mut before_date = search.before_date;
        let mut after_date = search.after_date;
        if before_date < after_date && before_date.is_some() && after_date.is_some() {
            mem::swap(&mut after_date, &mut before_date);
        }
        if let Some(before_date) = before_date {
            let midnight = NaiveTime::from_hms(23, 59, 59);
            query = query.filter(p::created_at.lt(before_date.and_time(midnight)));
        }
        if let Some(after_date) = after_date {
            let midnight = NaiveTime::from_hms(0, 0, 0);
            query = query.filter(p::created_at.gt(after_date.and_time(midnight)));
        }
        if search.for_user_id != 0 {
            query = query.filter(p::submitted_by.eq(search.for_user_id)).filter(p::anon.eq(false));
        }
        if search.title != "" {
            let title_query = Self::escape_like_query(&search.title);
            query = query.filter(p::title.like(format!("%{}%", &title_query)));
        }
        let mut data = PrettifyData::new(conn, 0);
        let current_stellar_time = if search.order_by == PostSearchOrderBy::Hottest {
            Self::get_current_stellar_time_(conn)
        } else {
            0
        };
        let limit = search.limit.into();
        let mut all: Vec<PostInfo> = if search.keywords != "" && search.order_by == PostSearchOrderBy::Hottest {
            if search.my_user_id != 0 {
                query
                    .left_outer_join(stars.on(s::post_id.eq(p::id).and(s::user_id.eq(search.my_user_id))))
                    .left_outer_join(flags.on(f::post_id.eq(p::id).and(f::user_id.eq(search.my_user_id))))
                    .left_outer_join(post_hides.on(ph::post_id.eq(p::id).and(ph::user_id.eq(search.my_user_id))))
                    .left_outer_join(comment_readpoints.on(cr::post_id.eq(p::id).and(cr::user_id.eq(search.my_user_id))))
                    .inner_join(users)
                    .inner_join(post_search_index)
                    .select((
                        p::id,
                        p::uuid,
                        p::title,
                        p::title_html,
                        p::url,
                        p::visible,
                        p::private,
                        p::initial_stellar_time,
                        p::score,
                        p::comment_count,
                        cr::comment_readpoint.nullable(),
                        p::blog_post,
                        p::created_at,
                        p::submitted_by,
                        p::excerpt,
                        p::excerpt_html,
                        s::post_id.nullable(),
                        f::post_id.nullable(),
                        ph::post_id.nullable(),
                        u::username,
                        p::banner_title,
                        p::banner_desc,
                        p::noindex,
                        p::locked,
                        p::anon,
                    ))
                    .filter(search_index.matches(plainto_tsquery(&search.keywords)))
                    .order_by(ts_rank_cd(search_index, plainto_tsquery(&search.keywords)).desc())
                    .offset(search.search_page as i64 * limit)
                    .limit(limit)
                    .get_results::<(i32, Base32, String, Option<String>, Option<String>, bool, bool, i32, i32, i32, Option<i32>, bool, NaiveDateTime, i32, Option<String>, Option<String>, Option<i32>, Option<i32>, Option<i32>, String, Option<String>, Option<String>, bool, bool, bool)>(conn)?
                    .into_iter()
                    .map(|t| tuple_to_post_info(&mut data, t, current_stellar_time))
                    .collect()
            } else {
                query
                    .inner_join(users)
                    .inner_join(post_search_index)
                    .select((
                        p::id,
                        p::uuid,
                        p::title,
                        p::title_html,
                        p::url,
                        p::visible,
                        p::private,
                        p::initial_stellar_time,
                        p::score,
                        p::comment_count,
                        p::blog_post,
                        p::created_at,
                        p::submitted_by,
                        p::excerpt,
                        p::excerpt_html,
                        u::username,
                        p::banner_title,
                        p::banner_desc,
                        p::noindex,
                        p::locked,
                        p::anon,
                    ))
                    .filter(search_index.matches(plainto_tsquery(&search.keywords)))
                    .order_by(ts_rank_cd(search_index, plainto_tsquery(&search.keywords)).desc())
                    .offset(search.search_page as i64 * limit)
                    .limit(limit)
                    .get_results::<(i32, Base32, String, Option<String>, Option<String>, bool, bool, i32, i32, i32, bool, NaiveDateTime, i32, Option<String>, Option<String>, String, Option<String>, Option<String>, bool, bool, bool)>(conn)?
                    .into_iter()
                    .map(|t| tuple_to_post_info_logged_out(&mut data, t, current_stellar_time))
                    .collect()
            }
        } else if search.my_user_id == 0 {
            if search.keywords != "" {
                let ids = post_search_index
                    .filter(search_index.matches(plainto_tsquery(&search.keywords)))
                    .select(crate::schema::post_search_index::dsl::post_id);
                query = query.filter(p::id.eq_any(ids));
            } else if search.after_post_id != 0 {
                query = query.filter(p::id.lt(search.after_post_id));
            }
            let search_page = if search.keywords == "" {
                0
            } else {
                search.search_page
            };
            query
                .inner_join(users)
                .select((
                    p::id,
                    p::uuid,
                    p::title,
                    p::title_html,
                    p::url,
                    p::visible,
                    p::private,
                    p::initial_stellar_time,
                    p::score,
                    p::comment_count,
                    p::blog_post,
                    p::created_at,
                    p::submitted_by,
                    p::excerpt,
                    p::excerpt_html,
                    u::username,
                    p::banner_title,
                    p::banner_desc,
                    p::noindex,
                    p::locked,
                    p::anon,
                ))
                .offset(search_page as i64 * limit)
                .limit(limit)
                .get_results::<(i32, Base32, String, Option<String>, Option<String>, bool, bool, i32, i32, i32, bool, NaiveDateTime, i32, Option<String>, Option<String>, String, Option<String>, Option<String>, bool, bool, bool)>(conn)?
                .into_iter()
                .map(|t| tuple_to_post_info_logged_out(&mut data, t, current_stellar_time))
                .collect()
        } else {
            if search.keywords != "" {
                let ids = post_search_index
                    .filter(search_index.matches(plainto_tsquery(&search.keywords)))
                    .select(crate::schema::post_search_index::dsl::post_id);
                query = query.filter(p::id.eq_any(ids));
            } else if search.after_post_id != 0 {
                query = query.filter(p::id.lt(search.after_post_id));
            }
            let search_page = if search.keywords == "" {
                0
            } else {
                search.search_page
            };
            query
                .left_outer_join(stars.on(s::post_id.eq(p::id).and(s::user_id.eq(search.my_user_id))))
                .left_outer_join(flags.on(f::post_id.eq(p::id).and(f::user_id.eq(search.my_user_id))))
                .left_outer_join(post_hides.on(ph::post_id.eq(p::id).and(ph::user_id.eq(search.my_user_id))))
                .left_outer_join(comment_readpoints.on(cr::post_id.eq(p::id).and(cr::user_id.eq(search.my_user_id))))
                .inner_join(users)
                .select((
                    p::id,
                    p::uuid,
                    p::title,
                    p::title_html,
                    p::url,
                    p::visible,
                    p::private,
                    p::initial_stellar_time,
                    p::score,
                    p::comment_count,
                    cr::comment_readpoint.nullable(),
                    p::blog_post,
                    p::created_at,
                    p::submitted_by,
                    p::excerpt,
                    p::excerpt_html,
                    s::post_id.nullable(),
                    f::post_id.nullable(),
                    ph::post_id.nullable(),
                    u::username,
                    p::banner_title,
                    p::banner_desc,
                    p::noindex,
                    p::locked,
                    p::anon,
                ))
                .offset(search_page as i64 * limit)
                .limit(limit)
                .get_results::<(i32, Base32, String, Option<String>, Option<String>, bool, bool, i32, i32, i32, Option<i32>, bool, NaiveDateTime, i32, Option<String>, Option<String>, Option<i32>, Option<i32>, Option<i32>, String, Option<String>, Option<String>, bool, bool, bool)>(conn)?
                .into_iter()
                .map(|t| tuple_to_post_info(&mut data, t, current_stellar_time))
                .collect()
        };
        if let (PostSearchOrderBy::Hottest, "") = (search.order_by, &search.keywords[..]) {
            all.sort_by_key(|info| OrderedFloat(-info.hotness));
            if let Ok(limit) = search.limit.try_into() {
                all.truncate(if limit > 20usize { limit - 10 } else { limit });
            }
        }
        if search.order_by == PostSearchOrderBy::Random {
            use ::rand::seq::SliceRandom;
            use ::rand::thread_rng;
            all.shuffle(&mut thread_rng());
        }
        Ok(all)
    }
    pub async fn get_post_info_similar(&self, user_id_param: i32, post_info: Post) -> Result<Vec<PostInfo>, DieselError> {
        self.run(move |conn| Self::get_post_info_similar_(conn, user_id_param, post_info)).await
    }
    fn get_post_info_similar_(conn: &PgConnection, user_id_param: i32, post_info: Post) -> Result<Vec<PostInfo>, DieselError> {
        use self::posts::dsl::{self as p, *};
        use self::stars::dsl::{self as s, *};
        use self::flags::dsl::{self as f, *};
        use self::post_hides::dsl::{self as ph, *};
        use self::users::dsl::{self as u, *};
        use self::comment_readpoints::{self as cr, *};
        use crate::schema::post_search_index::dsl::{self as psi, *};
        use diesel_full_text_search::{to_tsquery, ts_rank, TsVectorExtensions};
        // Find the 10 least frequent words in this post.
        // This is used as the outermost filter, because otherwise this would take forever.
        #[derive(QueryableByName)]
        #[table_name="post_word_freq"]
        struct Word { word: String }
        let mut words: Vec<Word> = diesel::sql_query(format!(r##"
          SELECT
            PWF.word AS word
          FROM post_word_freq AS PWF
          INNER JOIN (SELECT word FROM TS_STAT(CONCAT('SELECT to_tsvector(regexp_replace(regexp_replace(regexp_replace(excerpt, ''\[[uU][rR][lL]=(.*)\](.*?)\[\/[uU][rR][lL]\]'', ''\2''), ''\[[bB]\](.*?)\[/[bB]\]'', ''\1''), ''\[[iI]\](.*?)\[/[iI]\]'', ''\1'')) FROM posts WHERE id = {}'))) AS TS ON (TS.word = PWF.word)
          ORDER BY PWF.num ASC
          LIMIT 20
        "##, post_info.id)).get_results::<Word>(conn)?;
        let mut     words_title: Vec<Word> = diesel::sql_query(format!(r##"
          SELECT
            PWF.word AS word
          FROM post_word_freq AS PWF
          INNER JOIN (SELECT word FROM TS_STAT(CONCAT('SELECT to_tsvector(title) FROM posts WHERE id = {}'))) AS TS ON (TS.word = PWF.word)
          ORDER BY PWF.num ASC
          LIMIT 20
        "##, post_info.id)).get_results::<Word>(conn)?;
        words.append(&mut words_title);
        let word_list_short = words.iter().take(10).map(|word| &word.word[..]).collect::<Vec<&str>>().join("|");
        let word_list = words.iter().map(|word| &word.word[..]).collect::<Vec<&str>>().join("&");
        // Now actually find the "similar" posts.
        let mut data = PrettifyData::new(conn, 0);
        let current_stellar_time = Self::get_current_stellar_time_(conn);
        let all: Vec<PostInfo> = posts
            .left_outer_join(stars.on(s::post_id.eq(self::posts::dsl::id).and(s::user_id.eq(user_id_param))))
            .left_outer_join(flags.on(f::post_id.eq(p::id).and(f::user_id.eq(user_id_param))))
            .left_outer_join(post_hides.on(ph::post_id.eq(p::id).and(ph::user_id.eq(user_id_param))))
            .left_outer_join(cr::table.on(cr::post_id.eq(p::id).and(cr::user_id.eq(user_id_param))))
            .inner_join(users)
            .inner_join(post_search_index)
            .select((
                p::id,
                p::uuid,
                p::title,
                p::title_html,
                p::url,
                p::visible,
                p::private,
                p::initial_stellar_time,
                p::score,
                p::comment_count,
                cr::comment_readpoint.nullable(),
                p::blog_post,
                p::created_at,
                p::submitted_by,
                p::excerpt,
                p::excerpt_html,
                s::post_id.nullable(),
                f::post_id.nullable(),
                ph::post_id.nullable(),
                u::username,
                p::banner_title,
                p::banner_desc,
                p::noindex,
                p::locked,
                p::anon,
            ))
            .filter(visible.eq(true))
            .filter(private.eq(false))
            .filter(p::id.ne(post_info.id))
            .filter(psi::search_index.matches(to_tsquery(&word_list_short)))
            .order_by(ts_rank(psi::search_index, to_tsquery(&word_list)).desc())
            .limit(50)
            .get_results::<(i32, Base32, String, Option<String>, Option<String>, bool, bool, i32, i32, i32, Option<i32>, bool, NaiveDateTime, i32, Option<String>, Option<String>, Option<i32>, Option<i32>, Option<i32>, String, Option<String>, Option<String>, bool, bool, bool)>(conn)?
            .into_iter()
            .map(|t| tuple_to_post_info(&mut data, t, current_stellar_time))
            .collect();
        Ok(all)
    }
    pub async fn get_post_info_by_uuid(&self, user_id_param: i32, uuid_param: Base32) -> Result<PostInfo, DieselError> {
        self.run(move |conn| Self::get_post_info_by_uuid_(conn, user_id_param, uuid_param)).await
    }
    fn get_post_info_by_uuid_(conn: &PgConnection, user_id_param: i32, uuid_param: Base32) -> Result<PostInfo, DieselError> {
        use self::posts::dsl::{self as p, *};
        use self::stars::dsl::{self as s, *};
        use self::flags::dsl::{self as f, *};
        use self::post_hides::dsl::{self as ph, *};
        use self::users::dsl::{self as u, *};
        use self::comment_readpoints::dsl::{self as cr, *};
        // This is a bunch of duplicate code.
        let mut data = PrettifyData::new(conn, 0);
        Ok(tuple_to_post_info(&mut data, posts
            .left_outer_join(stars.on(s::post_id.eq(p::id).and(s::user_id.eq(user_id_param))))
            .left_outer_join(flags.on(f::post_id.eq(p::id).and(f::user_id.eq(user_id_param))))
            .left_outer_join(post_hides.on(ph::post_id.eq(p::id).and(ph::user_id.eq(user_id_param))))
            .left_outer_join(comment_readpoints.on(cr::post_id.eq(p::id).and(cr::user_id.eq(user_id_param))))
            .inner_join(users)
            .select((
                p::id,
                p::uuid,
                p::title,
                p::title_html,
                p::url,
                p::visible,
                p::private,
                p::initial_stellar_time,
                p::score,
                p::comment_count,
                cr::comment_readpoint.nullable(),
                p::blog_post,
                p::created_at,
                p::submitted_by,
                p::excerpt,
                p::excerpt_html,
                s::post_id.nullable(),
                f::post_id.nullable(),
                ph::post_id.nullable(),
                u::username,
                p::banner_title,
                p::banner_desc,
                p::noindex,
                p::locked,
                p::anon,
            ))
            .filter(rejected.eq(false))
            .filter(uuid.eq(uuid_param.into_i64()))
            .first::<(i32, Base32, String, Option<String>, Option<String>, bool, bool, i32, i32, i32, Option<i32>, bool, NaiveDateTime, i32, Option<String>, Option<String>, Option<i32>, Option<i32>, Option<i32>, String, Option<String>, Option<String>, bool, bool, bool)>(conn)?, Self::get_current_stellar_time_(conn)))
    }
    pub async fn create_poll(&self, post_id: i32, title: String, choices: Vec<String>, created_by: i32) -> Result<Poll, DieselError> {
        self.run(move |conn| Self::create_poll_(conn, post_id, title, choices, created_by)).await
    }
    fn create_poll_(conn: &PgConnection, post_id: i32, title: String, choices: Vec<String>, created_by: i32) -> Result<Poll, DieselError> {
        use self::polls::dsl as p;
        use self::poll_choices as pc;
        #[derive(Insertable)]
        #[table_name="polls"]
        struct NewPoll {
            post_id: i32,
            title: String,
            created_by: i32,
        }
        #[derive(Insertable)]
        #[table_name="poll_choices"]
        struct NewPollChoice {
            poll_id: i32,
            title: String,
            created_by: i32,
        }
        let result = diesel::insert_into(polls::table)
            .values(NewPoll {
                post_id,
                title,
                created_by,
            })
            .get_result::<Poll>(conn)?;
        for choice in choices {
            diesel::insert_into(poll_choices::table)
                .values(NewPollChoice {
                    poll_id: result.id,
                    title: choice,
                    created_by,
                })
                .execute(conn)?;
        }
        Ok(result)
    }
    pub async fn close_poll(&self, poll_id_value: i32) -> Result<Poll, DieselError> {
        self.run(move |conn| Self::close_poll_(conn, poll_id_value)).await
    }
    fn close_poll_(conn: &PgConnection, poll_id_value: i32) -> Result<Poll, DieselError> {
        use self::polls::dsl::*;
        diesel::update(polls.find(poll_id_value))
            .set((
                open.eq(false),
            ))
            .execute(conn)?;
        polls.find(poll_id_value).get_result::<Poll>(conn)
    }
    pub async fn get_poll(&self, user_id_param: i32, uuid_param: Base32) -> Result<Vec<PollInfo>, DieselError> {
        self.run(move |conn| Self::get_poll_(conn, user_id_param, uuid_param)).await
    }
    fn get_poll_(conn: &PgConnection, user_id_param: i32, uuid_param: Base32) -> Result<Vec<PollInfo>, DieselError> {
        use self::polls::dsl::{self as p, *};
        use self::posts::dsl::{self as posts_, *};
        use self::poll_choices::dsl::{self as pc, *};
        use self::poll_votes::dsl::{self as pv, *};
        let results = polls
            .inner_join(posts)
            .inner_join(poll_choices.left_outer_join(pv::poll_votes.on(pv::user_id.eq(user_id_param).and(pc::id.eq(pv::choice_id)))).on(pc::poll_id.eq(p::id)))
            .select((
                p::title,
                p::id,
                pc::title,
                pc::id,
                pv::score.nullable(),
                p::open,
            ))
            .filter(posts_::uuid.eq(uuid_param.into_i64()))
            .get_results::<(String, i32, String, i32, Option<i32>, bool)>(conn)?;
        let result = results.into_iter().fold(Vec::new(), |mut b: Vec<PollInfo>, item| {
            let average = poll_choices
                .inner_join(poll_votes)
                .select(diesel::dsl::avg(pv::score))
                .filter(pc::id.eq(item.3))
                .get_result::<Option<BigDecimal>>(conn)
                .ok()
                .flatten()
                .and_then(|x| x.to_f64())
                .unwrap_or(0.0);
            let count: i32 = poll_choices
                .inner_join(poll_votes)
                .select(diesel::dsl::count_star())
                .filter(pc::id.eq(item.3))
                .get_result::<i64>(conn)
                .unwrap_or(0i64) // report failure as 0
                .try_into()
                .unwrap_or(i32::MAX); // report overflow as i32::MAX
            if let Some(current) = b.last_mut() {
                if current.poll_id == item.1 {
                    current.choices.push(PollInfoChoice {
                        title: item.2.clone(),
                        choice_id: item.3,
                        score: item.4.unwrap_or(0),
                        average, count,
                    });
                    return b;
                }
            }
            let choices = vec![
                PollInfoChoice {
                    title: item.2.clone(),
                    choice_id: item.3,
                    score: item.4.unwrap_or(0),
                    average, count,
                }
            ];
            let new = PollInfo {
                poll_id: item.1,
                title: item.0.clone(),
                open: item.5,
                choices,
            };
            b.push(new);
            b
        });
        Ok(result)
    }
    pub async fn vote_poll(&self, user_id_param: i32, uuid_param: Base32, choice_id_param: i32, score_param: i32) -> Result<(), DieselError> {
        self.run(move |conn| Self::vote_poll_(conn, user_id_param, uuid_param, choice_id_param, score_param)).await
    }
    fn vote_poll_(conn: &PgConnection, user_id_param: i32, uuid_param: Base32, choice_id_param: i32, score_param: i32) -> Result<(), DieselError> {
        use self::polls::dsl::{self as p, *};
        use self::posts::dsl::{self as posts_, *};
        use self::poll_choices::dsl::{self as pc, *};
        use self::poll_votes::dsl as pv;
        #[derive(Insertable)]
        #[table_name="poll_votes"]
        struct NewPollVote {
            pub user_id: i32,
            pub choice_id: i32,
            pub score: i32,
        }
        let info = Self::get_poll_(conn, user_id_param, uuid_param)?;
        if !info.iter().any(|s| s.choices.iter().any(|s| s.choice_id == choice_id_param)) {
            return Err(diesel::result::Error::NotFound);
        }
        let r = diesel::update(
                pv::poll_votes
                .filter(pv::user_id.eq(user_id_param))
                .filter(pv::choice_id.eq(choice_id_param))
            )
            .set(pv::score.eq(score_param))
            .get_result::<PollVote>(conn);
        if let Err(diesel::result::Error::NotFound) = r {
            diesel::insert_into(pv::poll_votes)
                .values(NewPollVote {
                    user_id: user_id_param,
                    choice_id: choice_id_param,
                    score: score_param
                })
                .execute(conn)?;
        } else if let Err(e) = r {
            return Err(e);
        }
        Ok(())
    }
    pub async fn get_user_comments_count_today(&self, user_id_param: i32) -> i32 {
        self.run(move |conn| Self::get_user_comments_count_today_(conn, user_id_param)).await
    }
    fn get_user_comments_count_today_(conn: &PgConnection, user_id_value: i32) -> i32 {
        use self::comments::dsl::*;
        let yesterday = Utc::now().naive_utc() - Duration::days(1);
        comments
            .filter(created_by.eq(user_id_value))
            .filter(created_at.gt(yesterday))
            .count()
            .get_result(conn)
            .unwrap_or(0) as i32
    }
    pub async fn get_domain_posts_count_today(&self, domain_id_param: i32) -> i32 {
        self.run(move |conn| Self::get_domain_posts_count_today_(conn, domain_id_param)).await
    }
    fn get_domain_posts_count_today_(conn: &PgConnection, domain_id_value: i32) -> i32 {
        use self::posts::dsl::*;
        let yesterday = Utc::now().naive_utc() - Duration::days(1);
        posts
            .filter(domain_id.eq(domain_id_value))
            .filter(created_at.gt(yesterday))
            .count()
            .get_result(conn)
            .unwrap_or(0) as i32
    }
    pub async fn get_userdomain_posts_count_today(&self, user_id_param: i32, domain_id_param: i32) -> i32 {
        self.run(move |conn| Self::get_userdomain_posts_count_today_(conn, user_id_param, domain_id_param)).await
    }
    fn get_userdomain_posts_count_today_(conn: &PgConnection, user_id_value: i32, domain_id_value: i32) -> i32 {
        use self::posts::dsl::*;
        let yesterday = Utc::now().naive_utc() - Duration::days(1);
        posts
            .filter(domain_id.eq(domain_id_value))
            .filter(submitted_by.eq(user_id_value))
            .filter(created_at.gt(yesterday))
            .count()
            .get_result(conn)
            .unwrap_or(0) as i32
    }
    pub async fn get_user_posts_count_today(&self, user_id_param: i32) -> i32 {
        self.run(move |conn| Self::get_user_posts_count_today_(conn, user_id_param)).await
    }
    fn get_user_posts_count_today_(conn: &PgConnection, user_id_value: i32) -> i32 {
        use self::posts::dsl::*;
        let yesterday = Utc::now().naive_utc() - Duration::days(1);
        posts
            .filter(submitted_by.eq(user_id_value))
            .filter(created_at.gt(yesterday))
            .count()
            .get_result(conn)
            .unwrap_or(0) as i32
    }
    pub async fn get_current_stellar_time(&self) -> i32 {
        self.run(move |conn| Self::get_current_stellar_time_(conn)).await
    }
    fn get_current_stellar_time_(conn: &PgConnection) -> i32 {
        use self::stars::dsl::*;
        // the stars table should be limited by the i32 limits, but diesel doesn't know that
        stars.count().get_result(conn).unwrap_or(0) as i32
    }
    async fn get_post_domain_url(&self, url: Option<String>) -> (Option<String>, Option<Domain>) {
        self.run(move |conn| Self::get_post_domain_url_(conn, url)).await
    }
    fn get_post_domain_url_(conn: &PgConnection, url: Option<String>) -> (Option<String>, Option<Domain>) {
        let url_host = url
            .and_then(|u| Url::parse(&u[..]).ok())
            .and_then(|u| { let h = u.host_str()?.to_owned(); Some((u, h)) });
        if let Some((mut url, host)) = url_host {
            let mut host = &host[..];
            let mut is_www = false;
            if host.starts_with("www.") {
                host = &host[4..];
                is_www = true;
            }
            // These domains are "true synonyms,"
            // meaning we can actually strip the subdomain off and the link will always work.
            // Regular domain synonyms, that you can add through the UI, don't work that way.
            if host.starts_with("np.reddit.com") {
                host = &host[3..];
            }
            if host.starts_with("old.reddit.com") {
                host = &host[4..];
            }
            if host.starts_with("new.reddit.com") {
                host = &host[4..];
            }
            if host.starts_with("i.reddit.com") {
                host = &host[2..];
            }
            if host.starts_with("m.reddit.com") {
                host = &host[2..];
            }
            if host.starts_with("mobile.twitter.com") {
                host = &host[7..];
            }
            let is_https = url.scheme() == "https";
            let domain = Self::get_domain_by_hostname_(conn, host).unwrap_or_else(|_| {
                Self::create_domain_(conn, NewDomain {
                    hostname: host.to_owned(),
                    is_www, is_https
                }).expect("if domain does not exist, creating it should work")
            });
            if !is_www && domain.is_www {
                url.set_host(Some(&format!("www.{}", host))).expect("if is-www is true, then this scheme has a host");
            } else if is_www && !domain.is_www {
                url.set_host(Some(&host[..])).expect("if is-www is true, then this scheme has a host");
            }
            if !is_https && domain.is_https && url.scheme() == "http" {
                url.set_scheme("https").expect("https is a valid scheme");
            }
            (Some(url.to_string()), Some(domain))
        } else {
            (None, None)
        }
    }
    pub async fn create_post(&self, new_post: NewPost, body_format: BodyFormat, enforce_rate_limit: bool) -> Result<Post, CreatePostError> {
        self.run(move |conn| Self::create_post_(conn, new_post, body_format, enforce_rate_limit)).await
    }
    fn create_post_(conn: &PgConnection, new_post: NewPost, body_format: BodyFormat, enforce_rate_limit: bool) -> Result<Post, CreatePostError> {
        #[derive(Insertable)]
        #[table_name="posts"]
        struct CreatePost<'a> {
            title: &'a str,
            title_html: Option<&'a str>,
            uuid: i64,
            url: Option<String>,
            submitted_by: i32,
            initial_stellar_time: i32,
            excerpt: Option<&'a str>,
            excerpt_html: Option<&'a str>,
            visible: bool,
            private: bool,
            blog_post: bool,
            domain_id: Option<i32>,
            anon: bool,
        }
        let uuid: i64 = ::rand::random();
        let uuid_string = Base32::from(uuid).to_string();
        let mut visible = new_post.visible;
        let (url, domain) = Self::get_post_domain_url_(conn, new_post.url.as_ref().cloned());
        let url_str = url.as_ref().map(|u| &u[..]).unwrap_or(&uuid_string);
        let title_html_and_stuff = crate::prettify::prettify_title(&new_post.title, url_str, &mut PrettifyData::new(conn, 0), new_post.blog_post);
        if title_html_and_stuff.hash_tags.is_empty() && !new_post.private && !new_post.blog_post {
            return Err(CreatePostError::RequireTag);
        }
        // TODO: make this configurable
        if Self::get_user_posts_count_today_(conn, new_post.submitted_by) >= 5 && enforce_rate_limit {
            return Err(CreatePostError::TooManyPosts);
        }
        let excerpt_html_and_stuff = if let Some(excerpt) = &new_post.excerpt {
            let body = match body_format {
                BodyFormat::Plain => crate::prettify::prettify_body(&excerpt, &mut PrettifyData::new(conn, 0)),
                BodyFormat::BBCode => crate::prettify::prettify_body_bbcode(&excerpt, &mut PrettifyData::new(conn, 0)),
            };
            Some(body)
        } else {
            None
        };
        if new_post.title.chars().filter(|&c| c != ' ' && c != '\n').count() > 500 {
            return Err(CreatePostError::TooLong);
        }
        if new_post.excerpt.as_ref().map(|x| &x[..]).unwrap_or("").chars().filter(|&c| c != ' ' && c != '\n').count() > 2000 && !new_post.private && !new_post.blog_post {
            return Err(CreatePostError::TooLong);
        }
        if let Some(ref domain) = domain {
            if Self::get_domain_posts_count_today_(conn, domain.id) >= 4 && enforce_rate_limit {
                return Err(CreatePostError::TooManyPostsDomain);
            }
            if Self::get_userdomain_posts_count_today_(conn, new_post.submitted_by, domain.id) >= 2 && enforce_rate_limit {
                return Err(CreatePostError::TooManyPostsDomainUser);
            }
            if let Ok(restriction) = Self::get_domain_restriction_by_id_(conn, domain.id) {
                if restriction.restriction_level > 2 {
                    return Err(CreatePostError::TooManyPostsDomain);
                } else if restriction.restriction_level > 0 {
                    visible = false;
                }
            }
        }
        let result = diesel::insert_into(posts::table)
            .values(CreatePost {
                title: &new_post.title,
                title_html: Some(&title_html_and_stuff.string[..]),
                uuid,
                submitted_by: new_post.submitted_by,
                initial_stellar_time: Self::get_current_stellar_time_(conn),
                excerpt: new_post.excerpt.as_ref().map(|x| &x[..]),
                excerpt_html: excerpt_html_and_stuff.as_ref().map(|e| &e.string[..]),
                private: new_post.private,
                blog_post: new_post.blog_post,
                anon: new_post.anon,
                domain_id: domain.map(|d| d.id),
                url: url.as_ref().cloned(),
                visible,
            })
            .get_result::<Post>(conn);
        if let Ok(ref post) = result {
            for tag in title_html_and_stuff.hash_tags.iter().chain(excerpt_html_and_stuff.iter().flat_map(|e| e.hash_tags.iter())).map(|s| &s[..]).collect::<HashSet<&str>>() {
                if let Ok(tag_info) = Self::get_tag_by_name_(conn, &tag) {
                    diesel::insert_into(post_tagging::table)
                        .values(CreatePostTagging {
                            post_id: post.id,
                            tag_id: tag_info.id,
                        })
                        .execute(conn)?;
                }
            }
        }
        result.map_err(Into::into)
    }
    pub async fn update_post(&self, post_id_value: i32, bump: bool, new_post: NewPost, body_format: BodyFormat) -> Result<(), DieselError> {
        self.run(move |conn| Self::update_post_(conn, post_id_value, bump, new_post, body_format)).await
    }
    fn update_post_(conn: &PgConnection, post_id_value: i32, bump: bool, new_post: NewPost, body_format: BodyFormat) -> Result<(), DieselError> {
        let (url_value, domain) = Self::get_post_domain_url_(conn, new_post.url);
        let url_str = url_value.as_ref().map(|u| &u[..]).unwrap_or("");
        let title_html_and_stuff = crate::prettify::prettify_title(&new_post.title, url_str, &mut PrettifyData::new(conn, 0), new_post.blog_post);
        let excerpt_html_and_stuff = if let Some(e) = &new_post.excerpt {
            let body = match body_format {
                BodyFormat::Plain => crate::prettify::prettify_body(&e, &mut PrettifyData::new(conn, 0)),
                BodyFormat::BBCode => crate::prettify::prettify_body_bbcode(&e, &mut PrettifyData::new(conn, 0)),
            };
            Some(body)
        } else {
            None
        };
        use self::posts::dsl::*;
        use self::post_tagging::dsl::*;
        diesel::update(posts.find(post_id_value))
            .set((
                title.eq(new_post.title),
                title_html.eq(Some(title_html_and_stuff.string)),
                excerpt.eq(new_post.excerpt),
                url.eq(url_value),
                excerpt_html.eq(excerpt_html_and_stuff.as_ref().map(|x| &x.string[..])),
                visible.eq(new_post.visible),
                private.eq(new_post.private),
                blog_post.eq(new_post.blog_post),
                domain_id.eq(domain.map(|d| d.id))
            ))
            .execute(conn)?;
        if bump {
            diesel::update(posts.find(post_id_value))
                .set((
                    initial_stellar_time.eq(Self::get_current_stellar_time_(conn)),
                    visible.eq(new_post.visible),
                    private.eq(new_post.private),
                    blog_post.eq(new_post.blog_post),
                ))
                .execute(conn)?;
        }
        diesel::delete(post_tagging.filter(post_id.eq(post_id_value)))
            .execute(conn)?;
        for tag in title_html_and_stuff.hash_tags.iter().chain(excerpt_html_and_stuff.iter().flat_map(|e| e.hash_tags.iter())).map(|s| &s[..]).collect::<HashSet<&str>>() {
            if let Ok(tag_info) = Self::get_tag_by_name_(conn, &tag) {
                diesel::insert_into(post_tagging)
                    .values(CreatePostTagging {
                        post_id: post_id_value,
                        tag_id: tag_info.id,
                    })
                    .execute(conn)?;
            }
        }
        Ok(())
    }
    pub async fn add_star(&self, new_star: &NewStar) -> bool {
        let new_star = new_star.clone();
        self.run(move |conn| Self::add_star_(conn, &new_star)).await
    }
    fn add_star_(conn: &PgConnection, new_star: &NewStar) -> bool {
        let affected_rows = diesel::insert_into(stars::table)
            .values(new_star)
            .execute(conn)
            .unwrap_or(0);
        // affected rows will be 1 if inserted, or 0 otherwise
        Self::update_score_on_post_(conn, new_star.post_id, affected_rows as i32);
        affected_rows == 1
    }
    pub async fn rm_star(&self, new_star: &NewStar) -> bool {
        let new_star = new_star.clone();
        self.run(move |conn| Self::rm_star_(conn, &new_star)).await
    }
    fn rm_star_(conn: &PgConnection, new_star: &NewStar) -> bool {
        use self::stars::dsl::*;
        let affected_rows = diesel::delete(
            stars
                .filter(user_id.eq(new_star.user_id))
                .filter(post_id.eq(new_star.post_id))
        )
            .execute(conn)
            .unwrap_or(0);
        // affected rows will be 1 if deleted, or 0 otherwise
        Self::update_score_on_post_(conn, new_star.post_id, -(affected_rows as i32));
        affected_rows == 1
    }
    pub async fn add_hide(&self, new_hide: &NewHide) -> bool {
        let new_hide = new_hide.clone();
        self.run(move |conn| Self::add_hide_(conn, &new_hide)).await
    }
    fn add_hide_(conn: &PgConnection, new_hide: &NewHide) -> bool {
        let affected_rows = diesel::insert_into(post_hides::table)
            .values(new_hide)
            .execute(conn)
            .unwrap_or(0);
        // affected rows will be 1 if inserted, or 0 otherwise
        affected_rows == 1
    }
    pub async fn rm_hide(&self, new_hide: &NewHide) -> bool {
        let new_hide = new_hide.clone();
        self.run(move |conn| Self::rm_hide_(conn, &new_hide)).await
    }
    fn rm_hide_(conn: &PgConnection, new_hide: &NewHide) -> bool {
        use self::post_hides::dsl::*;
        let affected_rows = diesel::delete(
            post_hides
                .filter(user_id.eq(new_hide.user_id))
                .filter(post_id.eq(new_hide.post_id))
        )
            .execute(conn)
            .unwrap_or(0);
        // affected rows will be 1 if deleted, or 0 otherwise
        affected_rows == 1
    }
    pub async fn add_star_comment(&self, new_star: &NewStarComment) -> bool {
        let new_star = new_star.clone();
        self.run(move |conn| Self::add_star_comment_(conn, &new_star)).await
    }
    fn add_star_comment_(conn: &PgConnection, new_star: &NewStarComment) -> bool {
        let affected_rows = diesel::insert_into(comment_stars::table)
            .values(new_star)
            .execute(conn)
            .unwrap_or(0);
        affected_rows == 1
    }
    pub async fn rm_star_comment(&self, new_star: &NewStarComment) -> bool {
        let new_star = new_star.clone();
        self.run(move |conn| Self::rm_star_comment_(conn, &new_star)).await
    }
    fn rm_star_comment_(conn: &PgConnection, new_star: &NewStarComment) -> bool {
        use self::comment_stars::dsl::*;
        let affected_rows = diesel::delete(
            comment_stars
                .filter(user_id.eq(new_star.user_id))
                .filter(comment_id.eq(new_star.comment_id))
        )
            .execute(conn)
            .unwrap_or(0);
        affected_rows == 1
    }
    pub async fn add_hide_comment(&self, new_hide: &NewHideComment) -> bool {
        let new_hide = new_hide.clone();
        self.run(move |conn| Self::add_hide_comment_(conn, &new_hide)).await
    }
    fn add_hide_comment_(conn: &PgConnection, new_hide: &NewHideComment) -> bool {
        let affected_rows = diesel::insert_into(comment_hides::table)
            .values(new_hide)
            .execute(conn)
            .unwrap_or(0);
        affected_rows == 1
    }
    pub async fn rm_hide_comment(&self, new_hide: &NewHideComment) -> bool {
        let new_hide = new_hide.clone();
        self.run(move |conn| Self::rm_hide_comment_(conn, &new_hide)).await
    }
    fn rm_hide_comment_(conn: &PgConnection, new_hide: &NewHideComment) -> bool {
        use self::comment_hides::dsl::*;
        let affected_rows = diesel::delete(
            comment_hides
                .filter(user_id.eq(new_hide.user_id))
                .filter(comment_id.eq(new_hide.comment_id))
        )
            .execute(conn)
            .unwrap_or(0);
        affected_rows == 1
    }
    fn maybe_invisible_post_(conn: &PgConnection, post_id_param: i32) {
        use self::flags::dsl::*;
        let flag_count: i64 = flags.filter(post_id.eq(post_id_param)).count().get_result(conn).expect("if flagging worked, then so should counting");
        if flag_count == FLAG_INVISIBLE_THRESHOLD {
            Self::invisible_post_(conn, post_id_param).expect("if flagging worked, then so should hiding the post");
        }
    }
    fn maybe_invisible_comment_(conn: &PgConnection, comment_id_param: i32) {
        use self::comment_flags::dsl::*;
        let flag_count: i64 = comment_flags.filter(comment_id.eq(comment_id_param)).count().get_result(conn).expect("if flagging worked, then so should counting");
        if flag_count == FLAG_INVISIBLE_THRESHOLD {
            Self::invisible_comment_(conn, comment_id_param).expect("if flagging worked, then so should hiding the post");
        }
    }
    pub async fn add_flag(&self, new_flag: &NewFlag) -> bool {
        let new_flag = new_flag.clone();
        self.run(move |conn| Self::add_flag_(conn, &new_flag)).await
    }
    fn add_flag_(conn: &PgConnection, new_flag: &NewFlag) -> bool {
        let affected_rows = diesel::insert_into(flags::table)
            .values(new_flag)
            .execute(conn)
            .unwrap_or(0);
        if affected_rows == 1 {
            Self::maybe_invisible_post_(conn, new_flag.post_id);
        }
        affected_rows == 1
    }
    pub async fn rm_flag(&self, new_flag: &NewFlag) -> bool {
        let new_flag = new_flag.clone();
        self.run(move |conn| Self::rm_flag_(conn, &new_flag)).await
    }
    fn rm_flag_(conn: &PgConnection, new_flag: &NewFlag) -> bool {
        use self::flags::dsl::*;
        let affected_rows = diesel::delete(
            flags
                .filter(user_id.eq(new_flag.user_id))
                .filter(post_id.eq(new_flag.post_id))
        )
            .execute(conn)
            .unwrap_or(0);
        affected_rows == 1
    }
    pub async fn add_flag_comment(&self, new_flag: &NewFlagComment) -> bool {
        let new_flag = new_flag.clone();
        self.run(move |conn| Self::add_flag_comment_(conn, &new_flag)).await
    }
    fn add_flag_comment_(conn: &PgConnection, new_flag: &NewFlagComment) -> bool {
        let affected_rows = diesel::insert_into(comment_flags::table)
            .values(new_flag)
            .execute(conn)
            .unwrap_or(0);
        if affected_rows == 1 {
            Self::maybe_invisible_comment_(conn, new_flag.comment_id);
        }
        affected_rows == 1
    }
    pub async fn rm_flag_comment(&self, new_flag: &NewFlagComment) -> bool {
        let new_flag = new_flag.clone();
        self.run(move |conn| Self::rm_flag_comment_(conn, &new_flag)).await
    }
    fn rm_flag_comment_(conn: &PgConnection, new_flag: &NewFlagComment) -> bool {
        use self::comment_flags::dsl::*;
        let affected_rows = diesel::delete(
            comment_flags
                .filter(user_id.eq(new_flag.user_id))
                .filter(comment_id.eq(new_flag.comment_id))
        )
            .execute(conn)
            .unwrap_or(0);
        affected_rows == 1
    }
    fn update_score_on_post_(conn: &PgConnection, post_id_value: i32, count_value: i32) {
        use self::posts::dsl::*;
        diesel::update(posts.find(post_id_value)).set(score.eq(score + count_value))
            .execute(conn)
            .expect("if adding a star worked, then so should updating the post");
    }
    fn update_comment_count_on_post_(conn: &PgConnection, post_id_value: i32, count_value: i32) -> Result<(), DieselError> {
        use self::posts::dsl::*;
        diesel::update(posts.find(post_id_value)).set(comment_count.eq(comment_count + count_value))
            .execute(conn)
            .map(|_| ())
    }
    pub async fn has_users(&self) -> Result<bool, DieselError> {
        self.run(move |conn| Self::has_users_(conn)).await
    }
    fn has_users_(conn: &PgConnection) -> Result<bool, DieselError> {
        use self::users::dsl::*;
        use diesel::{select, dsl::exists};
        select(exists(users.select(id))).get_result(conn)
    }
    pub async fn get_user_by_id(&self, user_id_param: i32) -> Result<User, DieselError> {
        self.run(move |conn| Self::get_user_by_id_(conn, user_id_param)).await
    }
    fn get_user_by_id_(conn: &PgConnection, user_id_param: i32) -> Result<User, DieselError> {
        use self::users::dsl::*;
        users.filter(id.eq(user_id_param)).get_result(conn)
    }
    pub async fn get_user_by_username(&self, username_param: &str) -> Result<User, DieselError> {
        let username_param = username_param.to_owned();
        self.run(move |conn| Self::get_user_by_username_(conn, &username_param)).await
    }
    fn get_user_by_username_(conn: &PgConnection, username_param: &str) -> Result<User, DieselError> {
        use self::users::dsl::*;
        users.filter(username.eq(username_param)).get_result(conn)
    }
    pub async fn register_user(&self, new_user: NewUser) -> Result<User, DieselError> {
        self.run(move |conn| Self::register_user_(conn, new_user)).await
    }
    fn register_user_(conn: &PgConnection, new_user: NewUser) -> Result<User, DieselError> {
        #[derive(Insertable)]
        #[table_name="users"]
        struct CreateUser<'a> {
            username: &'a str,
            password_hash: &'a [u8],
            invited_by: Option<i32>,
            identicon: i32,
        }
        let password_hash = password_hash(&new_user.password);
        let identicon = ::rand::random();
        diesel::insert_into(users::table)
            .values(CreateUser {
                username: &new_user.username[..],
                password_hash: &password_hash[..],
                invited_by: new_user.invited_by, identicon,
            })
            .get_result(conn)
    }
    pub async fn authenticate_user(&self, new_user: &UserAuth) -> Option<User> {
        let new_user = new_user.clone();
        self.run(move |conn| Self::authenticate_user_(conn, &new_user)).await
    }
    fn authenticate_user_(conn: &PgConnection, new_user: &UserAuth) -> Option<User> {
        let mut user = Self::get_user_by_username_(conn, &new_user.username).ok()?;
        if password_verify(&new_user.password, &mut user.password_hash[..]) == PasswordResult::Passed {
            Some(user)
        } else {
            None
        }
    }
    pub async fn change_user_password(&self, user_id_value: i32, password: &str) -> Result<(), DieselError> {
        let password = password.to_owned();
        self.run(move |conn| Self::change_user_password_(conn, user_id_value, &password)).await
    }
    fn change_user_password_(conn: &PgConnection, user_id_value: i32, password: &str) -> Result<(), DieselError> {
        use self::users::dsl::*;
        let password_hash_value = crate::password::password_hash(password);
        diesel::update(users.find(user_id_value)).set(password_hash.eq(password_hash_value))
            .execute(conn)
            .map(|k| { assert_eq!(k, 1); })
    }
    pub async fn change_user_trust_level(&self, user_id_value: i32, trust_level_value: i32) -> Result<(), DieselError> {
        self.run(move |conn| Self::change_user_trust_level_(conn, user_id_value, trust_level_value)).await
    }
    fn change_user_trust_level_(conn: &PgConnection, user_id_value: i32, trust_level_value: i32) -> Result<(), DieselError> {
        use self::users::dsl::*;
        diesel::update(users.find(user_id_value)).set(trust_level.eq(trust_level_value))
            .execute(conn)
            .map(|k| { assert_eq!(k, 1); })
    }
    pub async fn change_user_banned(&self, user_id_value: i32, banned_value: bool) -> Result<(), DieselError> {
        self.run(move |conn| Self::change_user_banned_(conn, user_id_value, banned_value)).await
    }
    fn change_user_banned_(conn: &PgConnection, user_id_value: i32, banned_value: bool) -> Result<(), DieselError> {
        use self::users::dsl::*;
        diesel::update(users.find(user_id_value)).set(banned.eq(banned_value))
            .execute(conn)
            .map(|k| { assert_eq!(k, 1); })
    }
    pub async fn create_invite_token(&self, invited_by: i32) -> Result<InviteToken, DieselError> {
        self.run(move |conn| Self::create_invite_token_(conn, invited_by)).await
    }
    fn create_invite_token_(conn: &PgConnection, invited_by: i32) -> Result<InviteToken, DieselError> {
        #[derive(Insertable)]
        #[table_name="invite_tokens"]
        struct CreateInviteToken {
            invited_by: i32,
            uuid: i64,
        }
        diesel::insert_into(invite_tokens::table)
            .values(CreateInviteToken {
                uuid: ::rand::random(),
                invited_by
            })
            .get_result(conn)
    }
    pub async fn check_invite_token_exists(&self, uuid_value: Base32) -> bool {
        self.run(move |conn| Self::check_invite_token_exists_(conn, uuid_value)).await
    }
    fn check_invite_token_exists_(conn: &PgConnection, uuid_value: Base32) -> bool {
        use self::invite_tokens::dsl::*;
        use diesel::{select, dsl::exists};
        let uuid_value = uuid_value.into_i64();
        select(exists(invite_tokens.find(uuid_value))).get_result(conn).unwrap_or(false)
    }
    pub async fn consume_invite_token(&self, uuid_value: Base32) -> Result<InviteToken, DieselError> {
        self.run(move |conn| Self::consume_invite_token_(conn, uuid_value)).await
    }
    fn consume_invite_token_(conn: &PgConnection, uuid_value: Base32) -> Result<InviteToken, DieselError> {
        use self::invite_tokens::dsl::*;
        let uuid_value = uuid_value.into_i64();
        diesel::delete(invite_tokens.find(uuid_value)).get_result(conn)
    }
    pub async fn get_recent_users(&self, username: String) -> Result<Vec<User>, DieselError> {
        self.run(move |conn| Self::get_recent_users_(conn, username)).await
    }
    fn get_recent_users_(conn: &PgConnection, username_param: String) -> Result<Vec<User>, DieselError> {
        use self::users::dsl::{self as u, *};
        use self::user_sessions::dsl::*;
        let username_like = Self::escape_like_query(&username_param);
        users
            .inner_join(user_sessions)
            .select((u::id, banned, trust_level, username, password_hash, u::created_at, invited_by, dark_mode, big_mode, identicon))
            .filter(username.like(format!("%{}%", username_like)))
            .order_by(last_seen_at.desc())
            .limit(200)
            .get_results(conn)
    }
    pub async fn get_invite_tree(&self) -> HashMap<i32, Vec<User>> {
        self.run(move |conn| Self::get_invite_tree_(conn)).await
    }
    fn get_invite_tree_(conn: &PgConnection) -> HashMap<i32, Vec<User>> {
        use self::users::dsl::*;
        let mut ret_val: HashMap<i32, Vec<User>> = HashMap::new();
        for user in users.get_results::<User>(conn).unwrap_or(Vec::new()).into_iter() {
            ret_val.entry(user.invited_by.unwrap_or(0)).or_default().push(user)
        }
        ret_val
    }
    pub async fn get_comment_by_id(&self, comment_id_value: i32) -> Result<Comment, DieselError> {
        self.run(move |conn| Self::get_comment_by_id_(conn, comment_id_value)).await
    }
    fn get_comment_by_id_(conn: &PgConnection, comment_id_value: i32) -> Result<Comment, DieselError> {
        use self::comments::dsl::*;
        comments.find(comment_id_value).get_result::<Comment>(conn)
    }
    pub async fn get_comment_info_by_id(&self, comment_id_value: i32, user_id_param: i32) -> Result<CommentInfo, DieselError> {
        self.run(move |conn| Self::get_comment_info_by_id_(conn, comment_id_value, user_id_param)).await
    }
    fn get_comment_info_by_id_(conn: &PgConnection, comment_id_value: i32, user_id_param: i32) -> Result<CommentInfo, DieselError> {
        use self::comments::dsl::*;
        use self::comment_stars::dsl::*;
        use self::comment_flags::dsl::*;
        use self::comment_hides::dsl::*;
        use self::users::dsl::*;
        comments
            .left_outer_join(comment_stars.on(self::comment_stars::dsl::comment_id.eq(self::comments::dsl::id).and(self::comment_stars::dsl::user_id.eq(user_id_param))))
            .left_outer_join(comment_flags.on(self::comment_flags::dsl::comment_id.eq(self::comments::dsl::id).and(self::comment_flags::dsl::user_id.eq(user_id_param))))
            .left_outer_join(comment_hides.on(self::comment_hides::dsl::comment_id.eq(self::comments::dsl::id).and(self::comment_hides::dsl::user_id.eq(user_id_param))))
            .inner_join(users)
            .select((
                self::comments::dsl::id,
                self::comments::dsl::text,
                self::comments::dsl::html,
                self::comments::dsl::visible,
                self::comments::dsl::post_id,
                self::comments::dsl::created_at,
                self::comments::dsl::created_by,
                self::comment_stars::dsl::comment_id.nullable(),
                self::comment_flags::dsl::comment_id.nullable(),
                self::comment_hides::dsl::comment_id.nullable(),
                self::users::dsl::username,
                self::users::dsl::identicon,
            ))
            .filter(visible.eq(true))
            .filter(self::comments::dsl::id.eq(comment_id_value))
            .get_result::<(i32, String, String, bool, i32, NaiveDateTime, i32, Option<i32>, Option<i32>, Option<i32>, String, i32)>(conn)
            .map(|t| tuple_to_comment_info(conn, t))
    }
    pub async fn create_domain(&self, new_domain: NewDomain) -> Result<Domain, DieselError> {
        self.run(move |conn| Self::create_domain_(conn, new_domain)).await
    }
    fn create_domain_(conn: &PgConnection, new_domain: NewDomain) -> Result<Domain, DieselError> {
        use self::domains::dsl::*;
        diesel::insert_into(domains)
            .values(new_domain)
            .get_result(conn)
    }
    pub async fn get_domain_by_id(&self, domain_id_value: i32) -> Result<Domain, DieselError> {
        self.run(move |conn| Self::get_domain_by_id_(conn, domain_id_value)).await
    }
    fn get_domain_by_id_(conn: &PgConnection, domain_id_value: i32) -> Result<Domain, DieselError> {
        use self::domains::dsl::*;
        domains.find(domain_id_value).get_result::<Domain>(conn)
    }
    pub async fn get_domain_by_hostname(&self, hostname_value: &str) -> Result<Domain, DieselError> {
        let hostname_value = hostname_value.to_owned();
        self.run(move |conn| Self::get_domain_by_hostname_(conn, &hostname_value)).await
    }
    fn get_domain_by_hostname_(conn: &PgConnection, mut hostname_value: &str) -> Result<Domain, DieselError> {
        use self::domains::dsl::*;
        use self::domain_synonyms::*;
        if hostname_value.starts_with("www.") {
            hostname_value = &hostname_value[4..];
        }
        if let Ok(domain_synonym) = domain_synonyms::table.filter(from_hostname.eq(hostname_value)).get_result::<DomainSynonym>(conn) {
            domains.filter(id.eq(domain_synonym.to_domain_id)).get_result::<Domain>(conn)
        } else {
            domains.filter(hostname.eq(hostname_value)).get_result::<Domain>(conn)
        }
    }
    pub async fn get_domain_restriction_by_id(&self, domain_id_value: i32) -> Result<DomainRestriction, DieselError> {
        self.run(move |conn| Self::get_domain_restriction_by_id_(conn, domain_id_value)).await
    }
    fn get_domain_restriction_by_id_(conn: &PgConnection, domain_id_value: i32) -> Result<DomainRestriction, DieselError> {
        use self::domain_restrictions::dsl::*;
        domain_restrictions.find(domain_id_value).get_result::<DomainRestriction>(conn)
    }
    pub async fn comment_on_post(&self, new_post: NewComment, body_format: BodyFormat) -> Result<Comment, CreateCommentError> {
        self.run(move |conn| Self::comment_on_post_(conn, new_post, body_format)).await
    }
    fn comment_on_post_(conn: &PgConnection, new_post: NewComment, body_format: BodyFormat) -> Result<Comment, CreateCommentError> {
        #[derive(Insertable)]
        #[table_name="comments"]
        struct CreateComment<'a> {
            text: &'a str,
            html: &'a str,
            post_id: i32,
            created_by: i32,
            visible: bool,
        }
        // TODO: make this configurable
        if Self::get_user_comments_count_today_(conn, new_post.created_by) > 100_000 {
            return Err(CreateCommentError::TooManyComments);
        }
        let html_and_stuff = match body_format {
            BodyFormat::Plain => crate::prettify::prettify_body(&new_post.text, &mut PrettifyData::new(conn, new_post.post_id)),
            BodyFormat::BBCode => crate::prettify::prettify_body_bbcode(&new_post.text, &mut PrettifyData::new(conn, new_post.post_id)),
        };
        Self::update_comment_count_on_post_(conn, new_post.post_id, 1)?;
        Ok(diesel::insert_into(comments::table)
            .values(CreateComment{
                text: &new_post.text,
                html: &html_and_stuff.string,
                post_id: new_post.post_id,
                created_by: new_post.created_by,
                visible: new_post.visible,
            })
            .get_result(conn)?)
    }
    pub async fn update_comment(&self, post_id_value: i32, comment_id_value: i32, text_value: String, body_format: BodyFormat) -> Result<(), DieselError> {
        self.run(move |conn| Self::update_comment_(conn, post_id_value, comment_id_value, text_value, body_format)).await
    }
    fn update_comment_(conn: &PgConnection, post_id_value: i32, comment_id_value: i32, text_value: String, body_format: BodyFormat) -> Result<(), DieselError> {
        let html_and_stuff = match body_format {
            BodyFormat::Plain => crate::prettify::prettify_body(&text_value, &mut PrettifyData::new(conn, post_id_value)),
            BodyFormat::BBCode => crate::prettify::prettify_body_bbcode(&text_value, &mut PrettifyData::new(conn, post_id_value)),
        };
        use self::comments::dsl::*;
        diesel::update(comments.find(comment_id_value))
            .set((
                text.eq(text_value),
                html.eq(&html_and_stuff.string)
                ))
            .execute(conn)
            .map(|_| ())
    }
    pub async fn get_comments_from_post(&self, post_id_param: i32, user_id_param: i32) -> Result<Vec<CommentInfo>, DieselError> {
        self.run(move |conn| Self::get_comments_from_post_(conn, post_id_param, user_id_param)).await
    }
    fn get_comments_from_post_(conn: &PgConnection, post_id_param: i32, user_id_param: i32) -> Result<Vec<CommentInfo>, DieselError> {
        use self::comments::dsl::*;
        use self::comment_stars::dsl::*;
        use self::comment_flags::dsl::*;
        use self::comment_hides::dsl::*;
        use self::users::dsl::*;
        let all: Vec<CommentInfo> = comments
            .left_outer_join(comment_stars.on(self::comment_stars::dsl::comment_id.eq(self::comments::dsl::id).and(self::comment_stars::dsl::user_id.eq(user_id_param))))
            .left_outer_join(comment_flags.on(self::comment_flags::dsl::comment_id.eq(self::comments::dsl::id).and(self::comment_flags::dsl::user_id.eq(user_id_param))))
            .left_outer_join(comment_hides.on(self::comment_hides::dsl::comment_id.eq(self::comments::dsl::id).and(self::comment_hides::dsl::user_id.eq(user_id_param))))
            .inner_join(users)
            .select((
                self::comments::dsl::id,
                self::comments::dsl::text,
                self::comments::dsl::html,
                self::comments::dsl::visible,
                self::comments::dsl::post_id,
                self::comments::dsl::created_at,
                self::comments::dsl::created_by,
                self::comment_stars::dsl::comment_id.nullable(),
                self::comment_flags::dsl::comment_id.nullable(),
                self::comment_hides::dsl::comment_id.nullable(),
                self::users::dsl::username,
                self::users::dsl::identicon,
            ))
            .filter(visible.eq(true))
            .filter(self::comments::dsl::post_id.eq(post_id_param))
            .order_by(self::comments::dsl::created_at)
            .get_results::<(i32, String, String, bool, i32, NaiveDateTime, i32, Option<i32>, Option<i32>, Option<i32>, String, i32)>(conn)?
            .into_iter()
            .map(|t| tuple_to_comment_info(conn, t))
            .collect();
        Ok(all)
    }
    pub async fn search_comments(&self, user_id_param: Option<i32>, after_id_param: Option<i32>, self_user_id: i32) -> Result<Vec<CommentSearchResult>, DieselError> {
        self.run(move |conn| Self::search_comments_(conn, user_id_param, after_id_param, self_user_id)).await
    }
    fn search_comments_(conn: &PgConnection, user_id_param: Option<i32>, after_id_param: Option<i32>, self_user_id: i32) -> Result<Vec<CommentSearchResult>, DieselError> {
        use self::comments::dsl::*;
        use self::comment_stars::dsl::*;
        use self::comment_flags::dsl::*;
        use self::users::dsl::*;
        use self::posts::dsl::*;
        let query = comments
            .left_outer_join(comment_stars.on(self::comment_stars::dsl::comment_id.eq(self::comments::dsl::id).and(self::comment_stars::dsl::user_id.eq(self_user_id))))
            .left_outer_join(comment_flags.on(self::comment_flags::dsl::comment_id.eq(self::comments::dsl::id).and(self::comment_flags::dsl::user_id.eq(self_user_id))))
            .inner_join(users)
            .inner_join(posts)
            .select((
                self::comments::dsl::id,
                self::comments::dsl::html,
                self::posts::dsl::id,
                self::posts::dsl::uuid,
                self::posts::dsl::title,
                self::comments::dsl::created_at,
                self::comments::dsl::created_by,
                self::users::dsl::username,
                self::comment_stars::dsl::comment_id.nullable(),
                self::comment_flags::dsl::comment_id.nullable(),
                self::users::dsl::identicon,
                self::posts::dsl::locked,
            ))
            .filter(self::comments::dsl::visible.eq(true))
            .filter(self::posts::dsl::private.eq(false))
            .filter(self::posts::dsl::visible.eq(true))
            .order_by(self::comments::dsl::id.desc())
            .limit(50);
        let query = match (user_id_param, after_id_param) {
            (Some(user_id_param), Some(after_id_param)) => {
                query
                    .filter(self::comments::dsl::created_by.eq(user_id_param))
                    .filter(self::comments::dsl::id.lt(after_id_param))
                    .into_boxed()
            }
            (Some(user_id_param), None) => {
                query
                    .filter(self::comments::dsl::created_by.eq(user_id_param))
                    .into_boxed()
            }
            (None, Some(after_id_param)) => {
                query
                    .filter(self::comments::dsl::id.lt(after_id_param))
                    .into_boxed()
            }
            (None, None) => query.into_boxed()
        };
        let all: Vec<CommentSearchResult> = query
            .get_results::<(i32, String, i32, Base32, String, NaiveDateTime, i32, String, Option<i32>, Option<i32>, i32, bool)>(conn)?
            .into_iter()
            .map(|t| tuple_to_comment_search_results(conn, t))
            .collect();
        Ok(all)
    }
    pub async fn get_post_info_from_comment(&self, comment_id_param: i32) -> Result<PostInfo, DieselError> {
        self.run(move |conn| Self::get_post_info_from_comment_(conn, comment_id_param)).await
    }
    fn get_post_info_from_comment_(conn: &PgConnection, comment_id_param: i32) -> Result<PostInfo, DieselError> {
        use self::posts::dsl::{self as p, *};
        use self::stars::dsl::{self as s, *};
        use self::flags::dsl::{self as f, *};
        use self::post_hides::dsl::{self as ph, *};
        use self::users::dsl::{self as u, *};
        use self::comments::dsl::{self as c, *};
        use self::comment_readpoints::{self as cr, *};
        // This is a bunch of duplicate code.
        let mut data = PrettifyData::new(conn, 0);
        Ok(tuple_to_post_info_logged_out(&mut data, posts
            .inner_join(users)
            .inner_join(comments)
            .select((
                p::id,
                p::uuid,
                p::title,
                p::title_html,
                p::url,
                p::visible,
                p::private,
                p::initial_stellar_time,
                p::score,
                p::comment_count,
                p::blog_post,
                p::created_at,
                p::submitted_by,
                p::excerpt,
                p::excerpt_html,
                u::username,
                p::banner_title,
                p::banner_desc,
                p::noindex,
                p::locked,
                p::anon,
            ))
            .filter(self::comments::dsl::id.eq(comment_id_param))
            .first::<(i32, Base32, String, Option<String>, Option<String>, bool, bool, i32, i32, i32, bool, NaiveDateTime, i32, Option<String>, Option<String>, String, Option<String>, Option<String>, bool, bool, bool)>(conn)?, Self::get_current_stellar_time_(conn)))
    }
    pub async fn get_post_starred_by(&self, post_id_param: i32) -> Result<Vec<String>, DieselError> {
        self.run(move |conn| Self::get_post_starred_by_(conn, post_id_param)).await
    }
    fn get_post_starred_by_(conn: &PgConnection, post_id_param: i32) -> Result<Vec<String>, DieselError> {
        use self::stars::dsl::*;
        use self::users::dsl::*;
        let all: Vec<String> = stars
            .inner_join(users)
            .select((
                self::users::dsl::username,
            ))
            .filter(self::stars::dsl::post_id.eq(post_id_param))
            .limit(50)
            .get_results::<(String,)>(conn)?
            .into_iter()
            .map(|(t,)| t)
            .collect();
        Ok(all)
    }
    pub async fn get_comment_starred_by(&self, comment_id_param: i32) -> Result<Vec<String>, DieselError> {
        self.run(move |conn| Self::get_comment_starred_by_(conn, comment_id_param)).await
    }
    fn get_comment_starred_by_(conn: &PgConnection, comment_id_param: i32) -> Result<Vec<String>, DieselError> {
        use self::comment_stars::dsl::*;
        use self::users::dsl::*;
        let all: Vec<String> = comment_stars
            .inner_join(users)
            .select((
                self::users::dsl::username,
            ))
            .filter(self::comment_stars::dsl::comment_id.eq(comment_id_param))
            .limit(50)
            .get_results::<(String,)>(conn)?
            .into_iter()
            .map(|(t,)| t)
            .collect();
        Ok(all)
    }
    pub async fn get_tag_by_name(&self, name_param: &str) -> Result<Tag, DieselError> {
        let name_param = name_param.to_owned();
        self.run(move |conn| Self::get_tag_by_name_(conn, &name_param)).await
    }
    fn get_tag_by_name_(conn: &PgConnection, name_param: &str) -> Result<Tag, DieselError> {
        use self::tags::dsl::*;
        tags
            .filter(name.eq(name_param))
            .get_result::<Tag>(conn)
    }
    pub async fn create_or_update_tag(&self, new_tag: &NewTag) -> Result<Tag, DieselError> {
        let new_tag = new_tag.clone();
        self.run(move |conn| Self::create_or_update_tag_(conn, &new_tag)).await
    }
    fn create_or_update_tag_(conn: &PgConnection, new_tag: &NewTag) -> Result<Tag, DieselError> {
        #[derive(Insertable)]
        #[table_name="tags"]
        struct CreateTag<'a> {
            name: &'a str,
            description: Option<&'a str>,
        }
        if let Ok(tag) = Self::get_tag_by_name_(conn, &new_tag.name) {
            use self::tags::dsl::*;
            diesel::update(tags.find(tag.id))
                .set(description.eq(&new_tag.description))
                .get_result(conn)
        } else {
            diesel::insert_into(tags::table)
                .values(CreateTag {
                    name: &new_tag.name,
                    description: new_tag.description.as_ref().map(|x| &x[..]),
                })
                .get_result(conn)
        }
    }
    pub async fn add_domain_synonym(&self, new_domain_synonym: DomainSynonym) -> Result<(), DieselError> {
        self.run(move |conn| Self::add_domain_synonym_(conn, new_domain_synonym)).await
    }
    fn add_domain_synonym_(conn: &PgConnection, new_domain_synonym: DomainSynonym) -> Result<(), DieselError> {
        use self::posts::dsl::*;
        use self::domain_synonyms::dsl::*;
        use self::domains::dsl::*;
        if let Ok(old_domain) = Self::get_domain_by_hostname_(conn, &new_domain_synonym.from_hostname) {
            if old_domain.hostname == new_domain_synonym.from_hostname {
                diesel::update(posts.filter(domain_id.eq(old_domain.id)))
                    .set(domain_id.eq(new_domain_synonym.to_domain_id))
                    .execute(conn)?;
                diesel::delete(domains.find(old_domain.id))
                    .execute(conn)?;
            }
        }
        if let Ok(old_domain_synonym) = domain_synonyms.find(&new_domain_synonym.from_hostname).get_result::<DomainSynonym>(conn) {
            diesel::update(domain_synonyms.find(old_domain_synonym.from_hostname))
                .set(to_domain_id.eq(new_domain_synonym.to_domain_id))
                .execute(conn)
                .map(|_| ())
        } else {
            diesel::insert_into(domain_synonyms)
                .values(new_domain_synonym)
                .execute(conn)
                .map(|_| ())
        }
    }
    pub async fn get_all_domain_synonyms(&self) -> Result<Vec<DomainSynonymInfo>, DieselError> {
        self.run(move |conn| Self::get_all_domain_synonyms_(conn)).await
    }
    fn get_all_domain_synonyms_(conn: &PgConnection) -> Result<Vec<DomainSynonymInfo>, DieselError> {
        use self::domain_synonyms::dsl::*;
        use self::domains::dsl::*;
        domain_synonyms
            .inner_join(domains)
            .select((from_hostname, hostname))
            .get_results::<DomainSynonymInfo>(conn)
    }
    pub async fn add_blocked_regex(&self, new_blocked_regex: NewBlockedRegex) -> Result<(), DieselError> {
        self.run(move |conn| Self::add_blocked_regex_(conn, new_blocked_regex)).await
    }
    fn add_blocked_regex_(conn: &PgConnection, new_blocked_regex: NewBlockedRegex) -> Result<(), DieselError> {
        use self::blocked_regexes::dsl::*;
        diesel::insert_into(blocked_regexes)
            .values(new_blocked_regex)
            .execute(conn)
            .map(|_| ())
    }
    pub async fn delete_blocked_regex(&self, id: i32) -> Result<(), DieselError> {
        self.run(move |conn| Self::delete_blocked_regex_(conn, id)).await
    }
    fn delete_blocked_regex_(conn: &PgConnection, id_: i32) -> Result<(), DieselError> {
        use self::blocked_regexes::dsl::*;
        diesel::delete(blocked_regexes.filter(id.eq(id_)))
            .execute(conn)?;
        Ok(())
    }
    pub async fn get_all_blocked_regexes(&self) -> Result<Vec<BlockedRegex>, DieselError> {
        self.run(move |conn| Self::get_all_blocked_regexes_(conn)).await
    }
    fn get_all_blocked_regexes_(conn: &PgConnection) -> Result<Vec<BlockedRegex>, DieselError> {
        use self::blocked_regexes::dsl::*;
        blocked_regexes
            .get_results::<BlockedRegex>(conn)
    }
    pub async fn get_all_tags(&self) -> Result<Vec<Tag>, DieselError> {
        self.run(move |conn| Self::get_all_tags_(conn)).await
    }
    fn get_all_tags_(conn: &PgConnection) -> Result<Vec<Tag>, DieselError> {
        use self::tags::dsl::*;
        let mut t = tags.get_results::<Tag>(conn)?;
        t.sort_by(|a, b| {
            // place all-number tags last
            if a.name.as_bytes()[0] < b'a' && b.name.as_bytes()[0] >= b'a' { return Ordering::Greater };
            if b.name.as_bytes()[0] < b'a' && a.name.as_bytes()[0] >= b'a' { return Ordering::Less };
            a.name.cmp(&b.name)
        });
        Ok(t)
    }
    pub async fn search_domains(&self, query: String) -> Result<Vec<Domain>, DieselError> {
        self.run(move |conn| Self::search_domains_(conn, query)).await
    }
    fn search_domains_(conn: &PgConnection, query: String) -> Result<Vec<Domain>, DieselError> {
        use self::domains::dsl::*;
        let query = Self::escape_like_query(&query);
        let mut t = domains
            .filter(hostname.like(format!("%{}%", &query)))
            .limit(1000)
            .get_results::<Domain>(conn)?;
        t.sort_by(|a, b| {
            // place all-number domains last
            if a.hostname.as_bytes()[0] < b'a' && b.hostname.as_bytes()[0] >= b'a' { return Ordering::Greater };
            if b.hostname.as_bytes()[0] < b'a' && a.hostname.as_bytes()[0] >= b'a' { return Ordering::Less };
            a.hostname.cmp(&b.hostname)
        });
        Ok(t)
    }
    fn escape_like_query(query: &str) -> String {
        // https://www.postgresql.org/docs/8.3/functions-matching.html
        query.chars().flat_map(|x| {
            match x {
                '\\' | '%' | '_' => vec!['\\', x],
                _ => vec![x],
            }
        }).collect()
    }
    pub async fn set_dark_mode(&self, user_id_value: i32, dark_mode_value: bool) -> Result<(), DieselError> {
        self.run(move |conn| Self::set_dark_mode_(conn, user_id_value, dark_mode_value)).await
    }
    fn set_dark_mode_(conn: &PgConnection, user_id_value: i32, dark_mode_value: bool) -> Result<(), DieselError> {
        use self::users::dsl::*;
        diesel::update(users.find(user_id_value))
            .set(dark_mode.eq(dark_mode_value))
            .execute(conn)
            .map(|_| ())
    }
    pub async fn set_big_mode(&self, user_id_value: i32, dark_mode_value: bool) -> Result<(), DieselError> {
        self.run(move |conn| Self::set_big_mode_(conn, user_id_value, dark_mode_value)).await
    }
    fn set_big_mode_(conn: &PgConnection, user_id_value: i32, big_mode_value: bool) -> Result<(), DieselError> {
        use self::users::dsl::*;
        diesel::update(users.find(user_id_value))
            .set(big_mode.eq(big_mode_value))
            .execute(conn)
            .map(|_| ())
    }
    pub async fn get_mod_log_recent(&self) -> Result<Vec<ModerationInfo>, DieselError> {
        self.run(move |conn| Self::get_mod_log_recent_(conn)).await
    }
    fn get_mod_log_recent_(conn: &PgConnection) -> Result<Vec<ModerationInfo>, DieselError> {
        use self::moderation::dsl::*;
        Ok(moderation
            .inner_join(users::table)
            .select((
                id,
                payload,
                created_at,
                created_by,
                self::users::dsl::username,
            ))
            .order_by(id.desc())
            .limit(50)
            .get_results::<(i32, json::Value, NaiveDateTime, i32, String)>(conn)?
            .into_iter()
            .map(|t| tuple_to_moderation(t))
            .collect()
        )
    }
    pub async fn get_mod_log_starting_with(&self, starting_with_id: i32) -> Result<Vec<ModerationInfo>, DieselError> {
        self.run(move |conn| Self::get_mod_log_starting_with_(conn, starting_with_id)).await
    }
    fn get_mod_log_starting_with_(conn: &PgConnection, starting_with_id: i32) -> Result<Vec<ModerationInfo>, DieselError> {
        use self::moderation::dsl::*;
        Ok(moderation
            .inner_join(users::table)
            .select((
                id,
                payload,
                created_at,
                created_by,
                self::users::dsl::username,
            ))
            .filter(id.lt(starting_with_id))
            .order_by(id.desc())
            .limit(50)
            .get_results::<(i32, json::Value, NaiveDateTime, i32, String)>(conn)?
            .into_iter()
            .map(|t| tuple_to_moderation(t))
            .collect()
        )
    }
    pub async fn mod_log_edit_comment(
        &self,
        user_id_value: i32,
        comment_id_value: i32,
        post_uuid_value: Base32,
        old_text_value: String,
        new_text_value: String,
    ) -> Result<(), DieselError> {
        self.run(move |conn| Self::mod_log_edit_comment_(conn, user_id_value, comment_id_value, post_uuid_value, old_text_value, new_text_value)).await
    }
    fn mod_log_edit_comment_(
        conn: &PgConnection,
        user_id_value: i32,
        comment_id_value: i32,
        post_uuid_value: Base32,
        old_text_value: String,
        new_text_value: String,
    ) -> Result<(), DieselError> {
        diesel::insert_into(moderation::table)
            .values(CreateModeration{
                payload: json!{{
                    "type": "edit_comment",
                    "comment_id": comment_id_value,
                    "post_uuid": post_uuid_value,
                    "old_text": old_text_value,
                    "new_text": new_text_value,
                }},
                created_by: user_id_value,
            })
            .execute(conn)
            .map(|_| ())
    }
    pub async fn mod_log_edit_post(
        &self,
        user_id_value: i32,
        post_uuid_value: Base32,
        old_title_value: String,
        new_title_value: String,
        old_url_value: String,
        new_url_value: String,
        old_excerpt_value: String,
        new_excerpt_value: String,
    ) -> Result<(), DieselError> {
        self.run(move |conn| Self::mod_log_edit_post_(conn, user_id_value, post_uuid_value, old_title_value, new_title_value, old_url_value, new_url_value, old_excerpt_value, new_excerpt_value)).await
    }
    fn mod_log_edit_post_(
        conn: &PgConnection,
        user_id_value: i32,
        post_uuid_value: Base32,
        old_title_value: String,
        new_title_value: String,
        old_url_value: String,
        new_url_value: String,
        old_excerpt_value: String,
        new_excerpt_value: String,
    ) -> Result<(), DieselError> {
        diesel::insert_into(moderation::table)
            .values(CreateModeration{
                payload: json!{{
                    "type": "edit_post",
                    "post_uuid": post_uuid_value,
                    "old_title": old_title_value,
                    "new_title": new_title_value,
                    "old_url": old_url_value,
                    "new_url": new_url_value,
                    "old_excerpt": old_excerpt_value,
                    "new_excerpt": new_excerpt_value,
                }},
                created_by: user_id_value,
            })
            .execute(conn)
            .map(|_| ())
    }
    pub async fn mod_log_delete_comment(
        &self,
        user_id_value: i32,
        comment_id_value: i32,
        post_uuid_value: Base32,
        old_text_value: String,
    ) -> Result<(), DieselError> {
        self.run(move |conn| Self::mod_log_delete_comment_(conn, user_id_value, comment_id_value, post_uuid_value, old_text_value)).await
    }
    fn mod_log_delete_comment_(
        conn: &PgConnection,
        user_id_value: i32,
        comment_id_value: i32,
        post_uuid_value: Base32,
        old_text_value: String,
    ) -> Result<(), DieselError> {
        diesel::insert_into(moderation::table)
            .values(CreateModeration{
                payload: json!{{
                    "type": "delete_comment",
                    "comment_id": comment_id_value,
                    "post_uuid": post_uuid_value,
                    "old_text": old_text_value,
                }},
                created_by: user_id_value,
            })
            .execute(conn)
            .map(|_| ())
    }
    pub async fn mod_log_delete_post(
        &self,
        user_id_value: i32,
        post_uuid_value: Base32,
        old_title_value: String,
        old_url_value: String,
        old_excerpt_value: String,
    ) -> Result<(), DieselError> {
        self.run(move |conn| Self::mod_log_delete_post_(conn, user_id_value, post_uuid_value, old_title_value, old_url_value, old_excerpt_value)).await
    }
    fn mod_log_delete_post_(
        conn: &PgConnection,
        user_id_value: i32,
        post_uuid_value: Base32,
        old_title_value: String,
        old_url_value: String,
        old_excerpt_value: String,
    ) -> Result<(), DieselError> {
        diesel::insert_into(moderation::table)
            .values(CreateModeration{
                payload: json!{{
                    "type": "delete_post",
                    "post_uuid": post_uuid_value,
                    "old_title": old_title_value,
                    "old_url": old_url_value,
                    "old_excerpt": old_excerpt_value,
                }},
                created_by: user_id_value,
            })
            .execute(conn)
            .map(|_| ())
    }
    pub async fn mod_log_approve_comment(
        &self,
        user_id_value: i32,
        comment_id_value: i32,
        post_uuid_value: Base32,
        new_text_value: String,
    ) -> Result<(), DieselError> {
        self.run(move |conn| Self::mod_log_approve_comment_(conn, user_id_value, comment_id_value, post_uuid_value, new_text_value)).await
    }
    fn mod_log_approve_comment_(
        conn: &PgConnection,
        user_id_value: i32,
        comment_id_value: i32,
        post_uuid_value: Base32,
        new_text_value: String,
    ) -> Result<(), DieselError> {
        diesel::insert_into(moderation::table)
            .values(CreateModeration{
                payload: json!{{
                    "type": "approve_comment",
                    "comment_id": comment_id_value,
                    "post_uuid": post_uuid_value,
                    "new_text": new_text_value,
                }},
                created_by: user_id_value,
            })
            .execute(conn)
            .map(|_| ())
    }
    pub async fn mod_log_approve_post(
        &self,
        user_id_value: i32,
        post_uuid_value: Base32,
        old_title_value: String,
        old_url_value: String,
        old_excerpt_value: String,
    ) -> Result<(), DieselError> {
        self.run(move |conn| Self::mod_log_approve_post_(conn, user_id_value, post_uuid_value, old_title_value, old_url_value, old_excerpt_value)).await
    }
    fn mod_log_approve_post_(
        conn: &PgConnection,
        user_id_value: i32,
        post_uuid_value: Base32,
        new_title_value: String,
        new_url_value: String,
        new_excerpt_value: String,
    ) -> Result<(), DieselError> {
        diesel::insert_into(moderation::table)
            .values(CreateModeration{
                payload: json!{{
                    "type": "approve_post",
                    "post_uuid": post_uuid_value,
                    "new_title": new_title_value,
                    "new_url": new_url_value,
                    "new_excerpt": new_excerpt_value,
                }},
                created_by: user_id_value,
            })
            .execute(conn)
            .map(|_| ())
    }
    pub async fn mod_log_poll_post(
        &self,
        user_id_value: i32,
        post_uuid_value: Base32,
        poll_title_value: String,
        poll_value: i32,
    ) -> Result<(), DieselError> {
        self.run(move |conn| Self::mod_log_poll_post_(conn, user_id_value, post_uuid_value, poll_title_value, poll_value)).await
    }
    fn mod_log_poll_post_(
        conn: &PgConnection,
        user_id_value: i32,
        post_uuid_value: Base32,
        poll_title_value: String,
        poll_value: i32,
    ) -> Result<(), DieselError> {
        diesel::insert_into(moderation::table)
            .values(CreateModeration{
                payload: json!{{
                    "type": "poll_post",
                    "post_uuid": post_uuid_value,
                    "poll_title": poll_title_value,
                    "poll": poll_value,
                }},
                created_by: user_id_value,
            })
            .execute(conn)
            .map(|_| ())
    }
    pub async fn mod_log_close_poll(
        &self,
        user_id_value: i32,
        post_uuid_value: Base32,
        poll_value: i32,
    ) -> Result<(), DieselError> {
        self.run(move |conn| Self::mod_log_close_poll_(conn, user_id_value, post_uuid_value, poll_value)).await
    }
    fn mod_log_close_poll_(
        conn: &PgConnection,
        user_id_value: i32,
        post_uuid_value: Base32,
        poll_value: i32,
    ) -> Result<(), DieselError> {
        diesel::insert_into(moderation::table)
            .values(CreateModeration{
                payload: json!{{
                    "type": "poll_close",
                    "post_uuid": post_uuid_value,
                    "poll": poll_value,
                }},
                created_by: user_id_value,
            })
            .execute(conn)
            .map(|_| ())
    }
    pub async fn mod_log_banner_post(
        &self,
        user_id_value: i32,
        post_uuid_value: Base32,
        banner_title_value: String,
        banner_desc_value: String,
    ) -> Result<(), DieselError> {
        self.run(move |conn| Self::mod_log_banner_post_(conn, user_id_value, post_uuid_value, banner_title_value, banner_desc_value)).await
    }
    fn mod_log_banner_post_(
        conn: &PgConnection,
        user_id_value: i32,
        post_uuid_value: Base32,
        banner_title_value: String,
        banner_desc_value: String,
    ) -> Result<(), DieselError> {
        diesel::insert_into(moderation::table)
            .values(CreateModeration{
                payload: json!{{
                    "type": "banner_post",
                    "post_uuid": post_uuid_value,
                    "banner_title": banner_title_value,
                    "banner_desc": banner_desc_value,
                }},
                created_by: user_id_value,
            })
            .execute(conn)
            .map(|_| ())
    }
    pub async fn mod_log_lock(
        &self,
        user_id_value: i32,
        post_uuid_value: Base32,
        locked_value: bool,
    ) -> Result<(), DieselError> {
        self.run(move |conn| Self::mod_log_lock_(conn, user_id_value, post_uuid_value, locked_value)).await
    }
    fn mod_log_lock_(
        conn: &PgConnection,
        user_id_value: i32,
        post_uuid_value: Base32,
        locked_value: bool,
    ) -> Result<(), DieselError> {
        diesel::insert_into(moderation::table)
            .values(CreateModeration{
                payload: json!{{
                    "type": "lock",
                    "post_uuid": post_uuid_value,
                    "locked": locked_value,
                }},
                created_by: user_id_value,
            })
            .execute(conn)
            .map(|_| ())
    }
    pub async fn mod_log_noindex(
        &self,
        user_id_value: i32,
        post_uuid_value: Base32,
        noindex_value: bool,
    ) -> Result<(), DieselError> {
        self.run(move |conn| Self::mod_log_noindex_(conn, user_id_value, post_uuid_value, noindex_value)).await
    }
    fn mod_log_noindex_(
        conn: &PgConnection,
        user_id_value: i32,
        post_uuid_value: Base32,
        noindex_value: bool,
    ) -> Result<(), DieselError> {
        diesel::insert_into(moderation::table)
            .values(CreateModeration{
                payload: json!{{
                    "type": "noindex",
                    "post_uuid": post_uuid_value,
                    "noindex": noindex_value,
                }},
                created_by: user_id_value,
            })
            .execute(conn)
            .map(|_| ())
    }
    pub async fn find_moderated_posts(&self, user_id_param: i32) -> Result<Vec<PostInfo>, DieselError> {
        self.run(move |conn| Self::find_moderated_posts_(conn, user_id_param)).await
    }
    fn find_moderated_posts_(conn: &PgConnection, user_id_param: i32) -> Result<Vec<PostInfo>, DieselError> {
        use self::posts::dsl::{self as p, *};
        use self::stars::dsl::{self as s, *};
        use self::flags::dsl::{self as f, *};
        use self::post_hides::dsl::{self as ph, *};
        use self::users::dsl::{self as u, *};
        use self::comment_readpoints::{self as cr, *};
        let mut data = PrettifyData::new(conn, 0);
        let mut all: Vec<PostInfo> = posts
            .left_outer_join(stars.on(s::post_id.eq(p::id).and(s::user_id.eq(user_id_param))))
            .left_outer_join(flags.on(f::post_id.eq(p::id).and(f::user_id.eq(user_id_param))))
            .left_outer_join(post_hides.on(ph::post_id.eq(p::id).and(ph::user_id.eq(user_id_param))))
            .left_outer_join(cr::table.on(cr::post_id.eq(p::id).and(cr::user_id.eq(user_id_param))))
            .inner_join(users)
            .select((
                p::id,
                p::uuid,
                p::title,
                p::title_html,
                p::url,
                p::visible,
                p::private,
                p::initial_stellar_time,
                p::score,
                p::comment_count,
                cr::comment_readpoint.nullable(),
                p::blog_post,
                p::created_at,
                p::submitted_by,
                p::excerpt,
                p::excerpt_html,
                s::post_id.nullable(),
                f::post_id.nullable(),
                ph::post_id.nullable(),
                u::username,
                p::banner_title,
                p::banner_desc,
                p::noindex,
                p::locked,
                p::anon,
            ))
            .filter(visible.eq(false))
            .filter(rejected.eq(false))
            .filter(self::users::dsl::trust_level.gt(-2))
            .order_by(self::posts::dsl::created_at.asc())
            .limit(50)
            .get_results::<(i32, Base32, String, Option<String>, Option<String>, bool, bool, i32, i32, i32, Option<i32>, bool, NaiveDateTime, i32, Option<String>, Option<String>, Option<i32>, Option<i32>, Option<i32>, String, Option<String>, Option<String>, bool, bool, bool)>(conn)?
            .into_iter()
            .map(|t| tuple_to_post_info(&mut data, t, Self::get_current_stellar_time_(conn)))
            .collect();
        all.sort_by_key(|info| OrderedFloat(-info.hotness));
        all.truncate(10);
        Ok(all)
    }
    pub async fn find_moderated_comments(&self, user_id_param: i32) -> Result<Vec<CommentInfo>, DieselError> {
        self.run(move |conn| Self::find_moderated_comments_(conn, user_id_param)).await
    }
    fn find_moderated_comments_(conn: &PgConnection, user_id_param: i32) -> Result<Vec<CommentInfo>, DieselError> {
        use self::comments::dsl::*;
        use self::comment_stars::dsl::*;
        use self::comment_flags::dsl::*;
        use self::comment_hides::dsl::*;
        use self::users::dsl::*;
        let mut all: Vec<CommentInfo> = comments
            .left_outer_join(comment_stars.on(self::comment_stars::dsl::comment_id.eq(self::comments::dsl::id).and(self::comment_stars::dsl::user_id.eq(user_id_param))))
            .left_outer_join(comment_flags.on(self::comment_flags::dsl::comment_id.eq(self::comments::dsl::id).and(self::comment_flags::dsl::user_id.eq(user_id_param))))
            .left_outer_join(comment_hides.on(self::comment_hides::dsl::comment_id.eq(self::comments::dsl::id).and(self::comment_hides::dsl::user_id.eq(user_id_param))))
            .inner_join(users)
            .select((
                self::comments::dsl::id,
                self::comments::dsl::text,
                self::comments::dsl::html,
                self::comments::dsl::visible,
                self::comments::dsl::post_id,
                self::comments::dsl::created_at,
                self::comments::dsl::created_by,
                self::comment_stars::dsl::comment_id.nullable(),
                self::comment_flags::dsl::comment_id.nullable(),
                self::comment_hides::dsl::comment_id.nullable(),
                self::users::dsl::username,
                self::users::dsl::identicon,
            ))
            .filter(visible.eq(false))
            .filter(rejected.eq(false))
            .filter(self::users::dsl::trust_level.gt(-2))
            .order_by(self::comments::dsl::created_at.asc())
            .limit(50)
            .get_results::<(i32, String, String, bool, i32, NaiveDateTime, i32, Option<i32>, Option<i32>, Option<i32>, String, i32)>(conn)?
            .into_iter()
            .map(|t| tuple_to_comment_info(conn, t))
            .collect();
        all.truncate(10);
        Ok(all)
    }
    pub async fn approve_post(&self, post_id_value: i32) -> Result<(), DieselError> {
        self.run(move |conn| Self::approve_post_(conn, post_id_value)).await
    }
    fn approve_post_(conn: &PgConnection, post_id_value: i32) -> Result<(), DieselError> {
        use self::posts::dsl::*;
        diesel::update(posts.find(post_id_value))
            .set((
                visible.eq(true),
                initial_stellar_time.eq(Self::get_current_stellar_time_(conn)),
            ))
            .execute(conn)?;
        Ok(())
    }
    pub async fn lock_post(&self, post_id_value: i32, locked: bool) -> Result<(), DieselError> {
        self.run(move |conn| Self::lock_post_(conn, post_id_value, locked)).await
    }
    fn lock_post_(conn: &PgConnection, post_id_value: i32, locked_value: bool) -> Result<(), DieselError> {
        use self::posts::dsl::*;
        diesel::update(posts.find(post_id_value))
            .set((
                locked.eq(locked_value),
            ))
            .execute(conn)?;
        Ok(())
    }
    pub async fn noindex_post(&self, post_id_value: i32, noindex: bool) -> Result<(), DieselError> {
        self.run(move |conn| Self::noindex_post_(conn, post_id_value, noindex)).await
    }
    fn noindex_post_(conn: &PgConnection, post_id_value: i32, noindex_value: bool) -> Result<(), DieselError> {
        use self::posts::dsl::*;
        diesel::update(posts.find(post_id_value))
            .set((
                noindex.eq(noindex_value),
            ))
            .execute(conn)?;
        Ok(())
    }
    pub async fn banner_post(&self, post_id_value: i32, banner_title_value: Option<String>, banner_desc_value: Option<String>) -> Result<(), DieselError> {
        self.run(move |conn| Self::banner_post_(conn, post_id_value, banner_title_value, banner_desc_value)).await
    }
    fn banner_post_(conn: &PgConnection, post_id_value: i32, banner_title_value: Option<String>, banner_desc_value: Option<String>) -> Result<(), DieselError> {
        use self::posts::dsl::*;
        diesel::update(posts.find(post_id_value))
            .set((
                banner_title.eq(banner_title_value),
                banner_desc.eq(banner_desc_value),
            ))
            .execute(conn)?;
        Ok(())
    }
    pub async fn approve_comment(&self, comment_id_value: i32) -> Result<(), DieselError> {
        self.run(move |conn| Self::approve_comment_(conn, comment_id_value)).await
    }
    fn approve_comment_(conn: &PgConnection, comment_id_value: i32) -> Result<(), DieselError> {
        use self::comments::dsl::*;
        diesel::update(comments.find(comment_id_value))
            .set((
                visible.eq(true),
            ))
            .execute(conn)?;
        Ok(())
    }
    pub async fn invisible_post(&self, post_id_value: i32) -> Result<(), DieselError> {
        self.run(move |conn| Self::invisible_post_(conn, post_id_value)).await
    }
    fn invisible_post_(conn: &PgConnection, post_id_value: i32) -> Result<(), DieselError> {
        use self::posts::dsl::*;
        diesel::update(posts.find(post_id_value))
            .set((
                visible.eq(false),
            ))
            .execute(conn)?;
        Ok(())
    }
    pub async fn invisible_comment(&self, comment_id_value: i32) -> Result<(), DieselError> {
        self.run(move |conn| Self::invisible_comment_(conn, comment_id_value)).await
    }
    fn invisible_comment_(conn: &PgConnection, comment_id_value: i32) -> Result<(), DieselError> {
        use self::comments::dsl::*;
        diesel::update(comments.find(comment_id_value))
            .set((
                visible.eq(false),
            ))
            .execute(conn)?;
        Ok(())
    }
    pub async fn delete_post(&self, post_id_value: i32) -> Result<(), DieselError> {
        self.run(move |conn| Self::delete_post_(conn, post_id_value)).await
    }
    fn delete_post_(conn: &PgConnection, post_id_value: i32) -> Result<(), DieselError> {
        use self::posts::dsl::*;
        diesel::update(posts.find(post_id_value))
            .set((
                visible.eq(false),
                rejected.eq(true),
            ))
            .execute(conn)?;
        Ok(())
    }
    pub async fn delete_comment(&self, comment_id_value: i32) -> Result<(), DieselError> {
        self.run(move |conn| Self::delete_comment_(conn, comment_id_value)).await
    }
    fn delete_comment_(conn: &PgConnection, comment_id_value: i32) -> Result<(), DieselError> {
        use self::comments::dsl::*;
        diesel::update(comments.find(comment_id_value))
            .set((
                visible.eq(false),
                rejected.eq(true),
            ))
            .execute(conn)?;
        Ok(())
    }
    pub async fn user_has_received_a_star(&self, user_id_param: i32) -> bool {
        self.run(move |conn| Self::user_has_received_a_star_(conn, user_id_param)).await
    }
    fn user_has_received_a_star_(conn: &PgConnection, user_id_param: i32) -> bool {
        use self::stars::dsl::*;
        use self::posts::dsl::*;
        use self::comments::dsl::*;
        use self::comment_stars::dsl::*;
        use self::users::dsl::*;
        use diesel::{select, dsl::exists};
        let post_star = stars
            .inner_join(posts)
            .filter(self::posts::dsl::submitted_by.eq(user_id_param))
            .inner_join(users.on(self::stars::dsl::user_id.eq(self::users::dsl::id).and(trust_level.ge(1))));
        let comment_star = comment_stars
            .inner_join(comments)
            .filter(self::comments::dsl::created_by.eq(user_id_param))
            .inner_join(users.on(self::comment_stars::dsl::user_id.eq(self::users::dsl::id).and(trust_level.ge(1))));
        select(exists(post_star)).get_result(conn).unwrap_or(false) ||
            select(exists(comment_star)).get_result(conn).unwrap_or(false)
    }
    pub async fn maximum_post_id(&self) -> i32 {
        self.run(move |conn| Self::maximum_post_id_(conn)).await
    }
    fn maximum_post_id_(conn: &PgConnection) -> i32 {
        use self::posts::dsl::*;
        use diesel::dsl::max;
        posts.select(max(id)).get_result::<Option<i32>>(conn).unwrap_or(Some(0)).unwrap_or(0)
    }
    pub async fn get_post_by_uuid(&self, post_id_value: Base32) -> Result<Post, DieselError> {
        self.run(move |conn| Self::get_post_by_uuid_(conn, post_id_value)).await
    }
    fn get_post_by_uuid_(conn: &PgConnection, post_id_value: Base32) -> Result<Post, DieselError> {
        use self::posts::dsl::*;
        posts.filter(uuid.eq(post_id_value.into_i64())).get_result::<Post>(conn)
    }
    pub async fn get_legacy_comment_info_from_post(&self, post_id_param: i32, user_id_param: i32) -> Result<Vec<LegacyCommentInfo>, DieselError> {
        self.run(move |conn| Self::get_legacy_comment_info_from_post_(conn, post_id_param, user_id_param)).await
    }
    fn get_legacy_comment_info_from_post_(conn: &PgConnection, post_id_param: i32, _user_id_param: i32) -> Result<Vec<LegacyCommentInfo>, DieselError> {
        use self::legacy_comments::dsl;
        let all: Vec<LegacyCommentInfo> = dsl::legacy_comments
            .filter(dsl::post_id.eq(post_id_param))
            .order_by(dsl::created_at)
            .get_results::<LegacyComment>(conn)?
            .into_iter().map(|LegacyComment { id, post_id, author, text, html, created_at, updated_at } | {
                let created_at_relative = relative_date(&created_at);
                LegacyCommentInfo {
                    id, post_id, author, text, html, created_at, updated_at,
                    created_at_relative,
                }
            }).collect();
        Ok(all)
    }
    pub async fn get_legacy_comment_by_id(&self, legacy_comment_id_value: i32) -> Result<LegacyComment, DieselError> {
        self.run(move |conn| Self::get_legacy_comment_by_id_(conn, legacy_comment_id_value)).await
    }
    fn get_legacy_comment_by_id_(conn: &PgConnection, legacy_comment_id_value: i32) -> Result<LegacyComment, DieselError> {
        use self::legacy_comments::dsl::*;
        legacy_comments.find(legacy_comment_id_value).get_result::<LegacyComment>(conn)
    }
    pub async fn update_legacy_comment(&self, post_id_value: i32, legacy_comment_id_value: i32, text_value: String, body_format: BodyFormat) -> Result<(), DieselError> {
        self.run(move |conn| Self::update_legacy_comment_(conn, post_id_value, legacy_comment_id_value, text_value, body_format)).await
    }
    fn update_legacy_comment_(conn: &PgConnection, post_id_value: i32, legacy_comment_id_value: i32, text_value: String, body_format: BodyFormat) -> Result<(), DieselError> {
        let html_and_stuff = match body_format {
            BodyFormat::Plain => crate::prettify::prettify_body(&text_value, &mut PrettifyData::new(conn, post_id_value)),
            BodyFormat::BBCode => crate::prettify::prettify_body_bbcode(&text_value, &mut PrettifyData::new(conn, post_id_value)),
        };
        use self::legacy_comments::dsl::*;
        diesel::update(legacy_comments.find(legacy_comment_id_value))
            .set((
                text.eq(text_value),
                html.eq(&html_and_stuff.string)
            ))
            .execute(conn)
            .map(|_| ())
    }
    pub async fn bump_last_seen_at(&self, user_session_id: Base32) -> Result<(), DieselError> {
        self.run(move |conn| Self::bump_last_seen_at_(conn, user_session_id)).await
    }
    fn bump_last_seen_at_(conn: &PgConnection, user_session_id_value: Base32) -> Result<(), DieselError> {
        use self::user_sessions::dsl::*;
        let now = Utc::now().naive_utc();
        diesel::update(user_sessions.find(user_session_id_value.into_i64()))
            .set((
                last_seen_at.eq(now),
            ))
            .execute(conn)?;
        Ok(())
    }
    pub async fn create_session(&self, user_id: i32, user_agent: &str) -> Result<UserSession, DieselError> {
        let user_agent = user_agent.to_owned();
        self.run(move |conn| Self::create_session_(conn, user_id, &user_agent)).await
    }
    fn create_session_(conn: &PgConnection, user_id: i32, user_agent: &str) -> Result<UserSession, DieselError> {
        #[derive(Insertable)]
        #[table_name="user_sessions"]
        struct CreateSession<'a> {
            uuid: i64,
            user_agent: &'a str,
            user_id: i32,
        }
        let uuid = ::rand::random();
        diesel::insert_into(user_sessions::table)
            .values(CreateSession {
                uuid, user_agent, user_id
            })
            .get_result::<UserSession>(conn)
    }
    pub async fn get_session_by_uuid(&self, base32: Base32) -> Result<UserSession, DieselError> {
        self.run(move |conn| Self::get_session_by_uuid_(conn, base32)).await
    }
    fn get_session_by_uuid_(conn: &PgConnection, base32: Base32) -> Result<UserSession, DieselError> {
        use self::user_sessions::dsl::*;
        user_sessions.filter(uuid.eq(base32.into_i64())).get_result(conn)
    }
}

fn tuple_to_notification_info((post_uuid, post_title, comment_count, from_username): (Base32, String, i32, String)) -> NotificationInfo {
    NotificationInfo {
        post_uuid, post_title, comment_count, from_username,
    }
}

fn tuple_to_post_info_logged_out(data: &mut PrettifyData, (id, uuid, title, title_html, url, visible, private, initial_stellar_time, score, comment_count, blog_post, created_at, submitted_by, excerpt, excerpt_html, submitted_by_username, banner_title, banner_desc, noindex, locked, anon): (i32, Base32, String, Option<String>, Option<String>, bool, bool, i32, i32, i32, bool, NaiveDateTime, i32, Option<String>, Option<String>, String, Option<String>, Option<String>, bool, bool, bool), current_stellar_time: i32) -> PostInfo {
    tuple_to_post_info(data, (id, uuid, title, title_html, url, visible, private, initial_stellar_time, score, comment_count, None, blog_post, created_at, submitted_by, excerpt, excerpt_html, None, None, None, submitted_by_username, banner_title, banner_desc, noindex, locked, anon), current_stellar_time)
}

fn tuple_to_post_info(data: &mut PrettifyData, (id, uuid, title, title_html, url, visible, private, initial_stellar_time, score, comment_count, comment_readpoint, blog_post, created_at, submitted_by, excerpt, excerpt_html, starred_post_id, flagged_post_id, hidden_post_id, submitted_by_username, banner_title, banner_desc, noindex, locked, anon): (i32, Base32, String, Option<String>, Option<String>, bool, bool, i32, i32, i32, Option<i32>, bool, NaiveDateTime, i32, Option<String>, Option<String>, Option<i32>, Option<i32>, Option<i32>, String, Option<String>, Option<String>, bool, bool, bool), current_stellar_time: i32) -> PostInfo {
    let link_url = if let Some(ref url) = url {
        url.clone()
    } else {
        uuid.to_string()
    };
    let title_html_output;
    let title_html = if let Some(title_html) = title_html {
        title_html
    } else {
        title_html_output = prettify_title(&title, &link_url, data, blog_post);
        title_html_output.string
    };
    let excerpt_html_output;
    let excerpt_html = if let Some(excerpt_html) = excerpt_html {
        Some(excerpt_html)
    } else if let Some(ref excerpt) = excerpt {
        excerpt_html_output = crate::prettify::prettify_body_bbcode(excerpt, data);
        Some(excerpt_html_output.string)
    } else {
        None
    };
    let created_at_relative = relative_date(&created_at);
    let submitted_by_username_urlencode = utf8_percent_encode(&submitted_by_username, NON_ALPHANUMERIC).to_string();
    PostInfo {
        id, uuid, title, url, visible, private, score, blog_post,
        submitted_by, submitted_by_username, comment_count, title_html,
        comment_readpoint,
        excerpt_html, banner_title, banner_desc,
        created_at, created_at_relative,
        starred_by_me: starred_post_id.is_some(),
        flagged_by_me: flagged_post_id.is_some(),
        hidden_by_me: hidden_post_id.is_some(),
        submitted_by_username_urlencode,
        noindex, locked, anon,
        hotness: compute_hotness(initial_stellar_time, current_stellar_time, score, blog_post)
    }
}

fn tuple_to_comment_info(conn: &PgConnection, (id, text, html, visible, post_id, created_at, created_by, starred_comment_id, flagged_comment_id, hidden_comment_id, created_by_username, created_by_identicon): (i32, String, String, bool, i32, NaiveDateTime, i32, Option<i32>, Option<i32>, Option<i32>, String, i32)) -> CommentInfo {
    let created_at_relative = relative_date(&created_at);
    let created_by_username_urlencode = utf8_percent_encode(&created_by_username, NON_ALPHANUMERIC).to_string();
    let created_by_identicon = Base32::from(created_by_identicon as i64);
    CommentInfo {
        id, text, html, visible, post_id, created_by, created_by_username,
        created_at, created_at_relative,
        created_by_username_urlencode,
        created_by_identicon,
        starred_by: MoreInterestingConn::get_comment_starred_by_(conn, id).unwrap_or(Vec::new()),
        starred_by_me: starred_comment_id.is_some(),
        flagged_by_me: flagged_comment_id.is_some(),
        hidden_by_me: hidden_comment_id.is_some(),
    }
}

fn tuple_to_comment_search_results(conn: &PgConnection, (id, html, post_id, post_uuid, post_title, created_at, created_by, created_by_username, starred_comment_id, flagged_comment_id, created_by_identicon, post_locked): (i32, String, i32, Base32, String, NaiveDateTime, i32, String, Option<i32>, Option<i32>, i32, bool)) -> CommentSearchResult {
    let created_at_relative = relative_date(&created_at);
    let created_by_identicon = Base32::from(created_by_identicon as i64);
    CommentSearchResult {
        id, html, post_id, post_uuid, post_title, created_by, created_by_username,
        created_at, created_at_relative,
        created_by_identicon,
        post_locked,
        starred_by_me: starred_comment_id.is_some(),
        flagged_by_me: flagged_comment_id.is_some(),
        starred_by: MoreInterestingConn::get_comment_starred_by_(conn, id).unwrap_or(Vec::new()),
    }
}

fn tuple_to_moderation((id, payload, created_at, created_by, created_by_username): (i32, json::Value, NaiveDateTime, i32, String)) -> ModerationInfo {
    ModerationInfo {
        id,
        payload,
        created_at,
        created_by,
        created_by_username,
    }
}

fn compute_hotness(initial_stellar_time: i32, current_stellar_time: i32, score: i32, blog_post: bool) -> f64 {
    let gravity = 1.33;
    let boost = if blog_post { 0.33 } else { 0.0 };
    let stellar_age = max(current_stellar_time - initial_stellar_time, 0) as f64;
    (boost + (score as f64) + 1.0) / (stellar_age + 1.0).powf(gravity)
}

pub fn relative_date(dt: &NaiveDateTime) -> String {
    // Design rationale:
    //
    // - NaiveDateTime is used for timestamps because they're always in the past,
    //   and are conventionally UTC. They are also only really used for display,
    //   not scheduling, so jitter is acceptable.
    // - "Humanization" is done on the server side to avoid layout jumping after the
    //   JS loads, and is always done in a relative way that is timezone agnostic.
    //   Localized dates, which can't be shown until after the JS loads because
    //   timezones can't be reliably pulled from HTTP headers, are relegated
    //   to tooltips.
    use chrono_humanize::{Accuracy, HumanTime, Tense};
    let h = HumanTime::from(*dt - Utc::now().naive_utc());
    v_htmlescape::escape(&h.to_text_en(Accuracy::Rough, Tense::Present)).to_string()
}

pub struct PrettifyData<'a> {
    conn: &'a PgConnection,
    post_id: i32,
    tag_cache: HashSet<String>,
    has_user_cache: HashSet<String>,
    domain_map_cache: HashMap<String, String>,
}
impl<'a> PrettifyData<'a> {
    pub fn new(conn: &'a PgConnection, post_id: i32) -> PrettifyData<'a> {
        let tag_cache: HashSet<String> = MoreInterestingConn::get_all_tags_(conn)
            .unwrap_or(Vec::new())
            .into_iter()
            .map(|t| t.name)
            .collect();
        PrettifyData {
            conn, post_id, tag_cache,
            has_user_cache: HashSet::new(),
            domain_map_cache: HashMap::new(),
        }
    }
}
impl<'a> prettify::Data for PrettifyData<'a> {
    fn check_comment_ref(&mut self, comment_id: i32) -> bool {
        if self.post_id == 0 {
            false
        } else {
            if let Ok(comment) = MoreInterestingConn::get_comment_by_id_(self.conn, comment_id) {
                comment.post_id == self.post_id
            } else {
                false
            }
        }
    }
    fn check_hash_tag(&mut self, tag: &str) -> bool {
        self.tag_cache.contains(tag)
    }
    fn check_username(&mut self, username: &str) -> bool {
        if self.has_user_cache.contains(username) {
            true
        } else {
            let has_user = MoreInterestingConn::get_user_by_username_(self.conn, username).is_ok();
            if has_user {
                self.has_user_cache.insert(username.to_string());
            }
            has_user
        }
    }
    fn get_domain_canonical(&mut self, hostname: &str) -> String {
        let domain_map_cache = &mut self.domain_map_cache;
        let conn = self.conn;
        domain_map_cache.entry(hostname.to_string()).or_insert_with(|| {
            MoreInterestingConn::get_domain_by_hostname_(conn, hostname)
                .map(|domain| domain.hostname)
                .unwrap_or_else(|_| hostname.to_owned())
        }).clone()
    }
}
