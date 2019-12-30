use rocket_contrib::databases::diesel::PgConnection;
use diesel::prelude::*;
use diesel::result::Error as DieselError;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use crate::schema::{site_customization, users, user_sessions, posts, stars, invite_tokens, comments, comment_stars, tags, post_tagging, moderation, flags, comment_flags, domains, legacy_comments, domain_synonyms, notifications, subscriptions};
use crate::password::{password_hash, password_verify, PasswordResult};
use serde::Serialize;
use crate::base32::Base32;
use std::cmp::max;
use ordered_float::OrderedFloat;
use std::collections::{HashMap, HashSet};
use crate::prettify::{self, prettify_title};
use serde_json::{self as json, json};
use url::Url;

const FLAG_HIDE_THRESHOLD: i64 = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum BodyFormat {
    Plain,
    BBCode,
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
    pub authored_by_submitter: bool,
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
}

#[derive(Clone, Queryable, Serialize)]
pub struct UserSession {
    pub uuid: Base32,
    pub created_at: NaiveDateTime,
    pub user_agent: String,
    pub user_id: i32,
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
        }
    }
}

#[derive(Insertable, Queryable, QueryableByName, Serialize)]
#[table_name="site_customization"]
pub struct SiteCustomization {
    pub name: String,
    pub value: String,
}

pub struct NewPost<'a> {
    pub title: &'a str,
    pub url: Option<&'a str>,
    pub excerpt: Option<&'a str>,
    pub submitted_by: i32,
    pub visible: bool,
    pub private: bool,
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
    pub authored_by_submitter: bool,
    pub created_at: NaiveDateTime,
    pub created_at_relative: String,
    pub submitted_by: i32,
    pub submitted_by_username: String,
    pub starred_by_me: bool,
    pub flagged_by_me: bool,
    pub excerpt_html: Option<String>,
    pub banner_title: Option<String>,
    pub banner_desc: Option<String>,
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

pub struct NewComment<'a> {
    pub post_id: i32,
    pub text: &'a str,
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
    pub starred_by_me: bool,
    pub flagged_by_me: bool,
    pub starred_by: Vec<String>,
}

#[derive(Serialize)]
pub struct CommentSearchResult {
    pub id: i32,
    pub html: String,
    pub post_id: i32,
    pub post_uuid: Base32,
    pub post_title: String,
    pub created_at: NaiveDateTime,
    pub created_at_relative: String,
    pub created_by: i32,
    pub created_by_username: String,
}

#[derive(Queryable)]
pub struct InviteToken {
    pub uuid: Base32,
    pub created_at: NaiveDateTime,
    pub invited_by: i32,
}

#[derive(Insertable)]
#[table_name="stars"]
pub struct NewStar {
    pub user_id: i32,
    pub post_id: i32,
}

#[derive(Insertable)]
#[table_name="flags"]
pub struct NewFlag {
    pub user_id: i32,
    pub post_id: i32,
}

#[derive(Insertable)]
#[table_name="comment_stars"]
pub struct NewStarComment {
    pub user_id: i32,
    pub comment_id: i32,
}

#[derive(Insertable)]
#[table_name="comment_flags"]
pub struct NewFlagComment {
    pub user_id: i32,
    pub comment_id: i32,
}

pub struct UserAuth<'a> {
    pub username: &'a str,
    pub password: &'a str,
}

pub struct NewUser<'a> {
    pub username: &'a str,
    pub password: &'a str,
    pub invited_by: Option<i32>,
}

pub struct NewTag<'a> {
    pub name: &'a str,
    pub description: Option<&'a str>,
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

#[derive(Insertable)]
#[table_name="subscriptions"]
pub struct NewSubscription {
    pub user_id: i32,
    pub post_id: i32,
    pub created_by: i32,
}

pub struct PostSearch {
    pub my_user_id: i32,
    pub order_by: PostSearchOrderBy,
    pub keywords: String,
    pub or_tags: Vec<i32>,
    pub and_tags: Vec<i32>,
    pub or_domains: Vec<i32>,
    pub after_post_id: i32,
    pub subscriptions: bool,
    pub before_date: Option<NaiveDate>,
    pub after_date: Option<NaiveDate>,
}

impl PostSearch {
    pub fn with_my_user_id(my_user_id: i32) -> PostSearch {
        PostSearch {
            my_user_id,
            order_by: PostSearchOrderBy::Hottest,
            keywords: String::new(),
            or_tags: Vec::new(),
            and_tags: Vec::new(),
            or_domains: Vec::new(),
            after_post_id: 0,
            subscriptions: false,
            before_date: None,
            after_date: None,
        }
    }
}

#[database("more_interesting")]
pub struct MoreInterestingConn(PgConnection);

impl MoreInterestingConn {
    pub fn set_customization(&self, new: SiteCustomization) -> Result<(), DieselError> {
        use self::site_customization::dsl::*;
        let affected_rows = if self.get_customization_value(&new.name).is_some() {
            diesel::update(site_customization.find(&new.name))
                .set(value.eq(&new.value))
                .execute(&self.0)?
        } else {
            diesel::insert_into(site_customization)
                .values(new)
                .execute(&self.0)?
        };
        assert_eq!(affected_rows, 1);
        Ok(())
    }
    pub fn get_customizations(&self) -> Result<Vec<SiteCustomization>, DieselError> {
        use self::site_customization::dsl::*;
        site_customization.get_results::<SiteCustomization>(&self.0)
    }
    pub fn get_customization_value(&self, name_param: &str) -> Option<String> {
        use self::site_customization::dsl::*;
        site_customization
            .select(value)
            .filter(name.eq(name_param))
            .get_result::<String>(&self.0)
            .ok()
    }
    pub fn create_notification(&self, new: NewNotification) -> Result<(), DieselError> {
        diesel::insert_into(notifications::table)
            .values(new)
            .execute(&self.0)?;
        Ok(())
    }
    pub fn use_notification(&self, post_id_value: i32, user_id_value: i32) -> Result<(), DieselError> {
        use self::notifications::dsl::*;
        diesel::delete(notifications.filter(user_id.eq(user_id_value)).filter(post_id.eq(post_id_value)))
            .execute(&self.0)?;
        Ok(())
    }
    pub fn list_notifications(&self, user_id_value: i32) -> Result<Vec<NotificationInfo>, DieselError> {
        use self::posts::dsl::*;
        use self::users::dsl::*;
        use self::notifications::dsl::*;
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
            .get_results::<(Base32, String, i32, String)>(&self.0)?
            .into_iter()
            .map(|t| tuple_to_notification_info(t))
            .collect();
        Ok(all)
    }
    pub fn is_subscribed(&self, post_id_value: i32, user_id_value: i32) -> Result<bool, DieselError> {
        use self::subscriptions::dsl::*;
        use diesel::{select, dsl::exists};
        Ok(select(exists(subscriptions
            .filter(post_id.eq(post_id_value))
            .filter(user_id.eq(user_id_value))))
            .get_result::<bool>(&self.0)?)
    }
    pub fn create_subscription(&self, new: NewSubscription) -> Result<(), DieselError> {
        diesel::insert_into(subscriptions::table)
            .values(new)
            .execute(&self.0)?;
        Ok(())
    }
    pub fn drop_subscription(&self, new: NewSubscription) -> Result<(), DieselError> {
        use self::subscriptions::dsl::*;
        diesel::delete(subscriptions.filter(user_id.eq(new.user_id)).filter(post_id.eq(new.post_id)))
            .execute(&self.0)?;
        Ok(())
    }
    pub fn list_subscribed_users(&self, post_id_value: i32) -> Result<Vec<i32>, DieselError> {
        use self::subscriptions::dsl::*;
        Ok(subscriptions
            .select(user_id)
            .filter(post_id.eq(post_id_value))
            .get_results::<i32>(&self.0)?)
    }
    pub fn search_posts(&self, search: &PostSearch) -> Result<Vec<PostInfo>, DieselError> {
        use self::posts::dsl::*;
        use self::stars::dsl::*;
        use self::flags::dsl::*;
        use self::users::dsl::*;
        use self::subscriptions::dsl::*;
        use self::post_tagging::dsl::*;
        use crate::schema::post_search_index::dsl::*;
        use diesel_full_text_search::{plainto_tsquery, TsVectorExtensions, ts_rank_cd};
        let query = posts.filter(visible.eq(true));
        let mut query = match search.order_by {
            PostSearchOrderBy::Hottest if search.keywords == "" => query.order_by((initial_stellar_time.desc(), self::posts::dsl::created_at.desc())).into_boxed(),
            PostSearchOrderBy::Hottest => query.into_boxed(),
            PostSearchOrderBy::Top => query.order_by((score.desc(), self::posts::dsl::created_at.desc())).into_boxed(),
            PostSearchOrderBy::Newest => query.order_by(self::posts::dsl::id.desc()).into_boxed(),
            PostSearchOrderBy::Latest => query.order_by(self::posts::dsl::updated_at.desc()).into_boxed(),
        };
        if !search.or_domains.is_empty() {
            query = query.filter(domain_id.eq_any(&search.or_domains))
        }
        if !search.or_tags.is_empty() {
            let ids = post_tagging
                .filter(self::post_tagging::dsl::tag_id.eq_any(&search.or_tags))
                .select(self::post_tagging::dsl::post_id);
            query = query.filter(self::posts::dsl::id.eq_any(ids));
        }
        if !search.and_tags.is_empty() {
            for &tag_id_ in &search.and_tags {
                let ids = post_tagging
                    .filter(self::post_tagging::dsl::tag_id.eq(tag_id_))
                    .select(self::post_tagging::dsl::post_id);
                query = query.filter(self::posts::dsl::id.eq_any(ids));
            }
        }
        if search.subscriptions {
            let ids = subscriptions
                .filter(self::subscriptions::dsl::user_id.eq(search.my_user_id))
                .select(self::subscriptions::dsl::post_id);
            query = query.filter(self::posts::dsl::id.eq_any(ids));
        } else {
            query = query.filter(private.eq(false));
        }
        if let Some(before_date) = search.before_date {
            let midnight = NaiveTime::from_hms(23, 59, 59);
            query = query.filter(self::posts::dsl::created_at.lt(before_date.and_time(midnight)));
        }
        if let Some(after_date) = search.after_date {
            let midnight = NaiveTime::from_hms(0, 0, 0);
            query = query.filter(self::posts::dsl::created_at.gt(after_date.and_time(midnight)));
        }
        let mut data = PrettifyData::new(self, 0);
        let mut all: Vec<PostInfo> = if search.keywords != "" && search.order_by == PostSearchOrderBy::Hottest {
            let max_r = if search.after_post_id != 0 {
                post_search_index
                    .filter(crate::schema::post_search_index::dsl::post_id.eq(search.after_post_id))
                    .select(ts_rank_cd(search_index, plainto_tsquery(&search.keywords)))
                    .get_result(&self.0)?
            } else {
                1_000_000.0
            };
            query
                .left_outer_join(stars.on(self::stars::dsl::post_id.eq(self::posts::dsl::id).and(self::stars::dsl::user_id.eq(search.my_user_id))))
                .left_outer_join(flags.on(self::flags::dsl::post_id.eq(self::posts::dsl::id).and(self::flags::dsl::user_id.eq(search.my_user_id))))
                .inner_join(users)
                .inner_join(post_search_index)
                .select((
                    self::posts::dsl::id,
                    self::posts::dsl::uuid,
                    self::posts::dsl::title,
                    self::posts::dsl::url,
                    self::posts::dsl::visible,
                    self::posts::dsl::private,
                    self::posts::dsl::initial_stellar_time,
                    self::posts::dsl::score,
                    self::posts::dsl::comment_count,
                    self::posts::dsl::authored_by_submitter,
                    self::posts::dsl::created_at,
                    self::posts::dsl::submitted_by,
                    self::posts::dsl::excerpt_html,
                    self::stars::dsl::post_id.nullable(),
                    self::flags::dsl::post_id.nullable(),
                    self::users::dsl::username,
                    self::posts::dsl::banner_title,
                    self::posts::dsl::banner_desc,
                ))
                .filter(search_index.matches(plainto_tsquery(&search.keywords)))
                .filter(ts_rank_cd(search_index, plainto_tsquery(&search.keywords)).lt(max_r))
                .order_by(ts_rank_cd(search_index, plainto_tsquery(&search.keywords)).desc())
                .limit(75)
                .get_results::<(i32, Base32, String, Option<String>, bool, bool, i32, i32, i32, bool, NaiveDateTime, i32, Option<String>, Option<i32>, Option<i32>, String, Option<String>, Option<String>)>(&self.0)?
                .into_iter()
                .map(|t| tuple_to_post_info(&mut data, t, self.get_current_stellar_time()))
                .collect()
        } else {
            if search.keywords != "" {
                let ids = post_search_index
                    .filter(search_index.matches(plainto_tsquery(&search.keywords)))
                    .select(crate::schema::post_search_index::dsl::post_id);
                query = query.filter(self::posts::dsl::id.eq_any(ids));
            }
            if search.after_post_id != 0 {
                query = query.filter(self::posts::dsl::id.lt(search.after_post_id));
            }
            query
                .left_outer_join(stars.on(self::stars::dsl::post_id.eq(self::posts::dsl::id).and(self::stars::dsl::user_id.eq(search.my_user_id))))
                .left_outer_join(flags.on(self::flags::dsl::post_id.eq(self::posts::dsl::id).and(self::flags::dsl::user_id.eq(search.my_user_id))))
                .inner_join(users)
                .select((
                    self::posts::dsl::id,
                    self::posts::dsl::uuid,
                    self::posts::dsl::title,
                    self::posts::dsl::url,
                    self::posts::dsl::visible,
                    self::posts::dsl::private,
                    self::posts::dsl::initial_stellar_time,
                    self::posts::dsl::score,
                    self::posts::dsl::comment_count,
                    self::posts::dsl::authored_by_submitter,
                    self::posts::dsl::created_at,
                    self::posts::dsl::submitted_by,
                    self::posts::dsl::excerpt_html,
                    self::stars::dsl::post_id.nullable(),
                    self::flags::dsl::post_id.nullable(),
                    self::users::dsl::username,
                    self::posts::dsl::banner_title,
                    self::posts::dsl::banner_desc,
                ))
                .limit(75)
                .get_results::<(i32, Base32, String, Option<String>, bool, bool, i32, i32, i32, bool, NaiveDateTime, i32, Option<String>, Option<i32>, Option<i32>, String, Option<String>, Option<String>)>(&self.0)?
                .into_iter()
                .map(|t| tuple_to_post_info(&mut data, t, self.get_current_stellar_time()))
                .collect()
        };
        if let (PostSearchOrderBy::Hottest, "") = (search.order_by, &search.keywords[..]) {
            all.sort_by_key(|info| OrderedFloat(-info.hotness));
            all.truncate(50);
        }
        Ok(all)
    }
    pub fn get_post_info_recent_by_user(&self, user_id_param: i32, user_info_id_param: i32) -> Result<Vec<PostInfo>, DieselError> {
        use self::posts::dsl::*;
        use self::stars::dsl::*;
        use self::flags::dsl::*;
        use self::users::dsl::*;
        let mut data = PrettifyData::new(self, 0);
        let mut all: Vec<PostInfo> = posts
            .left_outer_join(stars.on(self::stars::dsl::post_id.eq(self::posts::dsl::id).and(self::stars::dsl::user_id.eq(user_id_param))))
            .left_outer_join(flags.on(self::flags::dsl::post_id.eq(self::posts::dsl::id).and(self::flags::dsl::user_id.eq(user_id_param))))
            .inner_join(users)
            .select((
                self::posts::dsl::id,
                self::posts::dsl::uuid,
                self::posts::dsl::title,
                self::posts::dsl::url,
                self::posts::dsl::visible,
                self::posts::dsl::private,
                self::posts::dsl::initial_stellar_time,
                self::posts::dsl::score,
                self::posts::dsl::comment_count,
                self::posts::dsl::authored_by_submitter,
                self::posts::dsl::created_at,
                self::posts::dsl::submitted_by,
                self::posts::dsl::excerpt_html,
                self::stars::dsl::post_id.nullable(),
                self::flags::dsl::post_id.nullable(),
                self::users::dsl::username,
                self::posts::dsl::banner_title,
                self::posts::dsl::banner_desc,
            ))
            .filter(visible.eq(true))
            .filter(private.eq(false))
            .filter(self::posts::dsl::submitted_by.eq(user_info_id_param))
            .order_by((initial_stellar_time.desc(), self::posts::dsl::created_at.desc()))
            .limit(75)
            .get_results::<(i32, Base32, String, Option<String>, bool, bool, i32, i32, i32, bool, NaiveDateTime, i32, Option<String>, Option<i32>, Option<i32>, String, Option<String>, Option<String>)>(&self.0)?
            .into_iter()
            .map(|t| tuple_to_post_info(&mut data, t, self.get_current_stellar_time()))
            .collect();
        all.sort_by_key(|info| OrderedFloat(-info.hotness));
        all.truncate(50);
        Ok(all)
    }
    pub fn get_post_info_by_uuid(&self, user_id_param: i32, uuid_param: Base32) -> Result<PostInfo, DieselError> {
        use self::posts::dsl::*;
        use self::stars::dsl::*;
        use self::flags::dsl::*;
        use self::users::dsl::*;
        // This is a bunch of duplicate code.
        let mut data = PrettifyData::new(self, 0);
        Ok(tuple_to_post_info(&mut data, posts
            .left_outer_join(stars.on(self::stars::dsl::post_id.eq(self::posts::dsl::id).and(self::stars::dsl::user_id.eq(user_id_param))))
            .left_outer_join(flags.on(self::flags::dsl::post_id.eq(self::posts::dsl::id).and(self::flags::dsl::user_id.eq(user_id_param))))
            .inner_join(users)
            .select((
                self::posts::dsl::id,
                self::posts::dsl::uuid,
                self::posts::dsl::title,
                self::posts::dsl::url,
                self::posts::dsl::visible,
                self::posts::dsl::private,
                self::posts::dsl::initial_stellar_time,
                self::posts::dsl::score,
                self::posts::dsl::comment_count,
                self::posts::dsl::authored_by_submitter,
                self::posts::dsl::created_at,
                self::posts::dsl::submitted_by,
                self::posts::dsl::excerpt_html,
                self::stars::dsl::post_id.nullable(),
                self::flags::dsl::post_id.nullable(),
                self::users::dsl::username,
                self::posts::dsl::banner_title,
                self::posts::dsl::banner_desc,
            ))
            .filter(rejected.eq(false))
            .filter(uuid.eq(uuid_param.into_i64()))
            .first::<(i32, Base32, String, Option<String>, bool, bool, i32, i32, i32, bool, NaiveDateTime, i32, Option<String>, Option<i32>, Option<i32>, String, Option<String>, Option<String>)>(&self.0)?, self.get_current_stellar_time()))
    }
    pub fn get_current_stellar_time(&self) -> i32 {
        use self::stars::dsl::*;
        // the stars table should be limited by the i32 limits, but diesel doesn't know that
        stars.count().get_result(&self.0).unwrap_or(0) as i32
    }
    fn get_post_domain_url(&self, url: Option<&str>) -> (Option<String>, Option<Domain>) {
        let url_host = url
            .and_then(|u| Url::parse(u).ok())
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
            let domain = self.get_domain_by_hostname(host).unwrap_or_else(|_| {
                self.create_domain(NewDomain {
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
    pub fn create_post(&self, new_post: &NewPost, body_format: BodyFormat) -> Result<Post, DieselError> {
        #[derive(Insertable)]
        #[table_name="posts"]
        struct CreatePost<'a> {
            title: &'a str,
            uuid: i64,
            url: Option<String>,
            submitted_by: i32,
            initial_stellar_time: i32,
            excerpt: Option<&'a str>,
            excerpt_html: Option<&'a str>,
            visible: bool,
            private: bool,
            domain_id: Option<i32>,
        }
        let title_html_and_stuff = crate::prettify::prettify_title(new_post.title, "", &mut PrettifyData::new(self, 0));
        let excerpt_html_and_stuff = if let Some(excerpt) = new_post.excerpt {
            let body = match body_format {
                BodyFormat::Plain => crate::prettify::prettify_body(excerpt, &mut PrettifyData::new(self, 0)),
                BodyFormat::BBCode => crate::prettify::prettify_body_bbcode(excerpt, &mut PrettifyData::new(self, 0)),
            };
            Some(body)
        } else {
            None
        };
        let (url, domain) = self.get_post_domain_url(new_post.url);
        let result = diesel::insert_into(posts::table)
            .values(CreatePost {
                title: new_post.title,
                uuid: rand::random(),
                submitted_by: new_post.submitted_by,
                initial_stellar_time: self.get_current_stellar_time(),
                excerpt: new_post.excerpt,
                excerpt_html: excerpt_html_and_stuff.as_ref().map(|e| &e.string[..]),
                visible: new_post.visible,
                private: new_post.private,
                domain_id: domain.map(|d| d.id),
                url
            })
            .get_result::<Post>(&self.0);
        if let Ok(ref post) = result {
            for tag in title_html_and_stuff.hash_tags.iter().chain(excerpt_html_and_stuff.iter().flat_map(|e| e.hash_tags.iter())).map(|s| &s[..]).collect::<HashSet<&str>>() {
                if let Ok(tag_info) = self.get_tag_by_name(&tag) {
                    diesel::insert_into(post_tagging::table)
                        .values(CreatePostTagging {
                            post_id: post.id,
                            tag_id: tag_info.id,
                        })
                        .execute(&self.0)?;
                }
            }
        }
        result
    }
    pub fn update_post(&self, post_id_value: i32, bump: bool, new_post: &NewPost, body_format: BodyFormat) -> Result<(), DieselError> {
        let title_html_and_stuff = crate::prettify::prettify_title(new_post.title, "", &mut PrettifyData::new(self, 0));
        let excerpt_html_and_stuff = if let Some(e) = new_post.excerpt {
            let body = match body_format {
                BodyFormat::Plain => crate::prettify::prettify_body(e, &mut PrettifyData::new(self, 0)),
                BodyFormat::BBCode => crate::prettify::prettify_body_bbcode(e, &mut PrettifyData::new(self, 0)),
            };
            Some(body)
        } else {
            None
        };
        use self::posts::dsl::*;
        use self::post_tagging::dsl::*;
        let (url_value, domain) = self.get_post_domain_url(new_post.url);
        diesel::update(posts.find(post_id_value))
            .set((
                title.eq(new_post.title),
                excerpt.eq(new_post.excerpt),
                url.eq(url_value),
                excerpt_html.eq(excerpt_html_and_stuff.as_ref().map(|x| &x.string[..])),
                visible.eq(new_post.visible),
                private.eq(new_post.private),
                domain_id.eq(domain.map(|d| d.id))
            ))
            .execute(&self.0)?;
        if bump {
            diesel::update(posts.find(post_id_value))
                .set((
                    initial_stellar_time.eq(self.get_current_stellar_time()),
                    visible.eq(new_post.visible),
                    private.eq(new_post.private),
                ))
                .execute(&self.0)?;
        }
        diesel::delete(post_tagging.filter(post_id.eq(post_id_value)))
            .execute(&self.0)?;
        for tag in title_html_and_stuff.hash_tags.iter().chain(excerpt_html_and_stuff.iter().flat_map(|e| e.hash_tags.iter())).map(|s| &s[..]).collect::<HashSet<&str>>() {
            if let Ok(tag_info) = self.get_tag_by_name(&tag) {
                diesel::insert_into(post_tagging)
                    .values(CreatePostTagging {
                        post_id: post_id_value,
                        tag_id: tag_info.id,
                    })
                    .execute(&self.0)?;
            }
        }
        Ok(())
    }
    pub fn add_star(&self, new_star: &NewStar) -> bool {
        let affected_rows = diesel::insert_into(stars::table)
            .values(new_star)
            .execute(&self.0)
            .unwrap_or(0);
        // affected rows will be 1 if inserted, or 0 otherwise
        self.update_score_on_post(new_star.post_id, affected_rows as i32);
        affected_rows == 1
    }
    pub fn rm_star(&self, new_star: &NewStar) -> bool {
        use self::stars::dsl::*;
        let affected_rows = diesel::delete(
            stars
                .filter(user_id.eq(new_star.user_id))
                .filter(post_id.eq(new_star.post_id))
        )
            .execute(&self.0)
            .unwrap_or(0);
        // affected rows will be 1 if deleted, or 0 otherwise
        self.update_score_on_post(new_star.post_id, -(affected_rows as i32));
        affected_rows == 1
    }
    pub fn add_star_comment(&self, new_star: &NewStarComment) -> bool {
        let affected_rows = diesel::insert_into(comment_stars::table)
            .values(new_star)
            .execute(&self.0)
            .unwrap_or(0);
        affected_rows == 1
    }
    pub fn rm_star_comment(&self, new_star: &NewStarComment) -> bool {
        use self::comment_stars::dsl::*;
        let affected_rows = diesel::delete(
            comment_stars
                .filter(user_id.eq(new_star.user_id))
                .filter(comment_id.eq(new_star.comment_id))
        )
            .execute(&self.0)
            .unwrap_or(0);
        affected_rows == 1
    }
    fn maybe_hide_post(&self, post_id_param: i32) {
        use self::flags::dsl::*;
        let flag_count: i64 = flags.filter(post_id.eq(post_id_param)).count().get_result(&self.0).expect("if flagging worked, then so should counting");
        if flag_count == FLAG_HIDE_THRESHOLD {
            self.hide_post(post_id_param).expect("if flagging worked, then so should hiding the post");
        }
    }
    fn maybe_hide_comment(&self, comment_id_param: i32) {
        use self::comment_flags::dsl::*;
        let flag_count: i64 = comment_flags.filter(comment_id.eq(comment_id_param)).count().get_result(&self.0).expect("if flagging worked, then so should counting");
        if flag_count == FLAG_HIDE_THRESHOLD {
            self.hide_comment(comment_id_param).expect("if flagging worked, then so should hiding the post");
        }
    }
    pub fn add_flag(&self, new_flag: &NewFlag) -> bool {
        let affected_rows = diesel::insert_into(flags::table)
            .values(new_flag)
            .execute(&self.0)
            .unwrap_or(0);
        if affected_rows == 1 {
            self.maybe_hide_post(new_flag.post_id);
        }
        affected_rows == 1
    }
    pub fn rm_flag(&self, new_flag: &NewFlag) -> bool {
        use self::flags::dsl::*;
        let affected_rows = diesel::delete(
            flags
                .filter(user_id.eq(new_flag.user_id))
                .filter(post_id.eq(new_flag.post_id))
        )
            .execute(&self.0)
            .unwrap_or(0);
        affected_rows == 1
    }
    pub fn add_flag_comment(&self, new_flag: &NewFlagComment) -> bool {
        let affected_rows = diesel::insert_into(comment_flags::table)
            .values(new_flag)
            .execute(&self.0)
            .unwrap_or(0);
        if affected_rows == 1 {
            self.maybe_hide_comment(new_flag.comment_id);
        }
        affected_rows == 1
    }
    pub fn rm_flag_comment(&self, new_flag: &NewFlagComment) -> bool {
        use self::comment_flags::dsl::*;
        let affected_rows = diesel::delete(
            comment_flags
                .filter(user_id.eq(new_flag.user_id))
                .filter(comment_id.eq(new_flag.comment_id))
        )
            .execute(&self.0)
            .unwrap_or(0);
        affected_rows == 1
    }
    fn update_score_on_post(&self, post_id_value: i32, count_value: i32) {
        use self::posts::dsl::*;
        diesel::update(posts.find(post_id_value)).set(score.eq(score + count_value))
            .execute(&self.0)
            .expect("if adding a star worked, then so should updating the post");
    }
    fn update_comment_count_on_post(&self, post_id_value: i32, count_value: i32) -> Result<(), DieselError> {
        use self::posts::dsl::*;
        diesel::update(posts.find(post_id_value)).set(comment_count.eq(comment_count + count_value))
            .execute(&self.0)
            .map(|_| ())
    }
    pub fn has_users(&self) -> Result<bool, DieselError> {
        use self::users::dsl::*;
        use diesel::{select, dsl::exists};
        select(exists(users.select(id))).get_result(&self.0)
    }
    pub fn get_user_by_id(&self, user_id_param: i32) -> Result<User, DieselError> {
        use self::users::dsl::*;
        users.filter(id.eq(user_id_param)).get_result(&self.0)
    }
    pub fn get_user_by_username(&self, username_param: &str) -> Result<User, DieselError> {
        use self::users::dsl::*;
        users.filter(username.eq(username_param)).get_result(&self.0)
    }
    pub fn register_user(&self, new_user: &NewUser) -> Result<User, DieselError> {
        #[derive(Insertable)]
        #[table_name="users"]
        struct CreateUser<'a> {
            username: &'a str,
            password_hash: &'a [u8],
            invited_by: Option<i32>,
        }
        let password_hash = password_hash(new_user.password);
        diesel::insert_into(users::table)
            .values(CreateUser {
                username: new_user.username,
                password_hash: &password_hash[..],
                invited_by: new_user.invited_by,
            })
            .get_result(&self.0)
    }
    pub fn authenticate_user(&self, new_user: &UserAuth) -> Option<User> {
        let mut user = self.get_user_by_username(new_user.username).ok()?;
        if password_verify(new_user.password, &mut user.password_hash[..]) == PasswordResult::Passed {
            Some(user)
        } else {
            None
        }
    }
    pub fn change_user_password(&self, user_id_value: i32, password: &str) -> Result<(), DieselError> {
        use self::users::dsl::*;
        let password_hash_value = crate::password::password_hash(password);
        diesel::update(users.find(user_id_value)).set(password_hash.eq(password_hash_value))
            .execute(&self.0)
            .map(|k| { assert_eq!(k, 1); })
    }
    pub fn change_user_trust_level(&self, user_id_value: i32, trust_level_value: i32) -> Result<(), DieselError> {
        use self::users::dsl::*;
        diesel::update(users.find(user_id_value)).set(trust_level.eq(trust_level_value))
            .execute(&self.0)
            .map(|k| { assert_eq!(k, 1); })
    }
    pub fn change_user_banned(&self, user_id_value: i32, banned_value: bool) -> Result<(), DieselError> {
        use self::users::dsl::*;
        diesel::update(users.find(user_id_value)).set(banned.eq(banned_value))
            .execute(&self.0)
            .map(|k| { assert_eq!(k, 1); })
    }
    pub fn create_invite_token(&self, invited_by: i32) -> Result<InviteToken, DieselError> {
        #[derive(Insertable)]
        #[table_name="invite_tokens"]
        struct CreateInviteToken {
            invited_by: i32,
            uuid: i64,
        }
        diesel::insert_into(invite_tokens::table)
            .values(CreateInviteToken {
                uuid: rand::random(),
                invited_by
            })
            .get_result(&self.0)
    }
    pub fn check_invite_token_exists(&self, uuid_value: Base32) -> bool {
        use self::invite_tokens::dsl::*;
        use diesel::{select, dsl::exists};
        let uuid_value = uuid_value.into_i64();
        select(exists(invite_tokens.find(uuid_value))).get_result(&self.0).unwrap_or(false)
    }
    pub fn consume_invite_token(&self, uuid_value: Base32) -> Result<InviteToken, DieselError> {
        use self::invite_tokens::dsl::*;
        let uuid_value = uuid_value.into_i64();
        diesel::delete(invite_tokens.find(uuid_value)).get_result(&self.0)
    }
    pub fn get_invite_tree(&self) -> HashMap<i32, Vec<User>> {
        use self::users::dsl::*;
        let mut ret_val: HashMap<i32, Vec<User>> = HashMap::new();
        for user in users.get_results::<User>(&self.0).unwrap_or(Vec::new()).into_iter() {
            ret_val.entry(user.invited_by.unwrap_or(0)).or_default().push(user)
        }
        ret_val
    }
    pub fn get_comment_by_id(&self, comment_id_value: i32) -> Result<Comment, DieselError> {
        use self::comments::dsl::*;
        comments.find(comment_id_value).get_result::<Comment>(&self.0)
    }
    pub fn get_comment_info_by_id(&self, comment_id_value: i32, user_id_param: i32) -> Result<CommentInfo, DieselError> {
        use self::comments::dsl::*;
        use self::comment_stars::dsl::*;
        use self::comment_flags::dsl::*;
        use self::users::dsl::*;
        comments
            .left_outer_join(comment_stars.on(self::comment_stars::dsl::comment_id.eq(self::comments::dsl::id).and(self::comment_stars::dsl::user_id.eq(user_id_param))))
            .left_outer_join(comment_flags.on(self::comment_flags::dsl::comment_id.eq(self::comments::dsl::id).and(self::comment_flags::dsl::user_id.eq(user_id_param))))
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
                self::users::dsl::username,
            ))
            .filter(visible.eq(true))
            .filter(self::comments::dsl::id.eq(comment_id_value))
            .get_result::<(i32, String, String, bool, i32, NaiveDateTime, i32, Option<i32>, Option<i32>, String)>(&self.0)
            .map(|t| tuple_to_comment_info(self, t))
    }
    pub fn create_domain(&self, new_domain: NewDomain) -> Result<Domain, DieselError> {
        use self::domains::dsl::*;
        diesel::insert_into(domains)
            .values(new_domain)
            .get_result(&self.0)
    }
    pub fn get_domain_by_id(&self, domain_id_value: i32) -> Result<Domain, DieselError> {
        use self::domains::dsl::*;
        domains.find(domain_id_value).get_result::<Domain>(&self.0)
    }
    pub fn get_domain_by_hostname(&self, mut hostname_value: &str) -> Result<Domain, DieselError> {
        use self::domains::dsl::*;
        use self::domain_synonyms::*;
        if hostname_value.starts_with("www.") {
            hostname_value = &hostname_value[4..];
        }
        if let Ok(domain_synonym) = domain_synonyms::table.filter(from_hostname.eq(hostname_value)).get_result::<DomainSynonym>(&self.0) {
            domains.filter(id.eq(domain_synonym.to_domain_id)).get_result::<Domain>(&self.0)
        } else {
            domains.filter(hostname.eq(hostname_value)).get_result::<Domain>(&self.0)
        }
    }
    pub fn comment_on_post(&self, new_post: NewComment, body_format: BodyFormat) -> Result<Comment, DieselError> {
        #[derive(Insertable)]
        #[table_name="comments"]
        struct CreateComment<'a> {
            text: &'a str,
            html: &'a str,
            post_id: i32,
            created_by: i32,
            visible: bool,
        }
        let html_and_stuff = match body_format {
            BodyFormat::Plain => crate::prettify::prettify_body(&new_post.text, &mut PrettifyData::new(self, new_post.post_id)),
            BodyFormat::BBCode => crate::prettify::prettify_body_bbcode(&new_post.text, &mut PrettifyData::new(self, new_post.post_id)),
        };
        self.update_comment_count_on_post(new_post.post_id, 1)?;
        diesel::insert_into(comments::table)
            .values(CreateComment{
                text: new_post.text,
                html: &html_and_stuff.string,
                post_id: new_post.post_id,
                created_by: new_post.created_by,
                visible: new_post.visible,
            })
            .get_result(&self.0)
    }
    pub fn update_comment(&self, post_id_value: i32, comment_id_value: i32, text_value: &str, body_format: BodyFormat) -> Result<(), DieselError> {
        let html_and_stuff = match body_format {
            BodyFormat::Plain => crate::prettify::prettify_body(text_value, &mut PrettifyData::new(self, post_id_value)),
            BodyFormat::BBCode => crate::prettify::prettify_body_bbcode(text_value, &mut PrettifyData::new(self, post_id_value)),
        };
        use self::comments::dsl::*;
        diesel::update(comments.find(comment_id_value))
            .set((
                text.eq(text_value),
                html.eq(&html_and_stuff.string)
                ))
            .execute(&self.0)
            .map(|_| ())
    }
    pub fn get_comments_from_post(&self, post_id_param: i32, user_id_param: i32) -> Result<Vec<CommentInfo>, DieselError> {
        use self::comments::dsl::*;
        use self::comment_stars::dsl::*;
        use self::comment_flags::dsl::*;
        use self::users::dsl::*;
        let all: Vec<CommentInfo> = comments
            .left_outer_join(comment_stars.on(self::comment_stars::dsl::comment_id.eq(self::comments::dsl::id).and(self::comment_stars::dsl::user_id.eq(user_id_param))))
            .left_outer_join(comment_flags.on(self::comment_flags::dsl::comment_id.eq(self::comments::dsl::id).and(self::comment_flags::dsl::user_id.eq(user_id_param))))
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
                self::users::dsl::username,
            ))
            .filter(visible.eq(true))
            .filter(self::comments::dsl::post_id.eq(post_id_param))
            .order_by(self::comments::dsl::created_at)
            .get_results::<(i32, String, String, bool, i32, NaiveDateTime, i32, Option<i32>, Option<i32>, String)>(&self.0)?
            .into_iter()
            .map(|t| tuple_to_comment_info(self, t))
            .collect();
        Ok(all)
    }
    pub fn search_comments_by_user(&self, user_id_param: i32) -> Result<Vec<CommentSearchResult>, DieselError> {
        use self::comments::dsl::*;
        use self::users::dsl::*;
        use self::posts::dsl::*;
        let all: Vec<CommentSearchResult> = comments
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
            ))
            .filter(self::comments::dsl::visible.eq(true))
            .filter(self::posts::dsl::private.eq(false))
            .filter(self::posts::dsl::visible.eq(true))
            .filter(self::comments::dsl::created_by.eq(user_id_param))
            .order_by(self::comments::dsl::id.desc())
            .limit(50)
            .get_results::<(i32, String, i32, Base32, String, NaiveDateTime, i32, String)>(&self.0)?
            .into_iter()
            .map(|t| tuple_to_comment_search_results(t))
            .collect();
        Ok(all)
    }
    pub fn search_comments_by_user_after(&self, user_id_param: i32, after_id_param: i32) -> Result<Vec<CommentSearchResult>, DieselError> {
        use self::comments::dsl::*;
        use self::users::dsl::*;
        use self::posts::dsl::*;
        let all: Vec<CommentSearchResult> = comments
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
            ))
            .filter(self::comments::dsl::visible.eq(true))
            .filter(self::posts::dsl::private.eq(false))
            .filter(self::posts::dsl::visible.eq(true))
            .filter(self::comments::dsl::created_by.eq(user_id_param))
            .filter(self::comments::dsl::id.lt(after_id_param))
            .order_by(self::comments::dsl::id.desc())
            .limit(50)
            .get_results::<(i32, String, i32, Base32, String, NaiveDateTime, i32, String)>(&self.0)?
            .into_iter()
            .map(|t| tuple_to_comment_search_results(t))
            .collect();
        Ok(all)
    }
    pub fn get_post_info_from_comment(&self, user_id_param: i32, comment_id_param: i32) -> Result<PostInfo, DieselError> {
        use self::posts::dsl::*;
        use self::stars::dsl::*;
        use self::flags::dsl::*;
        use self::users::dsl::*;
        use self::comments::dsl::*;
        // This is a bunch of duplicate code.
        let mut data = PrettifyData::new(self, 0);
        Ok(tuple_to_post_info(&mut data, posts
            .left_outer_join(stars.on(self::stars::dsl::post_id.eq(self::posts::dsl::id).and(self::stars::dsl::user_id.eq(user_id_param))))
            .left_outer_join(flags.on(self::flags::dsl::post_id.eq(self::posts::dsl::id).and(self::flags::dsl::user_id.eq(user_id_param))))
            .inner_join(users)
            .inner_join(comments)
            .select((
                self::posts::dsl::id,
                self::posts::dsl::uuid,
                self::posts::dsl::title,
                self::posts::dsl::url,
                self::posts::dsl::visible,
                self::posts::dsl::private,
                self::posts::dsl::initial_stellar_time,
                self::posts::dsl::score,
                self::posts::dsl::comment_count,
                self::posts::dsl::authored_by_submitter,
                self::posts::dsl::created_at,
                self::posts::dsl::submitted_by,
                self::posts::dsl::excerpt_html,
                self::stars::dsl::post_id.nullable(),
                self::flags::dsl::post_id.nullable(),
                self::users::dsl::username,
                self::posts::dsl::banner_title,
                self::posts::dsl::banner_desc,
            ))
            .filter(self::comments::dsl::id.eq(comment_id_param))
            .first::<(i32, Base32, String, Option<String>, bool, bool, i32, i32, i32, bool, NaiveDateTime, i32, Option<String>, Option<i32>, Option<i32>, String, Option<String>, Option<String>)>(&self.0)?, self.get_current_stellar_time()))
    }
    pub fn get_post_starred_by(&self, post_id_param: i32) -> Result<Vec<String>, DieselError> {
        use self::stars::dsl::*;
        use self::users::dsl::*;
        let all: Vec<String> = stars
            .inner_join(users)
            .select((
                self::users::dsl::username,
            ))
            .filter(self::stars::dsl::post_id.eq(post_id_param))
            .limit(50)
            .get_results::<(String,)>(&self.0)?
            .into_iter()
            .map(|(t,)| t)
            .collect();
        Ok(all)
    }
    pub fn get_comment_starred_by(&self, comment_id_param: i32) -> Result<Vec<String>, DieselError> {
        use self::comment_stars::dsl::*;
        use self::users::dsl::*;
        let all: Vec<String> = comment_stars
            .inner_join(users)
            .select((
                self::users::dsl::username,
            ))
            .filter(self::comment_stars::dsl::comment_id.eq(comment_id_param))
            .limit(50)
            .get_results::<(String,)>(&self.0)?
            .into_iter()
            .map(|(t,)| t)
            .collect();
        Ok(all)
    }
    pub fn get_tag_by_name(&self, name_param: &str) -> Result<Tag, DieselError> {
        use self::tags::dsl::*;
        tags
            .filter(name.eq(name_param))
            .get_result::<Tag>(&self.0)
    }
    pub fn create_or_update_tag(&self, new_tag: &NewTag) -> Result<Tag, DieselError> {
        #[derive(Insertable)]
        #[table_name="tags"]
        struct CreateTag<'a> {
            name: &'a str,
            description: Option<&'a str>,
        }
        if let Ok(tag) = self.get_tag_by_name(new_tag.name) {
            use self::tags::dsl::*;
            diesel::update(tags.find(tag.id))
                .set(description.eq(new_tag.description))
                .get_result(&self.0)
        } else {
            diesel::insert_into(tags::table)
                .values(CreateTag {
                    name: new_tag.name,
                    description: new_tag.description,
                })
                .get_result(&self.0)
        }
    }
    pub fn add_domain_synonym(&self, new_domain_synonym: &DomainSynonym) -> Result<(), DieselError> {
        use self::posts::dsl::*;
        use self::domain_synonyms::dsl::*;
        use self::domains::dsl::*;
        if let Ok(old_domain) = self.get_domain_by_hostname(&new_domain_synonym.from_hostname) {
            if old_domain.hostname == new_domain_synonym.from_hostname {
                diesel::update(posts.filter(domain_id.eq(old_domain.id)))
                    .set(domain_id.eq(new_domain_synonym.to_domain_id))
                    .execute(&self.0)?;
                diesel::delete(domains.find(old_domain.id))
                    .execute(&self.0)?;
            }
        }
        if let Ok(old_domain_synonym) = domain_synonyms.find(&new_domain_synonym.from_hostname).get_result::<DomainSynonym>(&self.0) {
            diesel::update(domain_synonyms.find(old_domain_synonym.from_hostname))
                .set(to_domain_id.eq(new_domain_synonym.to_domain_id))
                .execute(&self.0)
                .map(|_| ())
        } else {
            diesel::insert_into(domain_synonyms)
                .values(new_domain_synonym)
                .execute(&self.0)
                .map(|_| ())
        }
    }
    pub fn get_all_domain_synonyms(&self) -> Result<Vec<DomainSynonymInfo>, DieselError> {
        use self::domain_synonyms::dsl::*;
        use self::domains::dsl::*;
        domain_synonyms
            .inner_join(domains)
            .select((from_hostname, hostname))
            .get_results::<DomainSynonymInfo>(&self.0)
    }
    pub fn get_all_tags(&self) -> Result<Vec<Tag>, DieselError> {
        use self::tags::dsl::*;
        tags.get_results::<Tag>(&self.0)
    }
    pub fn set_dark_mode(&self, user_id_value: i32, dark_mode_value: bool) -> Result<(), DieselError> {
        use self::users::dsl::*;
        diesel::update(users.find(user_id_value))
            .set(dark_mode.eq(dark_mode_value))
            .execute(&self.0)
            .map(|_| ())
    }
    pub fn set_big_mode(&self, user_id_value: i32, big_mode_value: bool) -> Result<(), DieselError> {
        use self::users::dsl::*;
        diesel::update(users.find(user_id_value))
            .set(big_mode.eq(big_mode_value))
            .execute(&self.0)
            .map(|_| ())
    }
    pub fn get_mod_log_recent(&self) -> Result<Vec<ModerationInfo>, DieselError> {
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
            .get_results::<(i32, json::Value, NaiveDateTime, i32, String)>(&self.0)?
            .into_iter()
            .map(|t| tuple_to_moderation(t))
            .collect()
        )
    }
    pub fn get_mod_log_starting_with(&self, starting_with_id: i32) -> Result<Vec<ModerationInfo>, DieselError> {
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
            .get_results::<(i32, json::Value, NaiveDateTime, i32, String)>(&self.0)?
            .into_iter()
            .map(|t| tuple_to_moderation(t))
            .collect()
        )
    }
    pub fn mod_log_edit_comment(
        &self,
        user_id_value: i32,
        comment_id_value: i32,
        post_uuid_value: Base32,
        old_text_value: &str,
        new_text_value: &str
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
            .execute(&self.0)
            .map(|_| ())
    }
    pub fn mod_log_edit_post(
        &self,
        user_id_value: i32,
        post_uuid_value: Base32,
        old_title_value: &str,
        new_title_value: &str,
        old_url_value: &str,
        new_url_value: &str,
        old_excerpt_value: &str,
        new_excerpt_value: &str,
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
            .execute(&self.0)
            .map(|_| ())
    }
    pub fn mod_log_delete_comment(
        &self,
        user_id_value: i32,
        comment_id_value: i32,
        post_uuid_value: Base32,
        old_text_value: &str,
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
            .execute(&self.0)
            .map(|_| ())
    }
    pub fn mod_log_delete_post(
        &self,
        user_id_value: i32,
        post_uuid_value: Base32,
        old_title_value: &str,
        old_url_value: &str,
        old_excerpt_value: &str,
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
            .execute(&self.0)
            .map(|_| ())
    }
    pub fn mod_log_approve_comment(
        &self,
        user_id_value: i32,
        comment_id_value: i32,
        post_uuid_value: Base32,
        new_text_value: &str,
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
            .execute(&self.0)
            .map(|_| ())
    }
    pub fn mod_log_approve_post(
        &self,
        user_id_value: i32,
        post_uuid_value: Base32,
        new_title_value: &str,
        new_url_value: &str,
        new_excerpt_value: &str,
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
            .execute(&self.0)
            .map(|_| ())
    }
    pub fn mod_log_banner_post(
        &self,
        user_id_value: i32,
        post_uuid_value: Base32,
        banner_title_value: &str,
        banner_desc_value: &str,
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
            .execute(&self.0)
            .map(|_| ())
    }
    pub fn find_moderated_post(&self, user_id_param: i32) -> Option<PostInfo> {
        use self::posts::dsl::*;
        use self::stars::dsl::*;
        use self::flags::dsl::*;
        use self::users::dsl::*;
        let mut data = PrettifyData::new(self, 0);
        let mut all: Vec<PostInfo> = posts
            .left_outer_join(stars.on(self::stars::dsl::post_id.eq(self::posts::dsl::id).and(self::stars::dsl::user_id.eq(user_id_param))))
            .left_outer_join(flags.on(self::flags::dsl::post_id.eq(self::posts::dsl::id).and(self::flags::dsl::user_id.eq(user_id_param))))
            .inner_join(users)
            .select((
                self::posts::dsl::id,
                self::posts::dsl::uuid,
                self::posts::dsl::title,
                self::posts::dsl::url,
                self::posts::dsl::visible,
                self::posts::dsl::private,
                self::posts::dsl::initial_stellar_time,
                self::posts::dsl::score,
                self::posts::dsl::comment_count,
                self::posts::dsl::authored_by_submitter,
                self::posts::dsl::created_at,
                self::posts::dsl::submitted_by,
                self::posts::dsl::excerpt_html,
                self::stars::dsl::post_id.nullable(),
                self::flags::dsl::post_id.nullable(),
                self::users::dsl::username,
                self::posts::dsl::banner_title,
                self::posts::dsl::banner_desc,
            ))
            .filter(visible.eq(false))
            .filter(rejected.eq(false))
            .order_by(self::posts::dsl::created_at.asc())
            .limit(50)
            .get_results::<(i32, Base32, String, Option<String>, bool, bool, i32, i32, i32, bool, NaiveDateTime, i32, Option<String>, Option<i32>, Option<i32>, String, Option<String>, Option<String>)>(&self.0).ok()?
            .into_iter()
            .map(|t| tuple_to_post_info(&mut data, t, self.get_current_stellar_time()))
            .collect();
        all.sort_by_key(|info| OrderedFloat(-info.hotness));
        all.pop()
    }
    pub fn find_moderated_comment(&self, user_id_param: i32) -> Option<CommentInfo> {
        use self::comments::dsl::*;
        use self::comment_stars::dsl::*;
        use self::comment_flags::dsl::*;
        use self::users::dsl::*;
        let mut all: Vec<CommentInfo> = comments
            .left_outer_join(comment_stars.on(self::comment_stars::dsl::comment_id.eq(self::comments::dsl::id).and(self::comment_stars::dsl::user_id.eq(user_id_param))))
            .left_outer_join(comment_flags.on(self::comment_stars::dsl::comment_id.eq(self::comments::dsl::id).and(self::comment_flags::dsl::user_id.eq(user_id_param))))
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
                self::users::dsl::username,
            ))
            .filter(visible.eq(false))
            .filter(rejected.eq(false))
            .order_by(self::comments::dsl::created_at.asc())
            .limit(50)
            .get_results::<(i32, String, String, bool, i32, NaiveDateTime, i32, Option<i32>, Option<i32>, String)>(&self.0).ok()?
            .into_iter()
            .map(|t| tuple_to_comment_info(self, t))
            .collect();
        all.pop()
    }
    pub fn approve_post(&self, post_id_value: i32) -> Result<(), DieselError> {
        use self::posts::dsl::*;
        diesel::update(posts.find(post_id_value))
            .set((
                visible.eq(true),
                initial_stellar_time.eq(self.get_current_stellar_time()),
            ))
            .execute(&self.0)?;
        Ok(())
    }
    pub fn banner_post(&self, post_id_value: i32, banner_title_value: Option<&str>, banner_desc_value: Option<&str>) -> Result<(), DieselError> {
        use self::posts::dsl::*;
        diesel::update(posts.find(post_id_value))
            .set((
                banner_title.eq(banner_title_value),
                banner_desc.eq(banner_desc_value),
            ))
            .execute(&self.0)?;
        Ok(())
    }
    pub fn approve_comment(&self, comment_id_value: i32) -> Result<(), DieselError> {
        use self::comments::dsl::*;
        diesel::update(comments.find(comment_id_value))
            .set((
                visible.eq(true),
            ))
            .execute(&self.0)?;
        Ok(())
    }
    pub fn hide_post(&self, post_id_value: i32) -> Result<(), DieselError> {
        use self::posts::dsl::*;
        diesel::update(posts.find(post_id_value))
            .set((
                visible.eq(false),
            ))
            .execute(&self.0)?;
        Ok(())
    }
    pub fn hide_comment(&self, comment_id_value: i32) -> Result<(), DieselError> {
        use self::comments::dsl::*;
        diesel::update(comments.find(comment_id_value))
            .set((
                visible.eq(false),
            ))
            .execute(&self.0)?;
        Ok(())
    }
    pub fn delete_post(&self, post_id_value: i32) -> Result<(), DieselError> {
        use self::posts::dsl::*;
        diesel::update(posts.find(post_id_value))
            .set((
                visible.eq(false),
                rejected.eq(true),
            ))
            .execute(&self.0)?;
        Ok(())
    }
    pub fn delete_comment(&self, comment_id_value: i32) -> Result<(), DieselError> {
        use self::comments::dsl::*;
        diesel::update(comments.find(comment_id_value))
            .set((
                visible.eq(false),
                rejected.eq(true),
            ))
            .execute(&self.0)?;
        Ok(())
    }
    pub fn user_has_received_a_star(&self, user_id_param: i32) -> bool {
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
        select(exists(post_star)).get_result(&self.0).unwrap_or(false) ||
            select(exists(comment_star)).get_result(&self.0).unwrap_or(false)
    }
    pub fn maximum_post_id(&self) -> i32 {
        use self::posts::dsl::*;
        use diesel::dsl::max;
        posts.select(max(id)).get_result::<Option<i32>>(&self.0).unwrap_or(Some(0)).unwrap_or(0)
    }
    pub fn random_post(&self) -> Result<Option<Post>, DieselError> {
        use diesel::sql_query;
        sql_query("SELECT * FROM posts ORDER BY RANDOM() LIMIT 1").load(&self.0).map(|mut x: Vec<_>| x.pop())
    }
    pub fn get_post_by_id(&self, post_id_value: i32) -> Result<Post, DieselError> {
        use self::posts::dsl::*;
        posts.find(post_id_value).get_result::<Post>(&self.0)
    }
    pub fn get_legacy_comments_from_post(&self, post_id_param: i32, _user_id_param: i32) -> Result<Vec<LegacyComment>, DieselError> {
        use self::legacy_comments::dsl::*;
        let all: Vec<LegacyComment> = legacy_comments
            .filter(self::legacy_comments::dsl::post_id.eq(post_id_param))
            .order_by(self::legacy_comments::dsl::created_at)
            .get_results(&self.0)?;
        Ok(all)
    }
    pub fn get_legacy_comment_by_id(&self, legacy_comment_id_value: i32) -> Result<LegacyComment, DieselError> {
        use self::legacy_comments::dsl::*;
        legacy_comments.find(legacy_comment_id_value).get_result::<LegacyComment>(&self.0)
    }
    pub fn update_legacy_comment(&self, post_id_value: i32, legacy_comment_id_value: i32, text_value: &str, body_format: BodyFormat) -> Result<(), DieselError> {
        let html_and_stuff = match body_format {
            BodyFormat::Plain => crate::prettify::prettify_body(text_value, &mut PrettifyData::new(self, post_id_value)),
            BodyFormat::BBCode => crate::prettify::prettify_body_bbcode(text_value, &mut PrettifyData::new(self, post_id_value)),
        };
        use self::legacy_comments::dsl::*;
        diesel::update(legacy_comments.find(legacy_comment_id_value))
            .set((
                text.eq(text_value),
                html.eq(&html_and_stuff.string)
            ))
            .execute(&self.0)
            .map(|_| ())
    }
    pub fn create_session(&self, user_id: i32, user_agent: &str) -> Result<UserSession, DieselError> {
        #[derive(Insertable)]
        #[table_name="user_sessions"]
        struct CreateSession<'a> {
            uuid: i64,
            user_agent: &'a str,
            user_id: i32,
        }
        let uuid = rand::random();
        diesel::insert_into(user_sessions::table)
            .values(CreateSession {
                uuid, user_agent, user_id
            })
            .get_result::<UserSession>(&self.0)
    }
    pub fn get_session_by_uuid(&self, base32: Base32) -> Result<UserSession, DieselError> {
        use self::user_sessions::dsl::*;
        user_sessions.filter(uuid.eq(base32.into_i64())).get_result(&self.0)
    }
}

fn tuple_to_notification_info((post_uuid, post_title, comment_count, from_username): (Base32, String, i32, String)) -> NotificationInfo {
    NotificationInfo {
        post_uuid, post_title, comment_count, from_username,
    }
}

fn tuple_to_post_info(data: &mut PrettifyData, (id, uuid, title, url, visible, private, initial_stellar_time, score, comment_count, authored_by_submitter, created_at, submitted_by, excerpt_html, starred_post_id, flagged_post_id, submitted_by_username, banner_title, banner_desc): (i32, Base32, String, Option<String>, bool, bool, i32, i32, i32, bool, NaiveDateTime, i32, Option<String>, Option<i32>, Option<i32>, String, Option<String>, Option<String>), current_stellar_time: i32) -> PostInfo {
    let link_url = if let Some(ref url) = url {
        url.clone()
    } else {
        uuid.to_string()
    };
    let title_html_output = prettify_title(&title, &link_url, data);
    let title_html = title_html_output.string;
    let created_at_relative = relative_date(&created_at);
    PostInfo {
        id, uuid, title, url, visible, private, score, authored_by_submitter,
        submitted_by, submitted_by_username, comment_count, title_html,
        excerpt_html, banner_title, banner_desc,
        created_at, created_at_relative,
        starred_by_me: starred_post_id.is_some(),
        flagged_by_me: flagged_post_id.is_some(),
        hotness: compute_hotness(initial_stellar_time, current_stellar_time, score, authored_by_submitter)
    }
}

fn tuple_to_comment_info(conn: &MoreInterestingConn, (id, text, html, visible, post_id, created_at, created_by, starred_comment_id, flagged_comment_id, created_by_username): (i32, String, String, bool, i32, NaiveDateTime, i32, Option<i32>, Option<i32>, String)) -> CommentInfo {
    let created_at_relative = relative_date(&created_at);
    CommentInfo {
        id, text, html, visible, post_id, created_by, created_by_username,
        created_at, created_at_relative,
        starred_by: conn.get_comment_starred_by(id).unwrap_or(Vec::new()),
        starred_by_me: starred_comment_id.is_some(),
        flagged_by_me: flagged_comment_id.is_some(),
    }
}

fn tuple_to_comment_search_results((id, html, post_id, post_uuid, post_title, created_at, created_by, created_by_username): (i32, String, i32, Base32, String, NaiveDateTime, i32, String)) -> CommentSearchResult {
    let created_at_relative = relative_date(&created_at);
    CommentSearchResult {
        id, html, post_id, post_uuid, post_title, created_by, created_by_username,
        created_at, created_at_relative,
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

fn compute_hotness(initial_stellar_time: i32, current_stellar_time: i32, score: i32, authored_by_submitter: bool) -> f64 {
    let gravity = 1.33;
    let boost = if authored_by_submitter { 0.33 } else { 0.0 };
    let stellar_age = max(current_stellar_time - initial_stellar_time, 0) as f64;
    (boost + (score as f64) + 1.0) / (stellar_age + 1.0).powf(gravity)
}

fn relative_date(dt: &NaiveDateTime) -> String {
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
    use chrono::Utc;
    use chrono_humanize::{Accuracy, HumanTime, Tense};
    let h = HumanTime::from(*dt - Utc::now().naive_utc());
    v_htmlescape::escape(&h.to_text_en(Accuracy::Rough, Tense::Past)).to_string()
}

pub struct PrettifyData<'a> {
    conn: &'a MoreInterestingConn,
    post_id: i32,
    has_tag_cache: HashSet<String>,
    has_user_cache: HashSet<String>,
    domain_map_cache: HashMap<String, String>,
}
impl<'a> PrettifyData<'a> {
    pub fn new(conn: &'a MoreInterestingConn, post_id: i32) -> PrettifyData<'a> {
        PrettifyData {
            conn, post_id,
            has_tag_cache: HashSet::new(),
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
            if let Ok(comment) = self.conn.get_comment_by_id(comment_id) {
                comment.post_id == self.post_id
            } else {
                false
            }
        }
    }
    fn check_hash_tag(&mut self, tag: &str) -> bool {
        if self.has_tag_cache.contains(tag) {
            true
        } else {
            let has_tag = self.conn.get_tag_by_name(tag).is_ok();
            if has_tag {
                self.has_tag_cache.insert(tag.to_string());
            }
            has_tag
        }
    }
    fn check_username(&mut self, username: &str) -> bool {
        if self.has_user_cache.contains(username) {
            true
        } else {
            let has_user = self.conn.get_user_by_username(username).is_ok();
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
            conn.get_domain_by_hostname(hostname)
                .map(|domain| domain.hostname)
                .unwrap_or_else(|_| hostname.to_owned())
        }).clone()
    }
}
