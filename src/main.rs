#![allow(ellipsis_inclusive_range_patterns, unused)]

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate rocket_sync_db_pools;
#[macro_use]
extern crate log;

pub mod template;
mod customization;
mod schema;
mod models;
mod password;
mod session;
mod prettify;
mod pid_file_fairing;
mod sql_types;
mod forever;

use askama::Template;
use forever::CacheForever;
use rocket::response::content::Html;
use rocket::form::{Form, FromForm};
use rocket::request::FlashMessage;
use rocket::response::{Responder, Redirect, Flash, content};
use rocket::http::{CookieJar, Cookie, ContentType};
pub use models::MoreInterestingConn;
use models::PollInfo;
use models::{CreatePostError, CreateCommentError};
use models::{relative_date, PrettifyData, BodyFormat};
use crate::customization::Customization;
use models::User;
use models::UserAuth;
use rocket::http::{SameSite, Status};
use rocket::fs::FileServer;
use serde::{Serialize, Serializer, Deserialize};
use std::borrow::Cow;
use crate::models::{SiteCustomization, NotificationInfo, NewNotification, NewSubscription, PostSearch, PostSearchOrderBy, UserSession, PostInfo, NewStar, NewHide, NewHideComment, NewUser, CommentInfo, NewPost, NewComment, NewStarComment, NewTag, Tag, Comment, ModerationInfo, NewFlag, NewFlagComment, LegacyCommentInfo, CommentSearchResult, DomainSynonym, DomainSynonymInfo, NewDomain};
use crate::template::AdminPageId;
pub use crate::template::ModQueueItem;
use more_interesting_base32::Base32;
use url::Url;
use std::collections::HashMap;
use v_htmlescape::escape;
use crate::pid_file_fairing::PidFileFairing;
use rocket::fairing;
use rocket::State;
use std::str::FromStr;
use crate::session::{LoginSession, ModeratorSession, UserAgentString, ReferrerString};
use chrono::{NaiveDate, Duration, Utc};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use lazy_static::lazy_static;
use regex::Regex;
use more_interesting_avatar::render as render_avatar;
use more_interesting_avatar::to_png;

#[derive(Clone, Deserialize, Serialize)]
pub struct SiteConfig {
    #[serde(default)]
    enable_user_directory: bool,
    #[serde(default)]
    enable_anonymous_submissions: bool,
    #[serde(default)]
    enable_public_signup: bool,
    #[serde(with = "url_serde", default = "make_localhost")]
    public_url: Url,
    #[serde(default)]
    hide_text_post: bool,
    #[serde(default)]
    hide_link_post: bool,
    #[serde(default)]
    body_format: models::BodyFormat,
    #[serde(default)]
    pid_file: String,
    #[serde(default)]
    init_username: String,
    #[serde(default)]
    init_password: String,
}

fn make_localhost() -> Url {
    Url::parse("http://localhost").unwrap()
}

impl Default for SiteConfig {
    fn default() -> Self {
        SiteConfig {
            enable_user_directory: false,
            enable_anonymous_submissions: false,
            enable_public_signup: false,
            public_url: Url::parse("http://localhost").unwrap(),
            hide_text_post: false,
            hide_link_post: false,
            body_format: models::BodyFormat::Plain,
            pid_file: String::new(),
            init_username: String::new(),
            init_password: String::new(),
        }
    }
}

#[derive(Serialize, Default)]
struct TemplateContext {
    next_search_page: i32,
    title: Cow<'static, str>,
    mod_queue: Vec<ModQueueItem>,
    posts: Vec<PostInfo>,
    extra_blog_posts: Vec<PostInfo>,
    starred_by: Vec<String>,
    comments: Vec<CommentInfo>,
    legacy_comments: Vec<LegacyCommentInfo>,
    comment_search_result: Vec<CommentSearchResult>,
    comment: Option<Comment>,
    user: User,
    session: UserSession,
    alert: Option<String>,
    invite_token: Option<Base32>,
    raw_html: String,
    tags: Vec<Tag>,
    tag_param: Option<String>,
    domain: Option<String>,
    keywords_param: Option<String>,
    title_param: Option<String>,
    before_date_param: Option<NaiveDate>,
    after_date_param: Option<NaiveDate>,
    config: SiteConfig,
    customization: Customization,
    log: Vec<ModerationInfo>,
    is_home: bool,
    is_me: bool,
    is_private: bool,
    is_subscribed: bool,
    noindex: bool,
    locked: bool,
    blog_post: bool,
    notifications: Vec<NotificationInfo>,
    excerpt: Option<String>,
    comment_preview_text: String,
    comment_preview_html: String,
    polls: Vec<PollInfo>,
    poll_count: usize,
}

#[derive(Serialize, Default)]
struct AdminTemplateContext {
    title: Cow<'static, str>,
    user: User,
    session: UserSession,
    alert: Option<String>,
    tags: Vec<Tag>,
    domain_synonyms: Vec<DomainSynonymInfo>,
    config: SiteConfig,
    page: AdminPageId,
    site_customization: Vec<SiteCustomization>,
    post_flags: Vec<models::PostFlagInfo>,
    comment_flags: Vec<models::CommentFlagInfo>,
    users_list: Vec<models::User>,
    username: String,
}

fn default<T: Default>() -> T { T::default() }

trait ResultTo {
    type Ok;
    /// This function is exactly like `ok()`, except it writes a warning to the log.
    fn into_option(self) -> Option<Self::Ok>;
}

impl<T, E: std::fmt::Debug> ResultTo for Result<T, E> {
    type Ok = T;

    fn into_option(self) -> Option<T> {
        match self {
            Ok(t) => Some(t),
            Err(e) => {
                warn!("database lookup failed: {:?}", e);
                None
            }
        }
    }
}

#[derive(Clone, Copy, FromForm)]
struct MaybeRedirect {
    redirect: Option<Base32>,
}

impl MaybeRedirect {
    pub fn maybe_redirect(self) -> Result<Redirect, Status> {
        match self.redirect {
            Some(b) if b == Base32::zero() => Ok(Redirect::to(".")),
            Some(b) => Ok(Redirect::to(b.to_string())),
            None => Err(Status::Created)
        }
    }
    pub fn maybe_redirect_vote<T: FnOnce() -> String>(self, hash: T) -> VoteResponse {
        let p = match self.redirect {
            Some(b) if b == Base32::zero() => String::from("."),
            Some(b) => b.to_string(),
            None => return VoteResponse::C(Status::Created),
        };
        VoteResponse::B(Html(format!("<!DOCTYPE html><meta name=viewport content=width=device-width><h1><a href={}#{}>Redirect OK</a></h1><script>window.location = document.getElementsByTagName('a')[0].href</script>", escape(&p), escape(&hash()))))
    }
}

#[derive(Responder)]
enum VoteResponse {
    B(Html<String>),
    C(Status)
}

#[derive(Responder)]
enum Either<A, B> {
    A(A),
    B(B),
}

#[derive(Responder)]
enum OneOf<A, B, C, D> {
    A(A),
    B(B),
    C(C),
    D(D),
}

/*
Note on the URL scheme: we cram a lot of stuff into the top-level URL scheme.
It helps keep the URLs short and easy-to-remember.

* `http://example.instance/@notriddle` is the URL of notriddle's profile.
* `http://example.instance/85f844c8` is the URL of a post.
* `http://example.instance/add-star` is an internal URL.
* `http://example.instance/assets` is where the static files are
*/

#[derive(FromForm)]
struct VoteForm {
    rm_star: Option<Base32>,
    add_star: Option<Base32>,
    rm_flag: Option<Base32>,
    add_flag: Option<Base32>,
    rm_hide: Option<Base32>,
    add_hide: Option<Base32>,
}

#[post("/vote?<redirect..>", data = "<p>")]
async fn vote(conn: MoreInterestingConn, login: LoginSession, redirect: MaybeRedirect, p: Form<VoteForm>, customization: Customization) -> VoteResponse {
    let user = login.user;
    let (post, result) = match (p.add_star, p.rm_star, p.add_flag, p.rm_flag, p.add_hide, p.rm_hide) {
        (Some(u), None, None, None, None, None) => {
            let post = match conn.get_post_info_by_uuid(user.id, u).await {
                Ok(post) => post,
                Err(_) => return VoteResponse::C(Status::NotFound),
            };
            let id = post.id;
            (post, conn.add_star(&NewStar {
                user_id: user.id,
                post_id: id,
            }).await)
        }
        (None, Some(u), None, None, None, None) => {
            let post = match conn.get_post_info_by_uuid(user.id, u).await {
                Ok(post) => post,
                Err(_) => return VoteResponse::C(Status::NotFound),
            };
            let id = post.id;
            (post, conn.rm_star(&NewStar {
                user_id: user.id,
                post_id: id,
            }).await)
        }
        (None, None, Some(u), None, None, None) => {
            let post = match conn.get_post_info_by_uuid(user.id, u).await {
                Ok(post) => post,
                Err(_) => return VoteResponse::C(Status::NotFound),
            };
            let id = post.id;
            (post, conn.add_flag(&NewFlag {
                user_id: user.id,
                post_id: id,
            }).await)
        }
        (None, None, None, Some(u), None, None) => {
            let post = match conn.get_post_info_by_uuid(user.id, u).await {
                Ok(post) => post,
                Err(_) => return VoteResponse::C(Status::NotFound),
            };
            let id = post.id;
            (post, conn.rm_flag(&NewFlag {
                user_id: user.id,
                post_id: id,
            }).await)
        }
        (None, None, None, None, Some(u), None) => {
            let post = match conn.get_post_info_by_uuid(user.id, u).await {
                Ok(post) => post,
                Err(_) => return VoteResponse::C(Status::NotFound),
            };
            let id = post.id;
            (post, conn.add_hide(&NewHide {
                user_id: user.id,
                post_id: id,
            }).await)
        }
        (None, None, None, None, None, Some(u)) => {
            let post = match conn.get_post_info_by_uuid(user.id, u).await {
                Ok(post) => post,
                Err(_) => return VoteResponse::C(Status::NotFound),
            };
            let id = post.id;
            (post, conn.rm_hide(&NewHide {
                user_id: user.id,
                post_id: id,
            }).await)
        }
        _ => return VoteResponse::C(Status::BadRequest),
    };
    let blog_post = post.blog_post;
    if result {
        if user.trust_level == 0 &&
            (Utc::now().naive_utc() - user.created_at) > Duration::seconds(60 * 60 * 24) &&
            conn.user_has_received_a_star(user.id).await {
            conn.change_user_trust_level(user.id, 1).await.expect("if voting works, then so should switching trust level")
        }
        if redirect.redirect.is_some() {
            redirect.maybe_redirect_vote(|| format!("{}", post.uuid))
        } else {
            let starred_by = conn.get_post_starred_by(post.id).await.unwrap_or(Vec::new());
            VoteResponse::B(content::Html(template::ViewStar {
                starred_by, customization, blog_post,
            }.render().unwrap()))
        }
    } else {
        VoteResponse::C(Status::BadRequest)
    }
}

#[derive(FromForm)]
struct VoteCommentForm {
    add_star_comment: Option<i32>,
    rm_star_comment: Option<i32>,
    add_flag_comment: Option<i32>,
    rm_flag_comment: Option<i32>,
    add_hide_comment: Option<i32>,
    rm_hide_comment: Option<i32>,
}

#[post("/vote-comment?<redirect..>", data = "<c>")]
async fn vote_comment(conn: MoreInterestingConn, login: LoginSession, redirect: MaybeRedirect, c: Form<VoteCommentForm>, customization: Customization) -> VoteResponse {
    let user = login.user;
    let (id, result) = match (c.add_star_comment, c.rm_star_comment, c.add_flag_comment, c.rm_flag_comment, c.add_hide_comment, c.rm_hide_comment) {
        (Some(i), None, None, None, None, None) => (i, conn.add_star_comment(&NewStarComment{
            user_id: user.id,
            comment_id: i,
        }).await),
        (None, Some(i), None, None, None, None) => (i, conn.rm_star_comment(&NewStarComment{
            user_id: user.id,
            comment_id: i,
        }).await),
        (None, None, Some(i), None, None, None) if user.trust_level >= 1 => (i, conn.add_flag_comment(&NewFlagComment{
            user_id: user.id,
            comment_id: i,
        }).await),
        (None, None, None, Some(i), None, None) => (i, conn.rm_flag_comment(&NewFlagComment{
            user_id: user.id,
            comment_id: i,
        }).await),
        (None, None, None, None, Some(i), None) if user.trust_level >= 1 => (i, conn.add_hide_comment(&NewHideComment{
            user_id: user.id,
            comment_id: i,
        }).await),
        (None, None, None, None, None, Some(i)) => (i, conn.rm_hide_comment(&NewHideComment{
            user_id: user.id,
            comment_id: i,
        }).await),
        _ => (0, false),
    };
    if result {
        if user.trust_level == 0 &&
            (Utc::now().naive_utc() - user.created_at) > Duration::seconds(60 * 60 * 24) &&
            conn.user_has_received_a_star(user.id).await {
            conn.change_user_trust_level(user.id, 1).await.expect("if voting works, then so should switching trust level")
        }
        if redirect.redirect.is_some() {
            redirect.maybe_redirect_vote(|| format!("{}", id))
        } else {
            let starred_by = conn.get_comment_starred_by(id).await.unwrap_or(Vec::new());
            VoteResponse::B(content::Html(template::ViewStar {
                starred_by, customization, blog_post: false,
            }.render().unwrap()))
        }
    } else {
        VoteResponse::C(Status::BadRequest)
    }
}

#[derive(FromForm)]
struct IndexParams {
    tag: Option<String>,
    domain: Option<String>,
    q: Option<String>,
    title: Option<String>,
    after: Option<Base32>,
    page: Option<i32>,
    subscriptions: Option<bool>,
    before_date: Option<String>,
    after_date: Option<String>,
    user: Option<String>,
}

async fn parse_index_params(conn: &MoreInterestingConn, user: &User, params: Option<IndexParams>) -> Option<(PostSearch, Vec<Tag>)> {
    let mut tags = vec![];
    let mut search = PostSearch::with_my_user_id(user.id);
    if let Some(after_uuid) = params.as_ref().and_then(|params| params.after.as_ref()) {
        let after = conn.get_post_info_by_uuid(user.id, *after_uuid).await.ok()?;
        search.after_post_id = after.id;
    }
    if let Some(page) = params.as_ref().and_then(|params| params.page) {
        search.search_page = page;
    }
    if let Some(tag_names) = params.as_ref().and_then(|params| params.tag.as_ref()) {
        if tag_names.contains("|") {
            for tag_name in tag_names.split("|") {
                if tag_name == "" {
                    continue;
                }
                if let Ok(tag) = conn.get_tag_by_name(tag_name.trim()).await {
                    search.or_tags.push(tag.id);
                    tags.push(tag);
                } else {
                    return None;
                }
            }
        } else {
            for tag_name in tag_names.split(" ") {
                if tag_name == "" {
                    continue;
                }
                if tag_name.starts_with("-") {
                    if let Ok(tag) = conn.get_tag_by_name(&tag_name[1..]).await {
                        search.hide_tags.push(tag.id);
                        continue;
                    } else {
                        return None;
                    }
                }
                if let Ok(tag) = conn.get_tag_by_name(tag_name).await {
                    search.and_tags.push(tag.id);
                    tags.push(tag);
                } else {
                    return None;
                }
            }
        }
    }
    if let Some(query) = params.as_ref().and_then(|params| params.q.as_ref()) {
        search.keywords = query.to_string();
    }
    if let Some(query) = params.as_ref().and_then(|params| params.title.as_ref()) {
        search.title = query.to_string();
        if search.keywords == "" {
            search.keywords = query.to_string();
        }
    }
    if let Some(domain_names) = params.as_ref().and_then(|params| params.domain.as_ref()) {
        for domain_name in domain_names.split("|").flat_map(|d| d.split(" ")).map(|d| d.trim()).filter(|&d| d != "") {
            if let Ok(domain) = conn.get_domain_by_hostname(domain_name).await {
                search.or_domains.push(domain.id);
            } else {
                return None;
            }
        }
    }
    if let Some(user) = params.as_ref().and_then(|params| params.user.as_ref()) {
        if let Ok(user) = conn.get_user_by_username(user).await {
            search.for_user_id = user.id;
        }
    }
    if let Some(before_date) = params.as_ref().and_then(|params| params.before_date.as_ref()).and_then(|d| d.parse::<NaiveDate>().ok()) {
        search.before_date = Some(before_date);
    }
    if let Some(after_date) = params.as_ref().and_then(|params| params.after_date.as_ref()).and_then(|d| d.parse::<NaiveDate>().ok()) {
        search.after_date = Some(after_date);
    }
    if params.and_then(|p| p.subscriptions).unwrap_or(false) {
        search.subscriptions = true;
    }
    Some((search, tags))
}

#[get("/?<params..>")]
async fn index(conn: MoreInterestingConn, login: Option<LoginSession>, flash: Option<FlashMessage<'_>>, params: Option<IndexParams>, config: &State<SiteConfig>, customization: Customization) -> Option<template::Index> {
    let (user, session) = login.map(|l| (l.user, l.session)).unwrap_or((User::default(), UserSession::default()));

    let mut tag_param = params.as_ref().and_then(|params| Some(params.tag.as_ref()?.to_string())).unwrap_or_else(String::new);
    let mut domain = params.as_ref().and_then(|params| Some(params.domain.as_ref()?.to_string())).unwrap_or_else(String::new);
    let (search, tags) = parse_index_params(&conn, &user, params).await?;
    let search = PostSearch {
        blog_post: Some(false),
        .. search
    };
    let blog_search = PostSearch {
        blog_post: Some(true),
        limit: 4,
        .. search.clone()
    };
    let keywords_param = search.keywords.clone();
    let title_param = search.title.clone();
    let is_home = tag_param == "" && domain == "" && keywords_param == "" && title_param == "";
    let before_date_param = search.before_date;
    let after_date_param = search.after_date;
    let noindex = keywords_param != "" && (search.after_post_id != 0 || search.search_page != 0);
    let title = customization.title.clone();

    let notifications = conn.list_notifications(user.id);
    let posts = conn.search_posts(&search);
    let (posts, extra_blog_posts, notifications) = if is_home {
        let (posts, extra_blog_posts, notifications) = futures::join!(
            posts,
            conn.search_posts(&blog_search),
            notifications);
        (posts.ok()?, extra_blog_posts.ok()?, notifications.unwrap_or(Vec::new()))
    } else {
        let (posts, notifications) = futures::join!(posts, notifications);
        (posts.ok()?, Vec::new(), notifications.unwrap_or(Vec::new()))
    };

    Some(template::Index {
        alert: flash.map(|f| f.message().to_owned()).unwrap_or_else(String::new),
        config: config.inner().clone(),
        next_search_page: search.search_page + 1,
        customization, before_date_param, after_date_param,
        title, user, posts, is_home,
        tags, session, tag_param, domain, keywords_param,
        title_param, extra_blog_posts,
        notifications, noindex,
    })
}

#[get("/blog?<params..>")]
async fn blog_index(conn: MoreInterestingConn, login: Option<LoginSession>, flash: Option<FlashMessage<'_>>, params: Option<IndexParams>, config: &State<SiteConfig>, customization: Customization) -> Option<template::Blog> {
    let (user, session) = login.map(|l| (l.user, l.session)).unwrap_or((User::default(), UserSession::default()));

    let (search, tags) = parse_index_params(&conn, &user, params).await?;
    let search = PostSearch {
        blog_post: Some(true),
        order_by: PostSearchOrderBy::Newest,
        .. search
    };
    let keywords_param = search.keywords.clone();
    let title_param = search.title.clone();
    let before_date_param = search.before_date;
    let after_date_param = search.after_date;
    let posts = conn.search_posts(&search).await.ok()?;
    let notifications = conn.list_notifications(user.id).await.unwrap_or(Vec::new());
    let noindex = keywords_param != "" && (search.after_post_id != 0 || search.search_page != 0);
    let title = customization.title.clone();
    Some(template::Blog {
        alert: flash.map(|f| f.message().to_owned()).unwrap_or_else(String::new),
        config: config.inner().clone(),
        next_search_page: search.search_page + 1,
        customization, before_date_param, after_date_param,
        title, user, posts,
        session, keywords_param,
        title_param,
        notifications, noindex,
    })
}

#[get("/search?<params..>")]
async fn advanced_search(conn: MoreInterestingConn, login: Option<LoginSession>, flash: Option<FlashMessage<'_>>, params: Option<IndexParams>, config: &State<SiteConfig>, customization: Customization) -> Option<template::Search> {
    let (user, session) = login.map(|l| (l.user, l.session)).unwrap_or((User::default(), UserSession::default()));

    let tag_param = params.as_ref().and_then(|params| Some(params.tag.as_ref()?.to_string())).unwrap_or_else(String::new);
    let domain = params.as_ref().and_then(|params| Some(params.domain.as_ref()?.to_string())).unwrap_or_else(String::new);
    let (search, tags) = parse_index_params(&conn, &user, params).await?;
    let keywords_param = search.keywords.clone();
    let title_param = search.title.clone();
    let before_date_param = search.before_date;
    let after_date_param = search.after_date;
    let notifications = conn.list_notifications(user.id).await.unwrap_or(Vec::new());
    Some(template::Search {
        title: "Advanced Search",
        alert: flash.map(|f| f.message().to_owned()).unwrap_or_else(String::new),
        config: config.inner().clone(),
        next_search_page: search.search_page + 1,
        noindex: false,
        customization, before_date_param, after_date_param,
        user, tags, session, tag_param, domain, keywords_param,
        title_param,
        notifications,
    })
}

#[derive(FromForm)]
struct SearchCommentsParams {
    user: Option<String>,
    after: Option<i32>,
}

#[get("/comments?<params..>")]
async fn search_comments(conn: MoreInterestingConn, login: Option<LoginSession>, flash: Option<FlashMessage<'_>>, params: Option<SearchCommentsParams>, config: &State<SiteConfig>, customization: Customization) -> Option<template::ProfileComments> {
    let (user, session) = login.map(|l| (l.user, l.session)).unwrap_or((User::default(), UserSession::default()));
    if let Some(username) = params.as_ref().and_then(|params| params.user.as_ref()) {
        let by_user = conn.get_user_by_username(&username[..]).await.into_option()?;
        let comment_search_result = if let Some(after_id) = params.as_ref().and_then(|params| params.after.as_ref()) {
            conn.search_comments_by_user_after(by_user.id, *after_id).await.into_option()?
        } else {
            conn.search_comments_by_user(by_user.id).await.into_option()?
        };
        let title = by_user.username.clone();
        let notifications = conn.list_notifications(user.id).await.unwrap_or(Vec::new());
        Some(template::ProfileComments {
            alert: flash.map(|f| f.message().to_owned()).unwrap_or_else(String::new),
            config: config.inner().clone(),
            customization,
            is_me: by_user.id == user.id,
            title, user, comment_search_result, session,
            noindex: true,
            notifications,
        })
    } else {
        None
    }
}

#[get("/top?<params..>")]
async fn top(conn: MoreInterestingConn, login: Option<LoginSession>, flash: Option<FlashMessage<'_>>, config: &State<SiteConfig>, params: Option<IndexParams>, customization: Customization) -> Option<template::IndexTop> {
    let (user, session) = login.map(|l| (l.user, l.session)).unwrap_or((User::default(), UserSession::default()));
    let tag_param = params.as_ref().and_then(|params| Some(params.tag.as_ref()?.to_string())).unwrap_or_else(String::new);
    let domain = params.as_ref().and_then(|params| Some(params.domain.as_ref()?.to_string())).unwrap_or_else(String::new);
    let (search, tags) = parse_index_params(&conn, &user, params).await?;
    let before_date_param = search.before_date;
    let after_date_param = search.after_date;
    let search = PostSearch {
        order_by: PostSearchOrderBy::Top,
        blog_post: Some(false),
        .. search
    };
    let keywords_param = search.keywords.clone();
    let title_param = search.title.clone();
    let is_home = tag_param == "" && domain == "" && keywords_param == "";
    let posts = conn.search_posts(&search).await.ok()?;
    let notifications = conn.list_notifications(user.id).await.unwrap_or(Vec::new());
    let noindex = keywords_param != "" && (search.after_post_id != 0 || search.search_page != 0);
    Some(template::IndexTop {
        title: String::from("top"),
        next_search_page: search.search_page + 1,
        alert: flash.map(|f| f.message().to_owned()).unwrap_or_else(String::new),
        config: config.inner().clone(),
        customization, before_date_param, after_date_param,
        is_home, keywords_param, title_param,
        user, posts, session, tags, tag_param, domain,
        notifications, noindex
    })
}

#[get("/subscriptions?<params..>")]
async fn subscriptions(conn: MoreInterestingConn, login: Option<LoginSession>, flash: Option<FlashMessage<'_>>, config: &State<SiteConfig>, params: Option<IndexParams>, customization: Customization) -> Option<template::Subscriptions> {
    let (user, session) = login.map(|l| (l.user, l.session)).unwrap_or((User::default(), UserSession::default()));
    let tag_param = params.as_ref().and_then(|params| Some(params.tag.as_ref()?.to_string())).unwrap_or_else(String::new);
    let domain = params.as_ref().and_then(|params| Some(params.domain.as_ref()?.to_string())).unwrap_or_else(String::new);
    let (search, tags) = parse_index_params(&conn, &user, params).await?;
    let before_date_param = search.before_date;
    let after_date_param = search.after_date;
    let search = PostSearch {
        order_by: PostSearchOrderBy::Latest,
        subscriptions: true,
        blog_post: None,
        .. search
    };
    let keywords_param = search.keywords.clone();
    let title_param = search.title.clone();
    let is_home = tag_param == "" && domain == "" && keywords_param == "";
    let posts = conn.search_posts(&search).await.ok()?;
    let notifications = conn.list_notifications(user.id).await.unwrap_or(Vec::new());
    Some(template::Subscriptions {
        title: String::from("top"),
        alert: flash.map(|f| f.message().to_owned()).unwrap_or_else(String::new),
        config: config.inner().clone(),
        noindex: true,
        customization, before_date_param, after_date_param,
        is_home, keywords_param, title_param,
        user, posts, session, tags, tag_param, domain,
        notifications
    })
}

#[derive(FromForm)]
struct SubscriptionForm {
    post: Base32,
    subscribed: bool,
}

#[post("/subscriptions?<redirect..>", data = "<form>")]
async fn post_subscriptions(conn: MoreInterestingConn, login: LoginSession, form: Form<SubscriptionForm>, redirect: MaybeRedirect) -> Result<Redirect, Status> {
    let user = login.user;
    let post = conn.get_post_info_by_uuid(user.id, form.post).await.map_err(|_| Status::NotFound)?;
    if form.subscribed {
        let _ = conn.create_subscription(NewSubscription {
            user_id: user.id,
            created_by: user.id,
            post_id: post.id,
        }).await;
    } else {
        let _ = conn.drop_subscription(NewSubscription {
            user_id: user.id,
            created_by: user.id,
            post_id: post.id,
        }).await;
    }
    if conn.is_subscribed(post.id, user.id).await.unwrap_or(false) != form.subscribed {
        warn!("(un)subscription failed!");
        return Err(Status::InternalServerError);
    }
    redirect.maybe_redirect()
}

#[get("/new?<params..>")]
async fn new(conn: MoreInterestingConn, login: Option<LoginSession>, flash: Option<FlashMessage<'_>>, config: &State<SiteConfig>, params: Option<IndexParams>, customization: Customization) -> Option<template::IndexNew> {
    let (user, session) = login.map(|l| (l.user, l.session)).unwrap_or((User::default(), UserSession::default()));
    let tag_param = params.as_ref().and_then(|params| Some(params.tag.as_ref()?.to_string())).unwrap_or_else(String::new);
    let domain = params.as_ref().and_then(|params| Some(params.domain.as_ref()?.to_string())).unwrap_or_else(String::new);
    let (search, tags) = parse_index_params(&conn, &user, params).await?;
    let before_date_param = search.before_date;
    let after_date_param = search.after_date;
    let search = PostSearch {
        order_by: PostSearchOrderBy::Newest,
        blog_post: Some(false),
        .. search
    };
    let keywords_param = search.keywords.clone();
    let title_param = search.title.clone();
    let is_home = tag_param == "" && domain == "" && keywords_param == "";
    let posts = conn.search_posts(&search).await.ok()?;
    let notifications = conn.list_notifications(user.id).await.unwrap_or(Vec::new());
    let noindex = (keywords_param != "" || tags.len() > 0 || domain != "") && (search.after_post_id != 0 || search.search_page != 0);
    Some(template::IndexNew {
        title: String::from("new"),
        next_search_page: search.search_page + 1,
        alert: flash.map(|f| f.message().to_owned()).unwrap_or_else(String::new),
        config: config.inner().clone(),
        customization, before_date_param, after_date_param,
        is_home, keywords_param, title_param,
        user, posts, session, tags, tag_param, domain,
        notifications, noindex,
    })
}

#[get("/latest?<params..>")]
async fn latest(conn: MoreInterestingConn, login: Option<LoginSession>, params: Option<IndexParams>, flash: Option<FlashMessage<'_>>, config: &State<SiteConfig>, customization: Customization) -> Option<template::IndexLatest> {
    let (user, session) = login.map(|l| (l.user, l.session)).unwrap_or((User::default(), UserSession::default()));
    let tag_param = params.as_ref().and_then(|params| Some(params.tag.as_ref()?.to_string())).unwrap_or_else(String::new);
    let domain = params.as_ref().and_then(|params| Some(params.domain.as_ref()?.to_string())).unwrap_or_else(String::new);
    let (search, tags) = parse_index_params(&conn, &user, params).await?;
    let before_date_param = search.before_date;
    let after_date_param = search.after_date;
    let search = PostSearch {
        order_by: PostSearchOrderBy::Latest,
        blog_post: Some(false),
        .. search
    };
    let keywords_param = search.keywords.clone();
    let title_param = search.title.clone();
    let is_home = tag_param == "" && domain == "" && keywords_param == "";
    let posts = conn.search_posts(&search).await.ok()?;
    let notifications = conn.list_notifications(user.id).await.unwrap_or(Vec::new());
    let noindex = keywords_param != "" && (search.after_post_id != 0 || search.search_page != 0);
    Some(template::IndexLatest {
        title: String::from("latest"),
        next_search_page: search.search_page + 1,
        alert: flash.map(|f| f.message().to_owned()).unwrap_or_else(String::new),
        config: config.inner().clone(),
        customization, before_date_param, after_date_param,
        is_home, keywords_param, title_param,
        user, posts, session, tags, tag_param, domain,
        notifications, noindex,
    })
}

#[get("/rss")]
async fn rss(conn: MoreInterestingConn, config: &State<SiteConfig>, customization: Customization) -> Option<content::Custom<String>> {
    let search = PostSearch {
        order_by: PostSearchOrderBy::Newest,
        blog_post: Some(false),
        .. PostSearch::with_my_user_id(0)
    };
    let posts = conn.search_posts(&search).await.ok()?;
    Some(content::Custom(ContentType::from_str("application/rss+xml").unwrap(), template::Rss {
        posts,
        config: config.inner().clone(),
        link: config.public_url.to_string(),
        customization,
    }.render().unwrap()))
}

#[get("/blog.rss")]
async fn blog_rss(conn: MoreInterestingConn, config: &State<SiteConfig>, customization: Customization) -> Option<content::Custom<String>> {
    let search = PostSearch {
        order_by: PostSearchOrderBy::Newest,
        blog_post: Some(true),
        .. PostSearch::with_my_user_id(0)
    };
    let posts = conn.search_posts(&search).await.ok()?;
    Some(content::Custom(ContentType::from_str("application/rss+xml").unwrap(), template::BlogRss {
        posts,
        config: config.inner().clone(),
        link: config.public_url.to_string(),
        customization,
    }.render().unwrap()))
}

#[derive(FromForm)]
struct ModLogParams {
    after: Option<i32>,
}

#[get("/mod-log?<params..>")]
async fn mod_log(conn: MoreInterestingConn, login: Option<LoginSession>, flash: Option<FlashMessage<'_>>, params: Option<ModLogParams>, config: &State<SiteConfig>, customization: Customization) -> template::ModLog {
    let (user, session) = login.map(|l| (l.user, l.session)).unwrap_or((User::default(), UserSession::default()));
    let log = if let Some(after) = params.as_ref().and_then(|params| params.after) {
        conn.get_mod_log_starting_with(after).await
    } else {
        conn.get_mod_log_recent().await
    }.unwrap_or(Vec::new());
    let notifications = conn.list_notifications(user.id).await.unwrap_or(Vec::new());
    template::ModLog {
        title: String::from("mod log"),
        alert: flash.map(|f| f.message().to_owned()).unwrap_or_else(String::new),
        config: config.inner().clone(),
        noindex: true,
        customization,
        user, log, session,
        notifications,
    }
}

#[get("/post")]
async fn create_post_form(conn: MoreInterestingConn, login: Option<LoginSession>, config: &State<SiteConfig>, customization: Customization) -> template::CreatePost {
    let (user, session) = login.map(|l| (l.user, l.session)).unwrap_or((User::default(), UserSession::default()));
    let notifications = conn.list_notifications(user.id).await.unwrap_or(Vec::new());

    let submitted_by_username_urlencode = utf8_percent_encode(&user.username, NON_ALPHANUMERIC).to_string();
    let submitted_by_username = user.username.clone();

    template::CreatePost {
        post: PostInfo {
            title: String::new(),
            url: None,
            title_html: String::new(),
            excerpt_html: None,
            uuid: Base32::from(0i64),
            id: 0,
            visible: true,
            private: false,
            noindex: false,
            locked: false,
            hotness: 0.0,
            score: 0,
            comment_count: 0,
            comment_readpoint: None,
            blog_post: false,
            created_at: Utc::now().naive_utc(),
            created_at_relative: relative_date(&Utc::now().naive_utc()),
            submitted_by: user.id,
            submitted_by_username,
            submitted_by_username_urlencode,
            starred_by_me: false,
            flagged_by_me: false,
            hidden_by_me: false,
            banner_title: None,
            banner_desc: None,
        },
        title: String::from("post"),
        alert: String::new(),
        config: config.inner().clone(),
        excerpt: None,
        noindex: false,
        customization,
        user, session,
        notifications,
    }
}

#[post("/preview-post", data = "<post>")]
async fn post_preview(login: Option<LoginSession>, customization: Customization, conn: MoreInterestingConn, post: Form<NewPostForm>, config: &State<SiteConfig>) -> Result<Either<template::PreviewPost, template::CreatePost>, Status> {
    lazy_static!{
        static ref TAGS_SPLIT: Regex = Regex::new(r"[#, \t]+").unwrap();
    }
    let (user, session) = login.map(|l| (Some(l.user), l.session)).unwrap_or((None, UserSession::default()));
    let user = if let Some(user) = user {
        if user.banned {
            return Err(Status::InternalServerError);
        }
        user
    } else if config.enable_anonymous_submissions {
        if let Ok(user) = conn.get_user_by_username("anonymous").await {
            user
        } else {
            let p: [char; 16] = rand::random();
            let mut password = String::new();
            password.extend(p.iter());
            let user = conn.register_user(NewUser{
                username: "anonymous".to_owned(),
                password,
                invited_by: None,
            }).await.map_err(|_| Status::InternalServerError)?;
            conn.change_user_trust_level(user.id, -1).await.map_err(|_| Status::InternalServerError)?;
            conn.change_user_banned(user.id, true).await.map_err(|_| Status::InternalServerError)?;
            user
        }
    } else {
        return Err(Status::BadRequest);
    };
    if user.trust_level == 1 &&
        (Utc::now().naive_utc() - user.created_at) > Duration::seconds(60 * 60 * 24 * 7) {
        conn.change_user_trust_level(user.id, 2).await.expect("if voting works, then so should switching trust level")
    }
    let NewPostForm { title, url, excerpt, tags, no_preview, blog_post } = &*post;
    let mut title = title.clone();
    for tag in TAGS_SPLIT.split(tags.as_ref().map(|x| &x[..]).unwrap_or("")) {
        if tag == "" { continue }
        if conn.get_tag_by_name(tag).await.is_err() {
            continue;
        }
        title += " #";
        title += tag;
    }
    let title = &title[..];
    let url = url.as_ref().and_then(|u| {
        if u == "" {
            None
        } else if !u.contains(":") && !u.starts_with("//") {
            Some(format!("https://{}", u))
        } else {
            Some(u.to_owned())
        }
    });
    let excerpt = excerpt.as_ref().and_then(|k| if k == "" { None } else { Some(&k[..]) });
    let body_html = conn.prettify_body(0, excerpt.unwrap_or(""), config.body_format).await;
    let title_html = conn.prettify_title(0, &title, &url.as_ref().unwrap_or(&String::new())[..], true).await;
    let submitted_by_username_urlencode = utf8_percent_encode(&user.username, NON_ALPHANUMERIC).to_string();
    let submitted_by_username = user.username.clone();
    let mut post_info = PostInfo {
        title: title.to_owned(),
        url,
        title_html: title_html,
        excerpt_html: Some(body_html),
        uuid: Base32::from(0i64),
        id: 0,
        visible: true,
        private: false,
        noindex: false,
        locked: false,
        hotness: 0.0,
        score: 0,
        comment_count: 0,
        comment_readpoint: None,
        blog_post: *blog_post,
        created_at: Utc::now().naive_utc(),
        created_at_relative: relative_date(&Utc::now().naive_utc()),
        submitted_by: user.id,
        submitted_by_username,
        submitted_by_username_urlencode,
        starred_by_me: false,
        flagged_by_me: false,
        hidden_by_me: false,
        banner_title: Some("Preview".to_string()),
        banner_desc: None,
    };
    let notifications = conn.list_notifications(user.id).await.unwrap_or(Vec::new());
    if no_preview.is_some() {
        let mut title = String::new();
        let mut tags = String::new();
        let mut first = true;
        for part in post_info.title.split('#') {
            if first {
                title = part.to_owned();
                first = false;
            } else {
                tags = tags + &part;
            }
        }
        post_info.title = title;
        post_info.title_html = tags;
        return Ok(Either::B(template::CreatePost {
            alert: String::new(),
            post: post_info,
            title: String::from("post"),
            config: config.inner().clone(),
            excerpt: excerpt.map(ToOwned::to_owned),
            noindex: true,
            customization, notifications,
            user, session,
        }))
    }
    Ok(Either::A(template::PreviewPost {
        alert: String::new(),
        post: post_info,
        config: config.inner().clone(),
        title: String::from("post"),
        excerpt: excerpt.map(ToOwned::to_owned),
        noindex: true,
        customization, notifications,
        user, session,
    }))
}

#[get("/submit")]
async fn create_link_form(login: Option<LoginSession>, config: &State<SiteConfig>, conn: MoreInterestingConn, customization: Customization, flash: Option<FlashMessage<'_>>) -> template::Submit {
    let (user, session) = login.map(|l| (l.user, l.session)).unwrap_or((User::default(), UserSession::default()));
    let submitted_by_username_urlencode = utf8_percent_encode(&user.username, NON_ALPHANUMERIC).to_string();
    let submitted_by_username = user.username.clone();
    template::Submit {
        title: String::from("submit"),
        config: config.inner().clone(),
        alert: flash.map(|f| f.message().to_owned()).unwrap_or_else(String::new),
        excerpt: None,
        notifications: conn.list_notifications(user.id).await.unwrap_or(Vec::new()),
        noindex: false,
        post: PostInfo {
            title: String::new(),
            url: None,
            title_html: String::new(),
            excerpt_html: None,
            uuid: Base32::from(0i64),
            id: 0,
            visible: true,
            private: false,
            noindex: false,
            locked: false,
            hotness: 0.0,
            score: 0,
            comment_count: 0,
            comment_readpoint: None,
            blog_post: false,
            created_at: Utc::now().naive_utc(),
            created_at_relative: relative_date(&Utc::now().naive_utc()),
            submitted_by: user.id,
            submitted_by_username,
            submitted_by_username_urlencode,
            starred_by_me: false,
            flagged_by_me: false,
            hidden_by_me: false,
            banner_title: None,
            banner_desc: None,
        },
        customization,
        user, session,
    }
}

#[derive(FromForm)]
struct NewPostForm {
    title: String,
    tags: Option<String>,
    url: Option<String>,
    excerpt: Option<String>,
    no_preview: Option<bool>,
    blog_post: bool,
}

#[post("/preview-submit", data = "<post>")]
async fn submit_preview(login: Option<LoginSession>, customization: Customization, conn: MoreInterestingConn, post: Form<NewPostForm>, config: &State<SiteConfig>) -> Result<Either<template::PreviewSubmit, template::Submit>, Status> {
    lazy_static!{
        static ref TAGS_SPLIT: Regex = Regex::new(r"[#, \t]+").unwrap();
    }
    let (user, session) = login.map(|l| (Some(l.user), l.session)).unwrap_or((None, UserSession::default()));
    let user = if let Some(user) = user {
        if user.banned {
            return Err(Status::InternalServerError);
        }
        user
    } else if config.enable_anonymous_submissions {
        if let Ok(user) = conn.get_user_by_username("anonymous").await {
            user
        } else {
            let p: [char; 16] = rand::random();
            let mut password = String::new();
            password.extend(p.iter());
            let user = conn.register_user(NewUser{
                username: "anonymous".to_owned(),
                password,
                invited_by: None,
            }).await.map_err(|_| Status::InternalServerError)?;
            conn.change_user_trust_level(user.id, -1).await.map_err(|_| Status::InternalServerError)?;
            conn.change_user_banned(user.id, true).await.map_err(|_| Status::InternalServerError)?;
            user
        }
    } else {
        return Err(Status::BadRequest);
    };
    if user.trust_level == 1 &&
        (Utc::now().naive_utc() - user.created_at) > Duration::seconds(60 * 60 * 24 * 7) {
        conn.change_user_trust_level(user.id, 2).await.expect("if voting works, then so should switching trust level")
    }
    let NewPostForm { title, url, excerpt, tags, no_preview, blog_post } = &*post;
    let mut title = title.clone();
    for tag in TAGS_SPLIT.split(tags.as_ref().map(|x| &x[..]).unwrap_or("")) {
        if tag == "" { continue }
        if conn.get_tag_by_name(tag).await.is_err() {
            continue;
        }
        title += " #";
        title += tag;
    }
    let title = &title[..];
    let url = url.as_ref().and_then(|u| {
        if u == "" {
            None
        } else if !u.contains(":") && !u.starts_with("//") {
            Some(format!("https://{}", u))
        } else {
            Some(u.to_owned())
        }
    });
    let excerpt = excerpt.as_ref().and_then(|k| if k == "" { None } else { Some(&k[..]) });
    let body_html = conn.prettify_body(0, excerpt.unwrap_or(""), config.body_format).await;
    let title_html = conn.prettify_title(0, &title, &url.as_ref().unwrap_or(&String::new())[..], false).await;
    let submitted_by_username_urlencode = utf8_percent_encode(&user.username, NON_ALPHANUMERIC).to_string();
    let submitted_by_username = user.username.clone();
    let mut post_info = PostInfo {
        title: title.to_owned(),
        url,
        title_html: title_html,
        excerpt_html: Some(body_html),
        uuid: Base32::from(0i64),
        id: 0,
        visible: true,
        private: false,
        noindex: false,
        locked: false,
        hotness: 0.0,
        score: 0,
        comment_count: 0,
        comment_readpoint: None,
        blog_post: *blog_post,
        created_at: Utc::now().naive_utc(),
        created_at_relative: relative_date(&Utc::now().naive_utc()),
        submitted_by: user.id,
        submitted_by_username,
        submitted_by_username_urlencode,
        starred_by_me: false,
        flagged_by_me: false,
        hidden_by_me: false,
        banner_title: Some("Preview".to_string()),
        banner_desc: None,
    };
    if no_preview.is_some() {
        let mut title = String::new();
        let mut tags = String::new();
        let mut first = true;
        for part in post_info.title.split('#') {
            if first {
                title = part.to_owned();
                first = false;
            } else {
                tags = tags + &part;
            }
        }
        post_info.title = title;
        post_info.title_html = tags;
        return Ok(Either::B(template::Submit {
            alert: String::new(),
            post: post_info,
            title: String::from("submit"),
            config: config.inner().clone(),
            excerpt: excerpt.map(ToOwned::to_owned),
            notifications: conn.list_notifications(user.id).await.unwrap_or(Vec::new()),
            noindex: true,
            customization,
            user, session,
        }))
    }
    Ok(Either::A(template::PreviewSubmit {
        alert: String::new(),
        post: post_info,
        config: config.inner().clone(),
        title: String::from("submit"),
        excerpt: excerpt.map(ToOwned::to_owned),
        notifications: conn.list_notifications(user.id).await.unwrap_or(Vec::new()),
        noindex: true,
        customization,
        user, session,
    }))
}

#[post("/submit", data = "<post>")]
async fn create(login: Option<LoginSession>, conn: MoreInterestingConn, post: Form<NewPostForm>, config: &State<SiteConfig>) -> Result<Flash<Redirect>, Status> {
    lazy_static!{
        static ref TAGS_SPLIT: Regex = Regex::new(r"[#, \t]+").unwrap();
    }
    let user = login.as_ref().map(|l| l.user.clone());
    let user = if let Some(user) = user {
        if user.banned {
            return Err(Status::InternalServerError);
        }
        user
    } else if config.enable_anonymous_submissions {
        if let Ok(user) = conn.get_user_by_username("anonymous").await {
            user
        } else {
            let p: [char; 16] = rand::random();
            let mut password = String::new();
            password.extend(p.iter());
            let user = conn.register_user(NewUser{
                username: "anonymous".to_owned(),
                password,
                invited_by: None,
            }).await.map_err(|_| Status::InternalServerError)?;
            conn.change_user_trust_level(user.id, -1).await.map_err(|_| Status::InternalServerError)?;
            conn.change_user_banned(user.id, true).await.map_err(|_| Status::InternalServerError)?;
            user
        }
    } else {
        return Err(Status::BadRequest);
    };
    if user.trust_level == 1 &&
        (Utc::now().naive_utc() - user.created_at) > Duration::seconds(60 * 60 * 24 * 7) {
        conn.change_user_trust_level(user.id, 2).await.expect("if voting works, then so should switching trust level")
    }
    let NewPostForm { title, url, excerpt, tags, blog_post, .. } = &*post;
    let mut title = title.clone();
    for tag in TAGS_SPLIT.split(tags.as_ref().map(|x| &x[..]).unwrap_or("")) {
        if tag == "" { continue }
        if conn.get_tag_by_name(tag).await.is_err() {
            return Ok(Flash::error(Redirect::to("submit".to_string()), "The specified tag does not exist"))
        }
        title += " #";
        title += tag;
    }
    let url = url.as_ref().and_then(|u| {
        if u == "" {
            None
        } else if !u.contains(":") && !u.starts_with("//") {
            Some(format!("https://{}", u))
        } else {
            Some(u.to_owned())
        }
    });
    let visible = if url.is_some() {
        user.trust_level > 0i32
    } else if user.trust_level <= 0i32 {
        return Err(Status::BadRequest);
    } else {
        false
    };
    let excerpt = excerpt.as_ref().and_then(|k| if k == "" { None } else { Some(k.clone()) });
    match conn.create_post(NewPost {
        submitted_by: user.id,
        visible,
        private: false,
        blog_post: *blog_post,
        title, excerpt, url,
    }, config.body_format, user.username != "anonymous").await {
        Ok(post) => Ok(Flash::success(Redirect::to(post.uuid.to_string()), "Post created")),
        Err(CreatePostError::TooLong) => {
            Ok(Flash::error(Redirect::to("submit".to_string()), "Too long; please find a shorter excerpt"))
        }
        Err(CreatePostError::TooManyPosts) => {
            Ok(Flash::error(Redirect::to("submit".to_string()), "You have exceeded the post limit for today; try again tomorrow"))
        }
        Err(CreatePostError::TooManyPostsDomain) => {
            Ok(Flash::error(Redirect::to("submit".to_string()), "This domain name has exceeded the post limit for today; try submitting quotes from somewhere else"))
        }
        Err(CreatePostError::TooManyPostsDomainUser) => {
            Ok(Flash::error(Redirect::to("submit".to_string()), "You have submitted too many posts from this particular domain; try submitting quotes from somewhere else"))
        }
        Err(CreatePostError::RequireTag) => {
            Ok(Flash::error(Redirect::to("submit".to_string()), "Please specify at least one tag"))
        }
        Err(e) => {
            warn!("{:?}", e);
            Err(Status::InternalServerError)
        },
    }
}

#[get("/message")]
async fn create_message_form(login: LoginSession, conn: MoreInterestingConn, config: &State<SiteConfig>, customization: Customization) -> Option<template::Message> {
    let (user, session) = (login.user, login.session);
    if user.trust_level < 2 {
        return None;
    }
    Some(template::Message {
        alert: String::new(),
        title: String::from("message"),
        config: config.inner().clone(),
        notifications: conn.list_notifications(user.id).await.unwrap_or(Vec::new()),
        noindex: true,
        customization,
        user, session,
    })
}

#[derive(FromForm)]
struct NewMessageForm {
    title: String,
    users: String,
    excerpt: String,
}

#[post("/message", data = "<post>")]
async fn create_message(login: LoginSession, conn: MoreInterestingConn, post: Form<NewMessageForm>, config: &State<SiteConfig>) -> Result<Redirect, Status> {
    let user = login.user;
    if user.trust_level < 2 {
        return Err(Status::NotFound);
    }
    if user.banned {
        return Err(Status::InternalServerError);
    }
    let title = post.title.clone();
    let excerpt = Some(post.excerpt.clone());
    let users = &post.users;
    match conn.create_post(NewPost {
        url: None,
        submitted_by: user.id,
        visible: true,
        private: true,
        blog_post: false,
        title, excerpt
    }, config.body_format, false).await {
        Ok(post) => {
            conn.create_subscription(NewSubscription {
                user_id: user.id,
                created_by: user.id,
                post_id: post.id,
            }).await.unwrap_or_else(|e| warn!("Cannot subscribe self {}: {:?}", user.username, e));
            for notify in users.split(",") {
                for notify in notify.split(" ") {
                    if notify == "" { continue };
                    if let Ok(notify) = conn.get_user_by_username(notify).await {
                        conn.create_notification(NewNotification {
                            user_id: notify.id,
                            created_by: user.id,
                            post_id: post.id,
                        }).await.unwrap_or_else(|e| warn!("Cannot notify {}: {:?}", notify.username, e));
                        conn.create_subscription(NewSubscription {
                            user_id: notify.id,
                            created_by: user.id,
                            post_id: post.id,
                        }).await.unwrap_or_else(|e| warn!("Cannot subscribe {}: {:?}", notify.username, e));
                    }
                }
            }
            Ok(Redirect::to(post.uuid.to_string()))
        },
        Err(e) => {
            warn!("{:?}", e);
            Err(Status::InternalServerError)
        },
    }
}

#[get("/login")]
async fn login_form(config: &State<SiteConfig>, flash: Option<FlashMessage<'_>>, customization: Customization) -> template::Login {
    template::Login {
        title: String::from("log in"),
        config: config.inner().clone(),
        customization,
        alert: flash.map(|f| f.message().to_owned()).unwrap_or_else(String::new),
        user: User::default(),
        session: UserSession::default(),
        notifications: Vec::new(),
        noindex: false,
    }
}

#[derive(FromForm)]
struct UserForm {
    username: String,
    password: String,
}

#[post("/login", data = "<post>")]
async fn login(conn: MoreInterestingConn, post: Form<UserForm>, cookies: &CookieJar<'_>, user_agent: UserAgentString<'_>) -> Flash<Redirect> {
    match conn.authenticate_user(&UserAuth {
        username: post.username.clone(),
        password: post.password.clone(),
    }).await {
        Some(user) => {
            if user.trust_level == -2 || user.banned {
                let cookie = Cookie::build("B", "1").path("/").permanent().same_site(SameSite::None).finish();
                cookies.add(cookie);
            }
            if user.banned {
                return Flash::error(Redirect::to("."), "Sorry. Not sorry. You're banned.");
            }
            let session = conn.create_session(user.id, user_agent.user_agent).await.expect("failed to allocate a session");
            let cookie = Cookie::build("U", session.uuid.to_string()).path("/").permanent().same_site(SameSite::None).finish();
            cookies.add(cookie);
            let cookie = Cookie::build("N", user.username.to_string()).path("/").permanent().same_site(SameSite::None).finish();
            cookies.add(cookie);
            Flash::success(Redirect::to("."), "Congrats, you're in!")
        },
        None => {
            Flash::error(Redirect::to("login"), "Incorrect username or password")
        },
    }
}

#[post("/logout")]
async fn logout(cookies: &CookieJar<'_>) -> Redirect {
    let cookie = Cookie::build("U", "").path("/").permanent().same_site(SameSite::None).finish();
    cookies.add(cookie);
    Redirect::to(".")
}

#[get("/<uuid>", rank = 1)]
async fn get_comments(conn: MoreInterestingConn, login: Option<LoginSession>, uuid: String, config: &State<SiteConfig>, flash: Option<FlashMessage<'_>>, customization: Customization) -> Result<OneOf<template::ProfilePosts, template::Similar, template::Comments, template::Signup>, Status> {
    let (user, session) = login.map(|l| (l.user, l.session)).unwrap_or((User::default(), UserSession::default()));
    if uuid.len() > 0 && uuid.as_bytes()[0] == b'@' {
        let username = &uuid[1..];
        let user_info = if let Ok(user_info) = conn.get_user_by_username(username).await {
            user_info
        } else {
            return Err(Status::NotFound);
        };
        let search = PostSearch {
            for_user_id: user_info.id,
            order_by: PostSearchOrderBy::Newest,
            blog_post: None,
            .. PostSearch::with_my_user_id(user.id)
        };
        let posts = if let Ok(posts) = conn.search_posts(&search).await {
            posts
        } else {
            return Err(Status::InternalServerError);
        };

        let notifications = conn.list_notifications(user.id).await.unwrap_or(Vec::new());

        return Ok(OneOf::A(template::ProfilePosts {
            title: username.to_owned(),
            alert: flash.map(|f| f.message().to_owned()).unwrap_or_else(String::new),
            config: config.inner().clone(),
            customization,
            is_me: user_info.id == user.id,
            noindex: true,
            posts, user, session,
            notifications,
        }));
    }
    if uuid.len() > 0 && uuid.as_bytes()[0] == b'+' && user.id != 0 {
        let uuid = &uuid[1..];
        let uuid = if let Ok(uuid) = Base32::from_str(uuid) {
            uuid
        } else {
            return Err(Status::NotFound);
        };
        let post_info = if let Ok(post_info) = conn.get_post_by_uuid(uuid).await {
            post_info
        } else {
            return Err(Status::NotFound);
        };
        let title = post_info.title.clone();
        let posts = if let Ok(posts) = conn.get_post_info_similar(user.id, post_info).await {
            posts
        } else {
            return Err(Status::InternalServerError);
        };
        let notifications = conn.list_notifications(user.id).await.unwrap_or(Vec::new());
        return Ok(OneOf::B(template::Similar {
            title: format!("Similar to {}", &title),
            alert: flash.map(|f| f.message().to_owned()).unwrap_or_else(String::new),
            config: config.inner().clone(),
            customization,
            noindex: true,
            posts, user, session,
            notifications,
        }));
    }
    let uuid = if let Ok(uuid) = Base32::from_str(&uuid[..]) {
        uuid
    } else {
        return Err(Status::NotFound);
    };
    if let Ok(post_info) = conn.get_post_info_by_uuid(user.id, uuid).await {
        let comments = conn.get_comments_from_post(post_info.id, user.id).await.unwrap_or_else(|e| {
            warn!("Failed to get comments: {:?}", e);
            Vec::new()
        });
        let legacy_comments = conn.get_legacy_comment_info_from_post(post_info.id, user.id).await.unwrap_or_else(|e| {
            warn!("Failed to get comments: {:?}", e);
            Vec::new()
        });
        let post_id = post_info.id;
        let title = post_info.title.clone();
        if user.id != 0 {
            conn.use_notification(post_id, user.id).await.unwrap_or_else(|e| {
                warn!("Failed to consume notification: {:?}", e);
            });
            if let Some(comment) = comments.last() {
                conn.set_readpoint(post_id, user.id, comment.id).await.unwrap_or_else(|e| {
                    warn!("Failed to set readpoint: {:?}", e);
                });
            }
        }
        let is_private = post_info.private || !post_info.visible;
        let noindex = is_private || post_info.noindex;
        let locked = post_info.locked;
        let blog_post = post_info.blog_post;

        let (notifications, polls, is_subscribed) = futures::join!(
            conn.list_notifications(user.id),
            conn.get_poll(user.id, uuid),
            conn.is_subscribed(post_info.id, user.id)
            );
        let (notifications, polls, is_subscribed) = (notifications.unwrap_or(Vec::new()), polls.unwrap_or_else(|e| {
            warn!("Failed to read polls: {:?}", e);
            Vec::new()
        }), is_subscribed.unwrap_or(false));

        let poll_count = polls.len();

        Ok(OneOf::C(template::Comments {
            post_info: post_info,
            alert: flash.map(|f| f.message().to_owned()).unwrap_or_else(String::new),
            starred_by: conn.get_post_starred_by(post_id).await.unwrap_or(Vec::new()),
            config: config.inner().clone(),
            comment_preview_html: String::new(),
            comment_preview_text: String::new(),
            customization,
            noindex, locked,
            comments, user, title, legacy_comments, session,
            notifications, is_private, is_subscribed,
            polls, poll_count,
        }))
    } else if conn.check_invite_token_exists(uuid).await && user.id == 0 {
        Ok(OneOf::D(template::Signup {
            alert: String::new(),
            title: String::from("signup"),
            invite_token: Some(uuid),
            config: config.inner().clone(),
            user: User::default(),
            session: UserSession::default(),
            notifications: Vec::new(),
            noindex: true,
            customization,
        }))
    } else {
        Err(Status::NotFound)
    }
}

#[derive(FromForm)]
struct CommentForm {
    text: String,
    post: Base32,
    preview: Option<String>,
}

#[post("/comment", data = "<comment>")]
async fn post_comment(conn: MoreInterestingConn, login: LoginSession, comment: Form<CommentForm>, config: &State<SiteConfig>) -> Option<Flash<Redirect>> {
    let post_info = conn.get_post_info_by_uuid(login.user.id, comment.post).await.into_option()?;
    let visible = login.user.trust_level > 0 || post_info.private;
    if post_info.locked {
        return Some(Flash::error(
            Redirect::to(comment.post.to_string()),
            "This comment thread is locked"
        ));
    }
    let comment_result = conn.comment_on_post(NewComment {
        post_id: post_info.id,
        text: comment.text.clone(),
        created_by: login.user.id,
        visible,
    }, config.body_format).await;
    match comment_result {
        Ok(_) => (),
        Err(CreateCommentError::TooManyComments) => {
            return Some(Flash::error(
                Redirect::to(comment.post.to_string()),
                "Comment rate limit exceeded; try again later"
            ));
        }
        Err(e) => {
            warn!("Post comment error: {:?}", e);
            return None;
        }
    }
    let subscribed_users = conn.list_subscribed_users(post_info.id).await.unwrap_or_else(|e| {
        warn!("Failed to get subscribed users list for post uuid {}: {:?}", post_info.uuid, e);
        Vec::new()
    });
    for subscribed_user_id in subscribed_users {
        if subscribed_user_id == login.user.id { continue };
        conn.create_notification(NewNotification {
            user_id: subscribed_user_id,
            post_id: post_info.id,
            created_by: login.user.id,
        }).await.unwrap_or_else(|e| warn!("Cannot notify {} on comment: {:?}", subscribed_user_id, e));
    }
    Some(Flash::success(
        Redirect::to(comment.post.to_string()),
        if visible { "Your comment has been posted" } else { "Your comment will be posted after a mod gets a chance to look at it" }
    ))
}

#[post("/preview-comment", data = "<comment>")]
async fn preview_comment(conn: MoreInterestingConn, login: LoginSession, comment: Form<CommentForm>, config: &State<SiteConfig>, customization: Customization) -> Option<template::Comments> {
    use models::{BodyFormat, PrettifyData};
    let (user, session) = (login.user, login.session);
    let post_info = conn.get_post_info_by_uuid(user.id, comment.post).await.into_option()?;
    let comments = conn.get_comments_from_post(post_info.id, user.id).await.unwrap_or_else(|e| {
        warn!("Failed to get comments: {:?}", e);
        Vec::new()
    });
    let legacy_comments = conn.get_legacy_comment_info_from_post(post_info.id, user.id).await.unwrap_or_else(|e| {
        warn!("Failed to get comments: {:?}", e);
        Vec::new()
    });
    let post_id = post_info.id;
    let title = post_info.title.clone();
    let is_private = post_info.private;
    let noindex = is_private || post_info.noindex;
    let is_subscribed = conn.is_subscribed(post_info.id, user.id).await.unwrap_or(false);
    let comment_preview_text = comment.text.clone();
    let comment_preview_html = if comment.preview == Some(String::from("edit")) {
        String::new()
    } else {
        conn.prettify_body(post_id, &comment_preview_text, config.body_format).await
    };
    let uuid = post_info.uuid;
    let locked = post_info.locked;
    let (notifications, polls, is_subscribed) = futures::join!(
        conn.list_notifications(user.id),
        conn.get_poll(user.id, uuid),
        conn.is_subscribed(post_info.id, user.id)
        );
    let (notifications, polls, is_subscribed) = (notifications.unwrap_or(Vec::new()), polls.unwrap_or_else(|e| {
        warn!("Failed to read polls: {:?}", e);
        Vec::new()
    }), is_subscribed.unwrap_or(false));
    let poll_count = polls.len();
    Some(template::Comments {
        alert: String::new(),
        post_info,
        starred_by: conn.get_post_starred_by(post_id).await.unwrap_or(Vec::new()),
        config: config.inner().clone(),
        customization,
        noindex,
        locked, poll_count, polls,
        comments, user, title, legacy_comments, session,
        notifications, is_private, is_subscribed,
        comment_preview_text, comment_preview_html,
    })
}

#[derive(FromForm)]
struct SignupForm {
    username: String,
    password: String,
    invite_token: Option<Base32>,
}

#[post("/signup", data = "<form>")]
async fn signup(conn: MoreInterestingConn, user_agent: UserAgentString<'_>, form: Form<SignupForm>, cookies: &CookieJar<'_>, config: &State<SiteConfig>) -> Result<Flash<Redirect>, Status> {
    if form.username == "" || form.username == "anonymous" {
        return Err(Status::BadRequest);
    }
    let invited_by = if let Some(invite_token) = form.invite_token {
        if let Ok(invite_token) = conn.consume_invite_token(invite_token).await {
            Some(invite_token.invited_by)
        } else {
            return Err(Status::BadRequest)
        }
    } else {
        if config.enable_public_signup {
            None
        } else {
            return Err(Status::BadRequest)
        }
    };
    if let Ok(user) = conn.register_user(NewUser {
        username: form.username.to_owned(),
        password: form.password.to_owned(),
        invited_by,
    }).await {
        if cookies.get("B").is_some() {
            conn.change_user_trust_level(user.id, -2).await.expect("if logging in worked, then so should changing trust level");
        } else if let Some(other_user) = cookies.get("N") {
            if let Ok(other_user) = conn.get_user_by_username(other_user.value()).await {
                if other_user.banned || other_user.trust_level <= -2 {
                    conn.change_user_trust_level(user.id, -2).await.expect("if logging in worked, then so should changing trust level");
                }
            }
        } else if let Some(invited_by) = invited_by.as_ref() {
            if let Ok(invited_by) = conn.get_user_by_id(*invited_by).await {
                if invited_by.trust_level >= 2 {
                    conn.change_user_trust_level(user.id, 1).await.expect("if logging in worked, then so should changing trust level");
                }
            }
        }
        let session = conn.create_session(user.id, user_agent.user_agent).await.expect("failed to allocate a session");
        let cookie = Cookie::build("U", session.uuid.to_string()).path("/").permanent().same_site(SameSite::None).finish();
        cookies.add(cookie);
        let cookie = Cookie::build("N", user.username.to_string()).path("/").permanent().same_site(SameSite::None).finish();
        cookies.add(cookie);
        return Ok(Flash::success(Redirect::to("."), "Congrats, you're in!"));
    }
    Err(Status::BadRequest)
}

#[get("/signup")]
async fn get_public_signup(flash: Option<FlashMessage<'_>>, config: &State<SiteConfig>, customization: Customization) -> Result<template::Signup, Status> {
    if !config.enable_public_signup {
        return Err(Status::NotFound);
    }
    Ok(template::Signup {
        title: String::from("sign up"),
        alert: flash.map(|f| f.message().to_owned()).unwrap_or_else(String::new),
        config: config.inner().clone(),
        invite_token: None,
        user: User::default(),
        session: UserSession::default(),
        notifications: Vec::new(),
        noindex: false,
        customization,
    })
}

#[get("/settings")]
async fn get_settings(conn: MoreInterestingConn, login: LoginSession, flash: Option<FlashMessage<'_>>, config: &State<SiteConfig>, customization: Customization) -> template::Settings {
    let user = login.user;
    let session = login.session;
    template::Settings {
        title: String::from("settings"),
        alert: flash.map(|f| f.message().to_owned()).unwrap_or_else(String::new),
        config: config.inner().clone(),
        notifications: conn.list_notifications(user.id).await.unwrap_or(Vec::new()),
        noindex: true,
        customization,
        user, session,
    }
}

#[derive(FromForm)]
struct DarkModeForm {
    active: bool,
}

#[post("/set-dark-mode", data="<form>")]
async fn set_dark_mode<'a>(conn: MoreInterestingConn, login: LoginSession, form: Form<DarkModeForm>) -> Flash<Redirect> {
    match conn.set_dark_mode(login.user.id, form.active).await {
        Ok(()) => {
            Flash::success(Redirect::to(uri!(get_settings)), "")
        }
        Err(e) => {
            dbg!(e);
            Flash::error(Redirect::to(uri!(get_settings)), "Something went horribly wrong")
        }
    }
}

#[post("/set-big-mode", data="<form>")]
async fn set_big_mode<'a>(conn: MoreInterestingConn, login: LoginSession, form: Form<DarkModeForm>) -> Flash<Redirect> {
    match conn.set_big_mode(login.user.id, form.active).await {
        Ok(()) => {
            Flash::success(Redirect::to(uri!(get_settings)), "")
        }
        Err(e) => {
            dbg!(e);
            Flash::error(Redirect::to(uri!(get_settings)), "Something went horribly wrong")
        }
    }
}

#[post("/create-invite")]
async fn create_invite<'a>(conn: MoreInterestingConn, login: LoginSession, config: &State<SiteConfig>) -> Flash<Redirect> {
    match conn.create_invite_token(login.user.id).await {
        Ok(invite_token) => {
            let public_url = &config.public_url;
            let created_invite_url = public_url.join(&invite_token.uuid.to_string()).expect("base128 is a valid relative URL");
            Flash::success(Redirect::to(uri!(get_settings)), format!("To invite them, send them this link: {}", created_invite_url))
        }
        Err(e) => {
            dbg!(e);
            Flash::error(Redirect::to(uri!(get_settings)), "Failed to create invite")
        }
    }
}

#[get("/tags.json")]
async fn get_tags_json(conn: MoreInterestingConn) -> Option<content::Json<String>> {
    let tags = conn.get_all_tags().await.unwrap_or(Vec::new());
    let tags_map: serde_json::Map<String, serde_json::Value> = tags.into_iter().map(|tag| {
        (tag.name, tag.description.unwrap_or(String::new()).into())
    }).collect();
    let json = serde_json::to_string(&tags_map).ok()?;
    Some(content::Json(json))
}

#[get("/domains.json?<search>")]
async fn get_domains_json(conn: MoreInterestingConn, search: String) -> Option<content::Json<String>> {
    let domains = conn.search_domains(search).await.unwrap_or(Vec::new());
    let domains_map: serde_json::Map<String, serde_json::Value> = domains.into_iter().map(|domain| {
        (domain.hostname, String::new().into())
    }).collect();
    let json = serde_json::to_string(&domains_map).ok()?;
    Some(content::Json(json))
}

#[get("/tags")]
async fn get_tags(conn: MoreInterestingConn, login: Option<LoginSession>, config: &State<SiteConfig>, customization: Customization) -> template::Tags {
    let (user, session) = login.map(|l| (l.user, l.session)).unwrap_or((User::default(), UserSession::default()));
    assert!((user.id == 0) ^ (user.username != ""));
    let tags = conn.get_all_tags().await.unwrap_or(Vec::new());
    template::Tags {
        alert: String::new(),
        title: String::from("all tags"),
        config: config.inner().clone(),
        notifications: conn.list_notifications(user.id).await.unwrap_or(Vec::new()),
        noindex: false,
        customization,
        tags, user, session,
    }
}

#[get("/faq")]
async fn faq(conn: MoreInterestingConn, login: Option<LoginSession>, config: &State<SiteConfig>, customization: Customization) -> template::Faq {
    let (user, session) = login.map(|l| (l.user, l.session)).unwrap_or((User::default(), UserSession::default()));

    assert!((user.id == 0) ^ (user.username != ""));
    let raw_html = conn.get_customization_value("faq_html").await.unwrap_or_else(||String::from("To fill this in, modify the faq_html variable in the admin / customization screen"));
    let faq_title = customization.faq_title.clone();
    template::Faq {
        alert: String::new(),
        title: faq_title,
        config: config.inner().clone(),
        notifications: conn.list_notifications(user.id).await.unwrap_or(Vec::new()),
        noindex: false,
        customization,
        user, raw_html, session,
    }
}

#[get("/@")]
async fn invite_tree(conn: MoreInterestingConn, login: Option<LoginSession>, config: &State<SiteConfig>, customization: Customization) -> template::InviteTree {
    let (user, session) = login.map(|l| (l.user, l.session)).unwrap_or((User::default(), UserSession::default()));

    assert!((user.id == 0) ^ (user.username != ""));
    fn handle_invite_tree(invite_tree_html: &mut String, invite_tree: &HashMap<i32, Vec<User>>, id: i32) {
        if let Some(users) = invite_tree.get(&id) {
            for user in users {
                invite_tree_html.push_str("<li>");
                invite_tree_html.push_str(&escape(&user.username).to_string());
                invite_tree_html.push_str("<ul class=subtree>");
                handle_invite_tree(invite_tree_html, invite_tree, user.id);
                invite_tree_html.push_str("</ul></li>");
            }
        }
    }
    let mut raw_html = String::from("<ul class=tree>");
    if config.enable_user_directory {
        handle_invite_tree(&mut raw_html, &conn.get_invite_tree().await, 0);
    }
    raw_html.push_str("</ul>");
    template::InviteTree {
        alert: String::new(),
        title: String::from("user invite tree"),
        config: config.inner().clone(),
        notifications: conn.list_notifications(user.id).await.unwrap_or(Vec::new()),
        noindex: !config.enable_user_directory,
        customization,
        user, raw_html, session,
    }
}

#[get("/admin/tags")]
async fn get_admin_tags(conn: MoreInterestingConn, customization: Customization, login: ModeratorSession, flash: Option<FlashMessage<'_>>, config: &State<SiteConfig>) -> template::AdminTags {
    let tags = conn.get_all_tags().await.unwrap_or(Vec::new());
    template::AdminTags {
        title: String::from("add or edit tags"),
        user: login.user,
        session: login.session,
        alert: flash.map(|f| f.message().to_owned()).unwrap_or_else(String::new),
        config: config.inner().clone(),
        page: AdminPageId::Tags,
        tags, customization,
    }
}

#[get("/admin/flags")]
async fn get_admin_flags(conn: MoreInterestingConn, customization: Customization, login: ModeratorSession, flash: Option<FlashMessage<'_>>, config: &State<SiteConfig>) -> template::AdminFlags {
    let post_flags = conn.get_recent_post_flags().await;
    template::AdminFlags {
        title: String::from("recent flags"),
        user: login.user,
        session: login.session,
        alert: flash.map(|f| f.message().to_owned()).unwrap_or_else(String::new),
        config: config.inner().clone(),
        page: AdminPageId::Flags,
        post_flags, customization,
    }
}

#[get("/admin/comment-flags")]
async fn get_admin_comment_flags(conn: MoreInterestingConn, customization: Customization, login: ModeratorSession, flash: Option<FlashMessage<'_>>, config: &State<SiteConfig>) -> template::AdminCommentFlags {
    let comment_flags = conn.get_recent_comment_flags().await;
    template::AdminCommentFlags {
        title: String::from("recent flags"),
        user: login.user,
        session: login.session,
        alert: flash.map(|f| f.message().to_owned()).unwrap_or_else(String::new),
        config: config.inner().clone(),
        page: AdminPageId::CommentFlags,
        comment_flags, customization,
    }
}

#[get("/admin/users")]
async fn get_admin_users(conn: MoreInterestingConn, customization: Customization, login: ModeratorSession, flash: Option<FlashMessage<'_>>, config: &State<SiteConfig>) -> template::AdminUsers {
    let users_list = conn.get_recent_users(String::new()).await.unwrap_or(Vec::new());
    template::AdminUsers {
        title: String::from("recently logged in users"),
        user: login.user,
        session: login.session,
        alert: flash.map(|f| f.message().to_owned()).unwrap_or_else(String::new),
        config: config.inner().clone(),
        page: AdminPageId::Users,
        username: String::new(),
        users_list, customization,
    }
}

#[get("/admin/users?<username>")]
async fn get_admin_users_search(conn: MoreInterestingConn, customization: Customization, login: ModeratorSession, flash: Option<FlashMessage<'_>>, config: &State<SiteConfig>, username: &str) -> template::AdminUsers {
    let users_list = conn.get_recent_users(username.to_owned()).await.unwrap_or(Vec::new());
    template::AdminUsers {
        title: String::from("recently logged in users"),
        user: login.user,
        session: login.session,
        alert: flash.map(|f| f.message().to_owned()).unwrap_or_else(String::new),
        config: config.inner().clone(),
        page: AdminPageId::Users,
        username: username.to_owned(),
        users_list, customization,
    }
}

#[derive(FromForm)]
struct EditTagsForm {
    name: String,
    description: Option<String>,
}

#[post("/admin/tags", data = "<form>")]
async fn admin_tags(conn: MoreInterestingConn, _login: ModeratorSession, form: Form<EditTagsForm>) -> Flash<Redirect> {
    let name = if form.name.starts_with('#') { &form.name[1..] } else { &form.name[..] };
    match conn.create_or_update_tag(&NewTag {
        name: name.to_owned(),
        description: form.description.clone()
    }).await {
        Ok(_) => {
            Flash::success(Redirect::to(uri!(get_admin_tags)), "Updated site tags")
        }
        Err(e) => {
            debug!("Unable to update site tags: {:?}", e);
            Flash::error(Redirect::to(uri!(get_admin_tags)), "Unable to update site tags")
        }
    }
}

#[get("/admin/domains")]
async fn get_admin_domains(conn: MoreInterestingConn, customization: Customization, login: ModeratorSession, flash: Option<FlashMessage<'_>>, config: &State<SiteConfig>) -> template::AdminDomains {
    let domain_synonyms = conn.get_all_domain_synonyms().await.unwrap_or(Vec::new());
    template::AdminDomains {
        title: String::from("add or edit domain synonyms"),
        user: login.user,
        session: login.session,
        alert: flash.map(|f| f.message().to_owned()).unwrap_or_else(String::new),
        config: config.inner().clone(),
        page: AdminPageId::Domains,
        domain_synonyms, customization,
    }
}

#[derive(FromForm)]
struct EditDomainSynonymForm {
    from_hostname: String,
    to_hostname: String,
}

#[post("/admin/domains", data = "<form>")]
async fn admin_domains(conn: MoreInterestingConn, _login: ModeratorSession, form: Form<EditDomainSynonymForm>) -> Option<Flash<Redirect>> {
    let to_domain_id = if let Ok(to_domain) = conn.get_domain_by_hostname(&form.to_hostname).await {
        to_domain.id
    } else {
        conn.create_domain(NewDomain {
            hostname: form.to_hostname.clone(),
            is_www: false,
            is_https: false,
        }).await.ok()?.id
    };
    let from_hostname = form.from_hostname.clone();
    match conn.add_domain_synonym(DomainSynonym {
        from_hostname, to_domain_id
    }).await {
        Ok(_) => {
            Some(Flash::success(Redirect::to(uri!(get_admin_domains)), "Updated domain synonym"))
        }
        Err(e) => {
            warn!("Unable to update domain synonym: {:?}", e);
            Some(Flash::error(Redirect::to(uri!(get_admin_domains)), "Unable to update domain synonym"))
        }
    }
}

#[get("/admin/customization")]
async fn get_admin_customization(conn: MoreInterestingConn, customization: Customization, login: ModeratorSession, flash: Option<FlashMessage<'_>>, config: &State<SiteConfig>) -> template::AdminCustomization {
    let site_customization = conn.get_customizations().await.unwrap_or(Vec::new());
    template::AdminCustomization {
        title: String::from("site customization"),
        user: login.user,
        session: login.session,
        alert: flash.map(|f| f.message().to_owned()).unwrap_or_else(String::new),
        config: config.inner().clone(),
        page: AdminPageId::Customization,
        site_customization, customization,
    }
}

#[derive(FromForm)]
struct EditSiteCustomizationForm {
    name: String,
    value: String,
}

#[post("/admin/customization", data = "<form>")]
async fn admin_customization(conn: MoreInterestingConn, _login: ModeratorSession, form: Form<EditSiteCustomizationForm>) -> Option<Flash<Redirect>> {
    let result = conn.set_customization(SiteCustomization {
        name: form.name.clone(),
        value: form.value.clone(),
    }).await;
    match result {
        Ok(_) => {
            Some(Flash::success(Redirect::to(uri!(get_admin_customization)), "Updated customization"))
        }
        Err(e) => {
            warn!("Unable to update customization: {:?}", e);
            Some(Flash::error(Redirect::to(uri!(get_admin_customization)), "Unable to update customization"))
        }
    }
}

#[derive(FromForm)]
struct GetEditPost {
    post: Base32,
}

#[get("/edit-post?<post..>")]
async fn get_edit_post(conn: MoreInterestingConn, login: ModeratorSession, flash: Option<FlashMessage<'_>>, post: GetEditPost, config: &State<SiteConfig>, customization: Customization) -> Option<template::EditPost> {
    let post_info = conn.get_post_info_by_uuid(login.user.id, post.post).await.ok()?;
    let post = conn.get_post_by_uuid(post.post).await.ok()?;
    let user = login.user;
    Some(template::EditPost {
        title: String::from("edit post"),
        session: login.session,
        alert: flash.map(|f| f.message().to_owned()).unwrap_or_else(String::new),
        config: config.inner().clone(),
        notifications: conn.list_notifications(user.id).await.unwrap_or(Vec::new()),
        excerpt: post.excerpt,
        noindex: true,
        post_info, user, customization,
    })
}

#[derive(FromForm)]
struct EditPostForm {
    post: Base32,
    title: String,
    url: Option<String>,
    excerpt: Option<String>,
    delete: bool,
}

#[post("/edit-post", data = "<form>")]
async fn edit_post(conn: MoreInterestingConn, login: ModeratorSession, form: Form<EditPostForm>, config: &State<SiteConfig>) -> Result<Flash<Redirect>, Status> {
    let post_info = if let Ok(post_info) = conn.get_post_info_by_uuid(login.user.id, form.post).await {
        post_info
    } else {
        return Err(Status::NotFound);
    };
    let post_id = post_info.id;
    let post = if let Ok(post) = conn.get_post_by_uuid(post_info.uuid).await {
        post
    } else {
        return Err(Status::NotFound);
    };
    let url = if let Some(url) = &form.url {
        if url == "" { None } else { Some(url.clone()) }
    } else {
        None
    };
    let excerpt = if let Some(excerpt) = &form.excerpt {
        if excerpt == "" { None } else { Some(excerpt.clone()) }
    } else {
        None
    };
    if form.delete {
        match conn.delete_post(post_id).await {
            Ok(_) => {
                if !post_info.private {
                    conn.mod_log_delete_post(
                        login.user.id,
                        post_info.uuid,
                        post_info.title,
                        post_info.url.unwrap_or(String::new()),
                        post.excerpt.unwrap_or(String::new()),
                    ).await.expect("if updating the post worked, then so should logging");
                }
                return Ok(Flash::success(Redirect::to(uri!(get_mod_queue)), "Deleted post"))
            },
            Err(e) => {
                warn!("{:?}", e);
                return Err(Status::InternalServerError)
            },
        }
    } else {
        let title = form.title.clone();
        let url = form.url.clone();
        match conn.update_post(post_id, !post_info.visible && post_info.score == 0, NewPost {
            title,
            url: url.clone(),
            submitted_by: login.user.id,
            excerpt,
            visible: login.user.trust_level >= 3,
            private: post_info.private,
            blog_post: post_info.blog_post,
        }, config.body_format).await {
            Ok(_) => {
                if !post_info.private {
                    conn.mod_log_edit_post(
                        login.user.id,
                        post_info.uuid,
                        post_info.title,
                        form.title.clone(),
                        post_info.url.unwrap_or(String::new()),
                        url.unwrap_or(String::new()),
                        post.excerpt.unwrap_or(String::new()),
                        form.excerpt.clone().unwrap_or(String::new()),
                    ).await.expect("if updating the post worked, then so should logging");
                }
                Ok(Flash::success(Redirect::to(form.post.to_string()), "Updated post"))
            },
            Err(e) => {
                warn!("{:?}", e);
                Err(Status::InternalServerError)
            },
        }
    }
}

#[derive(FromForm)]
struct GetEditComment {
    comment: i32,
}

#[get("/edit-comment?<comment..>")]
async fn get_edit_comment(conn: MoreInterestingConn, login: LoginSession, flash: Option<FlashMessage<'_>>, comment: GetEditComment, config: &State<SiteConfig>, customization: Customization) -> Option<template::EditComment> {
    let comment = conn.get_comment_by_id(comment.comment).await.ok()?;
    if login.user.trust_level < 3 && comment.created_by != login.user.id {
        return None;
    }
    let user = login.user;
    Some(template::EditComment {
        title: String::from("edit comment"),
        alert: flash.map(|f| f.message().to_owned()).unwrap_or_else(String::new),
        config: config.inner().clone(),
        session: login.session,
        notifications: conn.list_notifications(user.id).await.unwrap_or(Vec::new()),
        noindex: true,
        customization, comment, user,
    })
}

#[derive(FromForm)]
struct EditCommentForm {
    comment: i32,
    text: String,
    delete: bool,
}

#[post("/edit-comment", data = "<form>")]
async fn edit_comment(conn: MoreInterestingConn, login: LoginSession, form: Form<EditCommentForm>, config: &State<SiteConfig>) -> Result<Flash<Redirect>, Status> {
    let user = login.user;
    let comment = conn.get_comment_by_id(form.comment).await.map_err(|_| Status::NotFound)?;
    let post = conn.get_post_info_from_comment(form.comment).await.map_err(|_| Status::NotFound)?;
    if user.trust_level < 3 && comment.created_by != user.id {
        return Err(Status::NotFound);
    }
    if post.locked {
        return Ok(Flash::error(
            Redirect::to(post.uuid.to_string()),
            "This comment thread is locked"
        ));
    }
    if form.delete && user.trust_level >= 3 {
        match conn.delete_comment(comment.id).await {
            Ok(_) => {
                if !post.private {
                    conn.mod_log_delete_comment(
                        user.id,
                        comment.id,
                        post.uuid,
                        comment.text,
                    ).await.expect("if updating the comment worked, then so should logging");
                }
                Ok(Flash::success(Redirect::to(uri!(get_mod_queue)), "Deleted comment"))
            },
            Err(e) => {
                warn!("{:?}", e);
                Err(Status::InternalServerError)
            },
        }
    } else {
        match conn.update_comment(post.id, form.comment, form.text.clone(), config.body_format).await {
            Ok(_) => {
                if !post.private {
                    conn.mod_log_edit_comment(
                        user.id,
                        comment.id,
                        post.uuid,
                        comment.text,
                        form.text.clone(),
                    ).await.expect("if updating the comment worked, then so should logging");
                }
                Ok(Flash::success(Redirect::to(post.uuid.to_string()), "Updated comment"))
            },
            Err(e) => {
                warn!("{:?}", e);
                Err(Status::InternalServerError)
            },
        }
    }
}

#[derive(FromForm)]
struct GetReplyComment {
    comment: i32,
    post: Base32,
}

#[get("/reply-comment?<comment..>")]
async fn get_reply_comment(conn: MoreInterestingConn, login: LoginSession, flash: Option<FlashMessage<'_>>, comment: GetReplyComment, config: &State<SiteConfig>, customization: Customization) -> Option<template::ReplyComment> {
    let post = conn.get_post_info_by_uuid(login.user.id, comment.post).await.ok()?;
    let comment = conn.get_comment_info_by_id(comment.comment, login.user.id).await.ok()?;
    let user = login.user;
    if comment.post_id != post.id { return None; }
    let is_subscribed = conn.is_subscribed(post.id, user.id).await.ok()?;
    Some(template::ReplyComment {
        title: String::from("reply to comment"),
        alert: flash.map(|f| f.message().to_owned()).unwrap_or_else(String::new),
        config: config.inner().clone(),
        post: post,
        session: login.session,
        notifications: conn.list_notifications(user.id).await.unwrap_or(Vec::new()),
        noindex: true,
        comment, customization, user, is_subscribed,
    })
}

#[derive(FromForm)]
struct ChangePasswordForm {
    old_password: String,
    new_password: String,
}

#[post("/change-password", data = "<form>")]
async fn change_password(conn: MoreInterestingConn, login: LoginSession, form: Form<ChangePasswordForm>) -> Result<Flash<Redirect>, Status> {
    let user = login.user;
    if form.new_password == "" {
        return Err(Status::BadRequest);
    }
    if conn.authenticate_user(&UserAuth {
        username: user.username.clone(),
        password: form.old_password.clone(),
    }).await.is_none() {
        return Err(Status::BadRequest);
    }
    match conn.change_user_password(user.id, &form.new_password).await {
        Ok(()) => Ok(Flash::success(Redirect::to(uri!(get_settings)), "Done!")),
        Err(_) => Err(Status::BadRequest),
    }
}

#[get("/mod-queue")]
async fn get_mod_queue(conn: MoreInterestingConn, login: ModeratorSession, flash: Option<FlashMessage<'_>>, config: &State<SiteConfig>, customization: Customization) -> Result<template::ModQueue, Status> {
    let user = login.user;
    let session = login.session;
    let mut queue = Vec::new();
    for comment in conn.find_moderated_comments(user.id).await.unwrap_or(Vec::new()) {
        let post = conn.get_post_info_from_comment(comment.id).await.map_err(|_| Status::NotFound)?;
        queue.push(ModQueueItem::Comment {
            post,
            comment,
        });
    }
    for post in conn.find_moderated_posts(user.id).await.unwrap_or(Vec::new()) {
        let comments = conn.get_comments_from_post(post.id, user.id).await.unwrap_or_else(|e| {
            warn!("Failed to get comments: {:?}", e);
            Vec::new()
        });
        queue.push(ModQueueItem::Post {
            post,
            comments,
        });
    }
    queue.sort_by_key(|item| {
        match item {
            ModQueueItem::Post { post, .. } => post.created_at,
            ModQueueItem::Comment { comment, .. } => comment.created_at,
        }
    });
    Ok(template::ModQueue {
        title: String::from("mod queue"),
        alert: flash.map(|f| f.message().to_owned()).unwrap_or_else(String::new),
        config: config.inner().clone(),
        notifications: conn.list_notifications(user.id).await.unwrap_or(Vec::new()),
        mod_queue: queue,
        noindex: false,
        customization,
        user, session,
    })
}

#[derive(FromForm)]
struct ModeratePostForm {
    post: Base32,
    action: String,
}

#[post("/moderate-post", data = "<form>")]
async fn moderate_post(conn: MoreInterestingConn, login: ModeratorSession, form: Form<ModeratePostForm>) -> Result<Flash<Redirect>, Status> {
    let user = login.user;
    let post_info = if let Ok(post_info) = conn.get_post_info_by_uuid(user.id, form.post).await {
        post_info
    } else {
        return Err(Status::NotFound);
    };
    let post_id = post_info.id;
    let post = if let Ok(post) = conn.get_post_by_uuid(post_info.uuid).await {
        post
    } else {
        return Err(Status::NotFound);
    };
    if form.action == "approve" {
        match conn.approve_post(post_id).await {
            Ok(_) => {
                if !post_info.private {
                    conn.mod_log_approve_post(
                        user.id,
                        post_info.uuid,
                        post_info.title,
                        post_info.url.unwrap_or(String::new()),
                        post.excerpt.unwrap_or(String::new()),
                    ).await.expect("if updating the post worked, then so should logging");
                }
                Ok(Flash::success(Redirect::to(uri!(get_mod_queue)), "Approved post"))
            },
            Err(e) => {
                warn!("{:?}", e);
                Err(Status::InternalServerError)
            },
        }
    } else {
        match conn.delete_post(post_id).await {
            Ok(_) => {
                if !post_info.private {
                    conn.mod_log_delete_post(
                        user.id,
                        post_info.uuid,
                        post_info.title,
                        post_info.url.unwrap_or(String::new()),
                        post.excerpt.unwrap_or(String::new()),
                    ).await.expect("if updating the post worked, then so should logging");
                }
                Ok(Flash::success(Redirect::to(uri!(get_mod_queue)), "Deleted post"))
            },
            Err(e) => {
                warn!("{:?}", e);
                Err(Status::InternalServerError)
            },
        }
    }
}

#[derive(FromForm)]
struct CreatePollForm {
    post: Base32,
    poll_title: String,
    poll_choices: String,
}

#[post("/create-poll", data = "<form>")]
async fn create_poll(conn: MoreInterestingConn, login: ModeratorSession, form: Form<CreatePollForm>) -> Result<Flash<Redirect>, Status> {
    let post_info = if let Ok(post_info) = conn.get_post_info_by_uuid(login.user.id, form.post).await {
        post_info
    } else {
        return Err(Status::NotFound);
    };
    let post_id = post_info.id;
    let title = form.poll_title.clone();
    let choices = form.poll_choices.lines().filter_map(|c| {
        match c.trim() {
            "" => None,
            c => Some(String::from(c)),
        }
    }).collect();
    match conn.create_poll(post_id, title, choices, login.user.id).await {
        Ok(poll) => {
            if !post_info.private {
                conn.mod_log_poll_post(
                    login.user.id,
                    post_info.uuid,
                    poll.title,
                    poll.id,
                ).await.expect("if updating the post worked, then so should logging");
            }
            Ok(Flash::success(Redirect::to(form.post.to_string()), "Added poll to post"))
        },
        Err(e) => {
            warn!("{:?}", e);
            Err(Status::InternalServerError)
        },
    }
}
#[derive(FromForm)]
struct ClosePollForm {
    poll: i32,
    post: Base32,
}

#[post("/close-poll", data = "<form>")]
async fn close_poll(conn: MoreInterestingConn, login: ModeratorSession, form: Form<ClosePollForm>) -> Result<Flash<Redirect>, Status> {
    let post_info = if let Ok(post_info) = conn.get_post_info_by_uuid(login.user.id, form.post).await {
        post_info
    } else {
        return Err(Status::NotFound);
    };
    match conn.close_poll(form.poll).await {
        Ok(poll) => {
            if !post_info.private {
                conn.mod_log_close_poll(
                    login.user.id,
                    form.post,
                    poll.id,
                ).await.expect("if updating the post worked, then so should logging");
            }
            Ok(Flash::success(Redirect::to(form.post.to_string()), "Added poll to post"))
        },
        Err(e) => {
            warn!("{:?}", e);
            Err(Status::InternalServerError)
        },
    }
}

#[derive(FromForm)]
struct VotePollForm {
    post: Base32,
    choice: i32,
    score: i32,
}

#[post("/vote-poll", data = "<form>")]
async fn vote_poll(conn: MoreInterestingConn, login: LoginSession, form: Form<VotePollForm>) -> Result<Flash<Redirect>, Status> {
    match conn.vote_poll(login.user.id, form.post, form.choice, form.score).await {
        Ok(_) => {
            Ok(Flash::success(Redirect::to(form.post.to_string()), "Vote cast"))
        },
        Err(e) => {
            warn!("{:?}", e);
            Err(Status::InternalServerError)
        },
    }
}

#[derive(FromForm)]
struct BannerPostForm {
    post: Base32,
    banner_title: String,
    banner_desc: String,
}

#[post("/banner-post", data = "<form>")]
async fn banner_post(conn: MoreInterestingConn, login: ModeratorSession, form: Form<BannerPostForm>) -> Result<Flash<Redirect>, Status> {
    let post_info = if let Ok(post_info) = conn.get_post_info_by_uuid(login.user.id, form.post).await {
        post_info
    } else {
        return Err(Status::NotFound);
    };
    let banner_title = if form.banner_title != "" { Some(form.banner_title.clone()) } else { None };
    let banner_desc = if form.banner_desc != "" { Some(form.banner_desc.clone()) } else { None };
    let post_id = post_info.id;
    match conn.banner_post(post_id, banner_title.clone(), banner_desc.clone()).await {
        Ok(_) => {
            if !post_info.private {
                conn.mod_log_banner_post(
                    login.user.id,
                    post_info.uuid,
                    banner_title.unwrap_or(String::new()),
                    banner_desc.unwrap_or(String::new()),
                ).await.expect("if updating the post worked, then so should logging");
            }
            Ok(Flash::success(Redirect::to(uri!(get_mod_queue)), "Added banner to post"))
        },
        Err(e) => {
            warn!("{:?}", e);
            Err(Status::InternalServerError)
        },
    }
}

#[derive(FromForm)]
struct AdvancedPostForm {
    post: Base32,
    noindex: bool,
    locked: bool,
}

#[post("/advanced-post", data = "<form>")]
async fn advanced_post(conn: MoreInterestingConn, login: ModeratorSession, form: Form<AdvancedPostForm>) -> Result<Flash<Redirect>, Status> {
    let post_info = if let Ok(post_info) = conn.get_post_info_by_uuid(login.user.id, form.post).await {
        post_info
    } else {
        return Err(Status::NotFound);
    };
    let post_id = post_info.id;
    if post_info.locked != form.locked {
        match conn.lock_post(post_id, form.locked).await {
            Ok(_) => {
                if !post_info.private {
                    conn.mod_log_lock(
                        login.user.id,
                        post_info.uuid,
                        form.locked,
                    ).await.expect("if updating the post worked, then so should logging");
                }
            },
            Err(e) => {
                warn!("{:?}", e);
                return Err(Status::InternalServerError)
            },
        }
    }
    if post_info.noindex != form.noindex {
        match conn.noindex_post(post_id, form.noindex).await {
            Ok(_) => {
                if !post_info.private {
                    conn.mod_log_noindex(
                        login.user.id,
                        post_info.uuid,
                        form.noindex,
                    ).await.expect("if updating the post worked, then so should logging");
                }
            },
            Err(e) => {
                warn!("{:?}", e);
                return Err(Status::InternalServerError)
            },
        }
    }
    Ok(Flash::success(Redirect::to(post_info.uuid.to_string()), "Changed settings on post"))
}

#[derive(FromForm)]
struct ModerateCommentForm {
    comment: i32,
    action: String,
}

#[post("/moderate-comment", data = "<form>")]
async fn moderate_comment(conn: MoreInterestingConn, login: ModeratorSession, form: Form<ModerateCommentForm>) -> Result<Flash<Redirect>, Status> {
    let comment_info = if let Ok(comment) = conn.get_comment_by_id(form.comment).await {
        comment
    } else {
        return Err(Status::NotFound);
    };
    let post_info = conn.get_post_info_from_comment(comment_info.id).await.unwrap();
    if form.action == "approve" {
        match conn.approve_comment(comment_info.id).await {
            Ok(_) => {
                if !post_info.private {
                    conn.mod_log_approve_comment(
                        login.user.id,
                        comment_info.id,
                        post_info.uuid,
                        comment_info.text,
                    ).await.expect("if updating the comment worked, then so should logging");
                }
                Ok(Flash::success(Redirect::to(uri!(get_mod_queue)), "Approved comment"))
            },
            Err(e) => {
                warn!("{:?}", e);
                Err(Status::InternalServerError)
            },
        }
    } else {
        match conn.delete_comment(comment_info.id).await {
            Ok(_) => {
                if !post_info.private {
                    conn.mod_log_delete_comment(
                        login.user.id,
                        comment_info.id,
                        post_info.uuid,
                        comment_info.text,
                    ).await.expect("if updating the comment worked, then so should logging");
                }
                Ok(Flash::success(Redirect::to(uri!(get_mod_queue)), "Deleted comment"))
            },
            Err(e) => {
                warn!("{:?}", e);
                Err(Status::InternalServerError)
            },
        }
    }
}

#[get("/random?<params..>")]
async fn random(conn: MoreInterestingConn, login: Option<LoginSession>, params: Option<IndexParams>, flash: Option<FlashMessage<'_>>, config: &State<SiteConfig>, customization: Customization) -> Option<template::IndexRandom> {
    let (user, session) = login.map(|l| (l.user, l.session)).unwrap_or((User::default(), UserSession::default()));
    let tag_param = params.as_ref().and_then(|params| Some(params.tag.as_ref()?.to_string())).unwrap_or_else(String::new);
    let domain = params.as_ref().and_then(|params| Some(params.domain.as_ref()?.to_string())).unwrap_or_else(String::new);
    let (search, tags) = parse_index_params(&conn, &user, params).await?;
    let before_date_param = search.before_date;
    let after_date_param = search.after_date;
    let search = PostSearch {
        order_by: PostSearchOrderBy::Random,
        blog_post: Some(false),
        .. search
    };
    let keywords_param = search.keywords.clone();
    let title_param = search.title.clone();
    let is_home = tag_param == "" && domain == "" && keywords_param == "";
    let posts = conn.search_posts(&search).await.ok()?;
    let notifications = conn.list_notifications(user.id).await.unwrap_or(Vec::new());
    let noindex = keywords_param != "" && (search.after_post_id != 0 || search.search_page != 0);
    Some(template::IndexRandom {
        title: String::from("random"),
        next_search_page: search.search_page + 1,
        alert: flash.map(|f| f.message().to_owned()).unwrap_or_else(String::new),
        config: config.inner().clone(),
        customization, before_date_param, after_date_param,
        is_home, keywords_param, title_param,
        user, posts, session, tags, tag_param, domain,
        notifications, noindex,
    })
}

#[get("/identicon/<id>")]
async fn identicon(id: Base32, referrer: ReferrerString<'_>, config: &State<SiteConfig>) -> Option<CacheForever<content::Custom<Vec<u8>>>> {
    let referrer = Url::parse(referrer.referrer).ok();
    if referrer.is_some() && referrer.as_ref().and_then(|u| u.host()) != config.public_url.host() {
        return None;
    }
    let img = render_avatar(id.into_u64() as u32);
    let png = to_png(img);
    Some(CacheForever(content::Custom(ContentType::from_str("image/png").unwrap(), png)))
}

#[get("/conv/<id>")]
async fn conv_legacy_id(id: Base32) -> String {
  id.into_i64().to_string()
}

#[get("/id/<id>")]
async fn redirect_legacy_id(id: i64) -> Redirect {
    Redirect::moved(format!("/{}", Base32::from(id)))
}

#[get("/robots.txt")]
async fn robots_txt() -> String {
// The important feature is that these numbers are all mutually prime.
// That way, when two different bots get different crawl-delays, they
// won't harmonize, even if the bot operator themselves is too stupid
// to randomize their delays (I'm looking at you, mj12bot).
let crawl_delay = match rand::random() {
    0...31u8 => 23,
    32...63 => 29,
    64...95 => 31,
    96...127 => 37,
    128...159 => 41,
    160...191 => 43,
    192...223 => 47,
    _ => 53,
};
format!("User-agent: *
Disallow: /mod-log
Disallow: /login
Disallow: /signup
Disallow: /vote
Disallow: /vote-comment
Disallow: /submit
Disallow: /identicon
Disallow: /random
Crawl-delay: {}

User-agent: seo spider
Disallow: /

User-agent: Seekport
Disallow: /

User-agent: AhrefsBot
Disallow: /

User-agent: SemrushBot-BA
Disallow: /

User-agent: SemrushBot-SA
Disallow: /

User-agent: SemrushBot
Disallow: /

User-agent: MJ12bot
Disallow: /

User-agent: BLEXBot
Disallow: /

User-agent: MauiBot
Disallow: /
    
User-agent: MegaIndex
Disallow: /

User-agent: serpstatbot
Disallow: /

User-agent: DotBot
Disallow: /

User-agent: omgili
Disallow: /

User-agent: moreover
Disallow: /
", crawl_delay)
}

#[rocket::launch]
fn launch() -> rocket::Rocket<rocket::Build> {
    //env_logger::init();
    rocket::build()
        .attach(MoreInterestingConn::fairing())
        .attach(fairing::AdHoc::config::<SiteConfig>())
        .attach(fairing::AdHoc::on_liftoff("setup", |rocket| {
            Box::pin(async move {
                let conn = MoreInterestingConn::get_one(&rocket).await.unwrap();
                if !conn.has_users().await.unwrap_or(true) {
                    let config = rocket.state::<SiteConfig>();
                    if let Some(config) = config {
                        if config.init_username == "" || config.init_password == "" {
                            return;
                        }
                        let username = config.init_username.clone();
                        let password = config.init_password.clone();
                        let user = conn.register_user(NewUser {
                            username,
                            password,
                            invited_by: None,
                        }).await.expect("registering the initial user should always succeed");
                        conn.change_user_trust_level(user.id, 4).await.expect("to make the initial user an admin");
                    }
                }
            })
        }))
        .mount("/", routes![index, blog_index, advanced_search, login_form, login, logout, create_link_form, create_post_form, create, post_preview, submit_preview, get_comments, vote, signup, get_settings, create_invite, invite_tree, change_password, post_comment, vote_comment, get_admin_tags, admin_tags, get_tags, edit_post, get_edit_post, edit_comment, get_edit_comment, set_dark_mode, set_big_mode, mod_log, get_mod_queue, moderate_post, moderate_comment, get_public_signup, random, redirect_legacy_id, latest, rss, blog_rss, top, banner_post, advanced_post, robots_txt, search_comments, new, get_admin_domains, admin_domains, create_message_form, create_message, subscriptions, post_subscriptions, get_reply_comment, preview_comment, get_admin_customization, admin_customization, conv_legacy_id, get_tags_json, get_domains_json, get_admin_flags, get_admin_comment_flags, get_admin_users, get_admin_users_search, faq, identicon, create_poll, close_poll, vote_poll])
        .mount("/assets", FileServer::from("assets"))
        .attach(PidFileFairing)
}
