use rocket_contrib::databases::diesel::PgConnection;
use diesel::prelude::*;
use diesel::result::Error as DieselError;
use chrono::NaiveDateTime;
use crate::schema::{users, posts, stars, invite_tokens, comments, comment_stars, tags, post_tagging, moderation, flags, comment_flags, domains};
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
}

#[derive(Queryable, Serialize)]
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

pub struct NewPost<'a> {
    pub title: &'a str,
    pub url: Option<&'a str>,
    pub excerpt: Option<&'a str>,
    pub submitted_by: i32,
    pub visible: bool,
}

#[derive(Serialize)]
pub struct PostInfo {
    pub id: i32,
    pub uuid: Base32,
    pub title: String,
    pub title_html: String,
    pub url: Option<String>,
    pub visible: bool,
    pub hotness: f64,
    pub score: i32,
    pub comment_count: i32,
    pub authored_by_submitter: bool,
    pub created_at: NaiveDateTime,
    pub submitted_by: i32,
    pub submitted_by_username: String,
    pub starred_by_me: bool,
    pub flagged_by_me: bool,
    pub excerpt: Option<String>,
    pub excerpt_html: Option<String>,
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
    pub created_by: i32,
    pub created_by_username: String,
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

#[database("more_interesting")]
pub struct MoreInterestingConn(PgConnection);

impl MoreInterestingConn {
    pub fn get_post_info_recent(&self, user_id_param: i32) -> Result<Vec<PostInfo>, DieselError> {
        use self::posts::dsl::*;
        use self::stars::dsl::*;
        use self::flags::dsl::*;
        use self::users::dsl::*;
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
                self::posts::dsl::initial_stellar_time,
                self::posts::dsl::score,
                self::posts::dsl::comment_count,
                self::posts::dsl::authored_by_submitter,
                self::posts::dsl::created_at,
                self::posts::dsl::submitted_by,
                self::posts::dsl::excerpt,
                self::posts::dsl::excerpt_html,
                self::stars::dsl::post_id.nullable(),
                self::flags::dsl::post_id.nullable(),
                self::users::dsl::username,
            ))
            .filter(visible.eq(true))
            .order_by((initial_stellar_time.desc(), self::posts::dsl::created_at.desc()))
            .limit(200)
            .get_results::<(i32, Base32, String, Option<String>, bool, i32, i32, i32, bool, NaiveDateTime, i32, Option<String>, Option<String>, Option<i32>, Option<i32>, String)>(&self.0)?
            .into_iter()
            .map(|t| tuple_to_post_info(self, t, self.get_current_stellar_time()))
            .collect();
        all.sort_by_key(|info| OrderedFloat(-info.hotness));
        all.truncate(100);
        Ok(all)
    }
    pub fn get_post_info_recent_by_tag(&self, user_id_param: i32, tag_id_param: i32) -> Result<Vec<PostInfo>, DieselError> {
        use self::posts::dsl::*;
        use self::stars::dsl::*;
        use self::flags::dsl::*;
        use self::users::dsl::*;
        use self::post_tagging::dsl::*;
        let mut all: Vec<PostInfo> = posts
            .left_outer_join(stars.on(self::stars::dsl::post_id.eq(self::posts::dsl::id).and(self::stars::dsl::user_id.eq(user_id_param))))
            .left_outer_join(flags.on(self::flags::dsl::post_id.eq(self::posts::dsl::id).and(self::flags::dsl::user_id.eq(user_id_param))))
            .inner_join(users)
            .inner_join(post_tagging)
            .select((
                self::posts::dsl::id,
                self::posts::dsl::uuid,
                self::posts::dsl::title,
                self::posts::dsl::url,
                self::posts::dsl::visible,
                self::posts::dsl::initial_stellar_time,
                self::posts::dsl::score,
                self::posts::dsl::comment_count,
                self::posts::dsl::authored_by_submitter,
                self::posts::dsl::created_at,
                self::posts::dsl::submitted_by,
                self::posts::dsl::excerpt,
                self::posts::dsl::excerpt_html,
                self::stars::dsl::post_id.nullable(),
                self::flags::dsl::post_id.nullable(),
                self::users::dsl::username,
            ))
            .filter(visible.eq(true))
            .filter(self::post_tagging::tag_id.eq(tag_id_param))
            .order_by((initial_stellar_time.desc(), self::posts::dsl::created_at.desc()))
            .limit(200)
            .get_results::<(i32, Base32, String, Option<String>, bool, i32, i32, i32, bool, NaiveDateTime, i32, Option<String>, Option<String>, Option<i32>, Option<i32>, String)>(&self.0)?
            .into_iter()
            .map(|t| tuple_to_post_info(self, t, self.get_current_stellar_time()))
            .collect();
        all.sort_by_key(|info| OrderedFloat(-info.hotness));
        all.truncate(100);
        Ok(all)
    }
    pub fn get_post_info_recent_by_domain(&self, user_id_param: i32, domain_id_param: i32) -> Result<Vec<PostInfo>, DieselError> {
        use self::posts::dsl::*;
        use self::stars::dsl::*;
        use self::flags::dsl::*;
        use self::users::dsl::*;
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
                self::posts::dsl::initial_stellar_time,
                self::posts::dsl::score,
                self::posts::dsl::comment_count,
                self::posts::dsl::authored_by_submitter,
                self::posts::dsl::created_at,
                self::posts::dsl::submitted_by,
                self::posts::dsl::excerpt,
                self::posts::dsl::excerpt_html,
                self::stars::dsl::post_id.nullable(),
                self::flags::dsl::post_id.nullable(),
                self::users::dsl::username,
            ))
            .filter(visible.eq(true))
            .filter(domain_id.eq(domain_id_param))
            .order_by((initial_stellar_time.desc(), self::posts::dsl::created_at.desc()))
            .limit(200)
            .get_results::<(i32, Base32, String, Option<String>, bool, i32, i32, i32, bool, NaiveDateTime, i32, Option<String>, Option<String>, Option<i32>, Option<i32>, String)>(&self.0)?
            .into_iter()
            .map(|t| tuple_to_post_info(self, t, self.get_current_stellar_time()))
            .collect();
        all.sort_by_key(|info| OrderedFloat(-info.hotness));
        all.truncate(100);
        Ok(all)
    }
    pub fn get_post_info_search(&self, user_id_param: i32, search: &str) -> Result<Vec<PostInfo>, DieselError> {
        use self::posts::dsl::*;
        use self::stars::dsl::*;
        use self::flags::dsl::*;
        use self::users::dsl::*;
        use crate::schema::post_search_index::dsl::*;
        use diesel_full_text_search::{plainto_tsquery, TsVectorExtensions};
        let mut all: Vec<PostInfo> = posts
            .left_outer_join(stars.on(self::stars::dsl::post_id.eq(self::posts::dsl::id).and(self::stars::dsl::user_id.eq(user_id_param))))
            .left_outer_join(flags.on(self::flags::dsl::post_id.eq(self::posts::dsl::id).and(self::flags::dsl::user_id.eq(user_id_param))))
            .inner_join(users)
            .inner_join(post_search_index)
            .select((
                self::posts::dsl::id,
                self::posts::dsl::uuid,
                self::posts::dsl::title,
                self::posts::dsl::url,
                self::posts::dsl::visible,
                self::posts::dsl::initial_stellar_time,
                self::posts::dsl::score,
                self::posts::dsl::comment_count,
                self::posts::dsl::authored_by_submitter,
                self::posts::dsl::created_at,
                self::posts::dsl::submitted_by,
                self::posts::dsl::excerpt,
                self::posts::dsl::excerpt_html,
                self::stars::dsl::post_id.nullable(),
                self::flags::dsl::post_id.nullable(),
                self::users::dsl::username,
            ))
            .filter(visible.eq(true))
            .filter(search_index.matches(plainto_tsquery(search)))
            .order_by((initial_stellar_time.desc(), self::posts::dsl::created_at.desc()))
            .limit(200)
            .get_results::<(i32, Base32, String, Option<String>, bool, i32, i32, i32, bool, NaiveDateTime, i32, Option<String>, Option<String>, Option<i32>, Option<i32>, String)>(&self.0)?
            .into_iter()
            .map(|t| tuple_to_post_info(self, t, self.get_current_stellar_time()))
            .collect();
        all.sort_by_key(|info| OrderedFloat(-info.hotness));
        all.truncate(200);
        Ok(all)
    }
    pub fn get_post_info_recent_by_user(&self, user_id_param: i32, user_info_id_param: i32) -> Result<Vec<PostInfo>, DieselError> {
        use self::posts::dsl::*;
        use self::stars::dsl::*;
        use self::flags::dsl::*;
        use self::users::dsl::*;
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
                self::posts::dsl::initial_stellar_time,
                self::posts::dsl::score,
                self::posts::dsl::comment_count,
                self::posts::dsl::authored_by_submitter,
                self::posts::dsl::created_at,
                self::posts::dsl::submitted_by,
                self::posts::dsl::excerpt,
                self::posts::dsl::excerpt_html,
                self::stars::dsl::post_id.nullable(),
                self::flags::dsl::post_id.nullable(),
                self::users::dsl::username,
            ))
            .filter(visible.eq(true))
            .filter(self::posts::dsl::submitted_by.eq(user_info_id_param))
            .order_by((initial_stellar_time.desc(), self::posts::dsl::created_at.desc()))
            .limit(200)
            .get_results::<(i32, Base32, String, Option<String>, bool, i32, i32, i32, bool, NaiveDateTime, i32, Option<String>, Option<String>, Option<i32>, Option<i32>, String)>(&self.0)?
            .into_iter()
            .map(|t| tuple_to_post_info(self, t, self.get_current_stellar_time()))
            .collect();
        all.sort_by_key(|info| OrderedFloat(-info.hotness));
        all.truncate(100);
        Ok(all)
    }
    pub fn get_post_info_by_uuid(&self, user_id_param: i32, uuid_param: Base32) -> Result<PostInfo, DieselError> {
        use self::posts::dsl::*;
        use self::stars::dsl::*;
        use self::flags::dsl::*;
        use self::users::dsl::*;
        // This is a bunch of duplicate code.
        Ok(tuple_to_post_info(self, posts
            .left_outer_join(stars.on(self::stars::dsl::post_id.eq(self::posts::dsl::id).and(self::stars::dsl::user_id.eq(user_id_param))))
            .left_outer_join(flags.on(self::flags::dsl::post_id.eq(self::posts::dsl::id).and(self::flags::dsl::user_id.eq(user_id_param))))
            .inner_join(users)
            .select((
                self::posts::dsl::id,
                self::posts::dsl::uuid,
                self::posts::dsl::title,
                self::posts::dsl::url,
                self::posts::dsl::visible,
                self::posts::dsl::initial_stellar_time,
                self::posts::dsl::score,
                self::posts::dsl::comment_count,
                self::posts::dsl::authored_by_submitter,
                self::posts::dsl::created_at,
                self::posts::dsl::submitted_by,
                self::posts::dsl::excerpt,
                self::posts::dsl::excerpt_html,
                self::stars::dsl::post_id.nullable(),
                self::flags::dsl::post_id.nullable(),
                self::users::dsl::username,
            ))
            .filter(rejected.eq(false))
            .filter(uuid.eq(uuid_param.into_i64()))
            .first::<(i32, Base32, String, Option<String>, bool, i32, i32, i32, bool, NaiveDateTime, i32, Option<String>, Option<String>, Option<i32>, Option<i32>, String)>(&self.0)?, self.get_current_stellar_time()))
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
    pub fn create_post(&self, new_post: &NewPost) -> Result<Post, DieselError> {
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
            domain_id: Option<i32>,
        }
        let title_html_and_stuff = crate::prettify::prettify_title(new_post.title, "", &mut PrettifyData(self, 0));
        let excerpt_html_and_stuff = if let Some(excerpt) = new_post.excerpt {
            Some(crate::prettify::prettify_body(excerpt, &mut PrettifyData(self, 0)))
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
    pub fn update_post(&self, post_id_value: i32, new_post: &NewPost) -> Result<(), DieselError> {
        let title_html_and_stuff = crate::prettify::prettify_title(new_post.title, "", &mut PrettifyData(self, 0));
        let excerpt_html_and_stuff = if let Some(e) = new_post.excerpt {
            Some(crate::prettify::prettify_body(e, &mut PrettifyData(self, 0)))
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
                domain_id.eq(domain.map(|d| d.id))
            ))
            .execute(&self.0)?;
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
        if hostname_value.starts_with("www.") {
            hostname_value = &hostname_value[4..];
        }
        domains.filter(hostname.eq(hostname_value)).get_result::<Domain>(&self.0)
    }
    pub fn comment_on_post(&self, new_post: NewComment) -> Result<Comment, DieselError> {
        #[derive(Insertable)]
        #[table_name="comments"]
        struct CreateComment<'a> {
            text: &'a str,
            html: &'a str,
            post_id: i32,
            created_by: i32,
            visible: bool,
        }
        let html_and_stuff = crate::prettify::prettify_body(new_post.text, &mut PrettifyData(self, new_post.post_id));
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
    pub fn update_comment(&self, post_id_value: i32, comment_id_value: i32, text_value: &str) -> Result<(), DieselError> {
        let html_and_stuff = crate::prettify::prettify_body(text_value, &mut PrettifyData(self, post_id_value));
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
            .filter(visible.eq(true))
            .filter(self::comments::dsl::post_id.eq(post_id_param))
            .order_by(self::comments::dsl::created_at)
            .limit(400)
            .get_results::<(i32, String, String, bool, i32, NaiveDateTime, i32, Option<i32>, Option<i32>, String)>(&self.0)?
            .into_iter()
            .map(|t| tuple_to_comment_info(self, t))
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
        Ok(tuple_to_post_info(self, posts
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
                self::posts::dsl::initial_stellar_time,
                self::posts::dsl::score,
                self::posts::dsl::comment_count,
                self::posts::dsl::authored_by_submitter,
                self::posts::dsl::created_at,
                self::posts::dsl::submitted_by,
                self::posts::dsl::excerpt,
                self::posts::dsl::excerpt_html,
                self::stars::dsl::post_id.nullable(),
                self::flags::dsl::post_id.nullable(),
                self::users::dsl::username,
            ))
            .filter(self::comments::dsl::id.eq(comment_id_param))
            .first::<(i32, Base32, String, Option<String>, bool, i32, i32, i32, bool, NaiveDateTime, i32, Option<String>, Option<String>, Option<i32>, Option<i32>, String)>(&self.0)?, self.get_current_stellar_time()))
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
            .limit(400)
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
            .limit(400)
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
            .limit(200)
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
            .limit(200)
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
    pub fn find_moderated_post(&self, user_id_param: i32) -> Option<PostInfo> {
        use self::posts::dsl::*;
        use self::stars::dsl::*;
        use self::flags::dsl::*;
        use self::users::dsl::*;
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
                self::posts::dsl::initial_stellar_time,
                self::posts::dsl::score,
                self::posts::dsl::comment_count,
                self::posts::dsl::authored_by_submitter,
                self::posts::dsl::created_at,
                self::posts::dsl::submitted_by,
                self::posts::dsl::excerpt,
                self::posts::dsl::excerpt_html,
                self::stars::dsl::post_id.nullable(),
                self::flags::dsl::post_id.nullable(),
                self::users::dsl::username,
            ))
            .filter(visible.eq(false))
            .filter(rejected.eq(false))
            .order_by((initial_stellar_time.desc(), self::posts::dsl::created_at.desc()))
            .limit(50)
            .get_results::<(i32, Base32, String, Option<String>, bool, i32, i32, i32, bool, NaiveDateTime, i32, Option<String>, Option<String>, Option<i32>, Option<i32>, String)>(&self.0).ok()?
            .into_iter()
            .map(|t| tuple_to_post_info(self, t, self.get_current_stellar_time()))
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
            .order_by(self::comments::dsl::created_at.desc())
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
        use self::comment_stars::dsl::*;
        use diesel::{select, dsl::exists};
        select(exists(stars.filter(self::stars::dsl::user_id.eq(user_id_param)))).get_result(&self.0).unwrap_or(false) ||
            select(exists(comment_stars.filter(self::comment_stars::dsl::user_id.eq(user_id_param)))).get_result(&self.0).unwrap_or(false)
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
}

fn tuple_to_post_info(conn: &MoreInterestingConn, (id, uuid, title, url, visible, initial_stellar_time, score, comment_count, authored_by_submitter, created_at, submitted_by, excerpt, excerpt_html, starred_post_id, flagged_post_id, submitted_by_username): (i32, Base32, String, Option<String>, bool, i32, i32, i32, bool, NaiveDateTime, i32, Option<String>, Option<String>, Option<i32>, Option<i32>, String), current_stellar_time: i32) -> PostInfo {
    let link_url = if let Some(ref url) = url {
        url.clone()
    } else {
        uuid.to_string()
    };
    let title_html_output = prettify_title(&title, &link_url, &mut PrettifyData(conn, 0));
    let title_html = title_html_output.string;
    PostInfo {
        id, uuid, title, url, visible, score, authored_by_submitter, created_at,
        submitted_by, submitted_by_username, comment_count, title_html,
        excerpt, excerpt_html,
        starred_by_me: starred_post_id.is_some(),
        flagged_by_me: flagged_post_id.is_some(),
        hotness: compute_hotness(initial_stellar_time, current_stellar_time, score, authored_by_submitter)
    }
}

fn tuple_to_comment_info(conn: &MoreInterestingConn, (id, text, html, visible, post_id, created_at, created_by, starred_comment_id, flagged_comment_id, created_by_username): (i32, String, String, bool, i32, NaiveDateTime, i32, Option<i32>, Option<i32>, String)) -> CommentInfo {
    CommentInfo {
        id, text, html, visible, post_id, created_at, created_by, created_by_username,
        starred_by: conn.get_comment_starred_by(id).unwrap_or(Vec::new()),
        starred_by_me: starred_comment_id.is_some(),
        flagged_by_me: flagged_comment_id.is_some(),
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

struct PrettifyData<'a>(&'a MoreInterestingConn, i32);
impl<'a> prettify::Data for PrettifyData<'a> {
    fn check_comment_ref(&mut self, comment_id: i32) -> bool {
        let post_id = self.1;
        if post_id == 0 {
            false
        } else {
            if let Ok(comment) = self.0.get_comment_by_id(comment_id) {
                comment.post_id == post_id
            } else {
                false
            }
        }
    }
    fn check_hash_tag(&mut self, tag: &str) -> bool {
        self.0.get_tag_by_name(tag).is_ok()
    }
    fn check_username(&mut self, username: &str) -> bool {
        self.0.get_user_by_username(username).is_ok()
    }
}