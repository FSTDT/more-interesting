#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate rocket_contrib;
#[macro_use]
extern crate log;

mod schema;
mod models;
mod password;
mod session;
mod base32;
mod prettify;
mod pid_file_fairing;
mod extract_excerpt;
mod sql_types;

use rocket::request::{Form, FlashMessage, FromParam};
use rocket::response::{Responder, Redirect, Flash};
use rocket::http::{Cookies, Cookie, RawStr};
pub use models::MoreInterestingConn;
use models::User;
use models::UserAuth;
use rocket::http::Status;
use rocket_contrib::serve::StaticFiles;
use rocket_contrib::templates::{Template, handlebars};
use serde::Serialize;
use std::borrow::Cow;
use crate::models::{PostInfo, NewStar, NewUser, CommentInfo, NewPost, NewComment, NewStarComment, NewTag, Tag, Comment, ModerationInfo, NewFlag, NewFlagComment};
use base32::Base32;
use url::Url;
use std::collections::HashMap;
use v_htmlescape::escape;
use handlebars::{Helper, Handlebars, Context, RenderContext, Output, HelperResult};
use crate::pid_file_fairing::PidFileFairing;
use rocket::fairing;
use rocket::State;
use session::Moderator;
use std::mem::replace;

#[derive(Clone, Serialize)]
struct SiteConfig {
    enable_user_directory: bool,
    enable_anonymous_submissions: bool,
    enable_public_signup: bool,
    #[serde(with = "url_serde")]
    public_url: Url,
    custom_css: String,
    site_title_html: String,
    custom_footer_html: String,
    custom_header_html: String,
    hide_text_post: bool,
    hide_link_post: bool,
}

impl Default for SiteConfig {
    fn default() -> Self {
        SiteConfig {
            enable_user_directory: false,
            enable_anonymous_submissions: false,
            enable_public_signup: false,
            public_url: Url::parse("http://localhost").unwrap(),
            site_title_html: String::new(),
            custom_footer_html: String::new(),
            custom_header_html: String::new(),
            custom_css: String::new(),
            hide_text_post: false,
            hide_link_post: false,
        }
    }
}

#[derive(Serialize, Default)]
struct TemplateContext {
    title: Cow<'static, str>,
    posts: Vec<PostInfo>,
    starred_by: Vec<String>,
    comments: Vec<CommentInfo>,
    comment: Option<Comment>,
    user: User,
    parent: &'static str,
    alert: Option<String>,
    invite_token: Option<Base32>,
    raw_html: String,
    tags: Vec<Tag>,
    config: SiteConfig,
    log: Vec<ModerationInfo>,
    is_home: bool,
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
    pub fn maybe_redirect(self) -> Result<impl Responder<'static>, Status> {
        match self.redirect {
            Some(b) if b == Base32::zero() => Ok(Redirect::to("/")),
            Some(b) => Ok(Redirect::to(uri!(get_comments: b))),
            None => Err(Status::Created)
        }
    }
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
}

#[post("/vote?<redirect..>", data = "<post>")]
fn vote(conn: MoreInterestingConn, user: User, redirect: Form<MaybeRedirect>, post: Form<VoteForm>) -> Result<impl Responder<'static>, Status> {
    let result = match (post.add_star, post.rm_star, post.add_flag, post.rm_flag) {
        (Some(post), None, None, None) => {
            let post = conn.get_post_info_by_uuid(user.id, post).map_err(|_| Status::NotFound)?;
            conn.add_star(&NewStar {
                user_id: user.id,
                post_id: post.id,
            })
        }
        (None, Some(post), None, None) => {
            let post = conn.get_post_info_by_uuid(user.id, post).map_err(|_| Status::NotFound)?;
            conn.rm_star(&NewStar {
                user_id: user.id,
                post_id: post.id,
            })
        }
        (None, None, Some(post), None) => {
            let post = conn.get_post_info_by_uuid(user.id, post).map_err(|_| Status::NotFound)?;
            conn.add_flag(&NewFlag {
                user_id: user.id,
                post_id: post.id,
            })
        }
        (None, None, None, Some(post)) => {
            let post = conn.get_post_info_by_uuid(user.id, post).map_err(|_| Status::NotFound)?;
            conn.rm_flag(&NewFlag {
                user_id: user.id,
                post_id: post.id,
            })
        }
        _ => false,
    };
    if result {
        use chrono::{Utc, Duration};
        if user.trust_level == 0 &&
            (Utc::now().naive_utc() - user.created_at) > Duration::seconds(60 * 60 * 24) &&
            conn.user_has_received_a_star(user.id) {
            conn.change_user_trust_level(user.id, 1).expect("if voting works, then so should switching trust level")
        }
        redirect.maybe_redirect()
    } else {
        Err(Status::BadRequest)
    }
}

#[derive(FromForm)]
struct VoteCommentForm {
    add_star_comment: Option<i32>,
    rm_star_comment: Option<i32>,
    add_flag_comment: Option<i32>,
    rm_flag_comment: Option<i32>,
}

#[post("/vote-comment?<redirect..>", data = "<comment>")]
fn vote_comment(conn: MoreInterestingConn, user: User, redirect: Form<MaybeRedirect>, comment: Form<VoteCommentForm>) -> Result<impl Responder<'static>, Status> {
    let result = match (comment.add_star_comment, comment.rm_star_comment, comment.add_flag_comment, comment.rm_flag_comment) {
        (Some(comment), None, None, None) => conn.add_star_comment(&NewStarComment{
            user_id: user.id,
            comment_id: comment,
        }),
        (None, Some(comment), None, None) => conn.rm_star_comment(&NewStarComment{
            user_id: user.id,
            comment_id: comment,
        }),
        (None, None, Some(comment), None) if user.trust_level >= 1 => conn.add_flag_comment(&NewFlagComment{
            user_id: user.id,
            comment_id: comment,
        }),
        (None, None, None, Some(comment)) => conn.rm_flag_comment(&NewFlagComment{
            user_id: user.id,
            comment_id: comment,
        }),
        _ => false,
    };
    if result {
        use chrono::{Utc, Duration};
        if user.trust_level == 0 &&
            (Utc::now().naive_utc() - user.created_at) > Duration::seconds(60 * 60 * 24) &&
            conn.user_has_received_a_star(user.id) {
            conn.change_user_trust_level(user.id, 1).expect("if voting works, then so should switching trust level")
        }
        redirect.maybe_redirect()
    } else {
        Err(Status::BadRequest)
    }
}

#[derive(FromForm)]
struct IndexParams {
    tag: Option<String>,
    domain: Option<String>,
    q: Option<String>,
}

#[get("/?<params..>")]
fn index(conn: MoreInterestingConn, user: Option<User>, flash: Option<FlashMessage>, params: Option<Form<IndexParams>>, config: State<SiteConfig>) -> Option<impl Responder<'static>> {
    let user = user.unwrap_or(User::default());
    let title;
    let posts = if let Some(tag_name) = params.as_ref().and_then(|params| params.tag.as_ref()) {
        if let Ok(recent) = conn.get_tag_by_name(tag_name)
            .and_then(|tag| conn.get_post_info_recent_by_tag(user.id, tag.id)) {
            title = Cow::Owned(tag_name.clone());
            recent
        } else {
            return None;
        }
    } else if let Some(query) = params.as_ref().and_then(|params| params.q.as_ref()) {
        if let Ok(recent) = conn.get_post_info_search(user.id, query) {
            title = Cow::Owned(query.clone());
            recent
        } else {
            return None;
        }
    } else if let Some(domain_name) = params.as_ref().and_then(|params| params.domain.as_ref()) {
        let domain = conn.get_domain_by_hostname(domain_name);
        if let Ok(domain) = domain {
            if let Ok(recent) = conn.get_post_info_recent_by_domain(user.id, domain.id) {
                title = Cow::Owned(domain_name.clone());
                recent
            } else {
                return None;
            }
        } else {
            return None;
        }
    } else {
        if let Ok(recent) = conn.get_post_info_recent(user.id) {
            title = Cow::Borrowed("home");
            recent
        } else {
            return None;
        }
    };
    Some(Template::render("index", &TemplateContext {
        parent: "layout",
        alert: flash.map(|f| f.msg().to_owned()),
        config: config.clone(),
        is_home: title == "home",
        title, user, posts,
        ..default()
    }))
}

#[get("/latest")]
fn latest(conn: MoreInterestingConn, user: Option<User>, flash: Option<FlashMessage>, config: State<SiteConfig>) -> Option<impl Responder<'static>> {
    let user = user.unwrap_or(User::default());
    let posts = conn.get_post_info_latest(user.id).ok()?;
    Some(Template::render("index", &TemplateContext {
        title: Cow::Borrowed("latest"),
        parent: "layout",
        alert: flash.map(|f| f.msg().to_owned()),
        config: config.clone(),
        user, posts,
        ..default()
    }))
}

#[derive(FromForm)]
struct ModLogParams {
    after: Option<i32>,
}

#[get("/mod-log?<params..>")]
fn mod_log(conn: MoreInterestingConn, user: Option<User>, flash: Option<FlashMessage>, params: Option<Form<ModLogParams>>, config: State<SiteConfig>) -> impl Responder<'static> {
    let user = user.unwrap_or(User::default());
    let log = if let Some(after) = params.as_ref().and_then(|params| params.after) {
        conn.get_mod_log_starting_with(after)
    } else {
        conn.get_mod_log_recent()
    }.unwrap_or(Vec::new());
    Template::render("mod-log", &TemplateContext {
        title: Cow::Borrowed("moderation"),
        parent: "layout",
        alert: flash.map(|f| f.msg().to_owned()),
        config: config.clone(),
        user, log,
        ..default()
    })
}

#[get("/post")]
fn create_post_form(user: Option<User>, config: State<SiteConfig>) -> impl Responder<'static> {
    Template::render("post", &TemplateContext {
        title: Cow::Borrowed("post"),
        parent: "layout",
        config: config.clone(),
        user: user.unwrap_or(User::default()),
        ..default()
    })
}
#[get("/submit")]
fn create_link_form(user: Option<User>, config: State<SiteConfig>) -> impl Responder<'static> {
    Template::render("submit", &TemplateContext {
        title: Cow::Borrowed("submit"),
        parent: "layout",
        config: config.clone(),
        user: user.unwrap_or(User::default()),
        ..default()
    })
}

#[derive(FromForm)]
struct NewPostForm {
    title: String,
    url: Option<String>,
    excerpt: Option<String>,
}

#[post("/submit", data = "<post>")]
fn create(user: Option<User>, conn: MoreInterestingConn, post: Form<NewPostForm>, config: State<SiteConfig>) -> Result<impl Responder<'static>, Status> {
    let user = if let Some(user) = user {
        if user.trust_level < 0 {
            return Err(Status::InternalServerError);
        }
        user
    } else if config.enable_anonymous_submissions {
        conn.get_user_by_username("anonymous").or_else(|_| {
            let p: [char; 16] = rand::random();
            let mut password = String::new();
            password.extend(p.iter());
            let user = conn.register_user(&NewUser{
                username: "anonymous",
                password: &password,
                invited_by: None,
            })?;
            conn.change_user_trust_level(user.id, -1)?;
            Ok(user)
        }).map_err(|_: diesel::result::Error| Status::InternalServerError)?
    } else {
        return Err(Status::BadRequest);
    };
    let NewPostForm { title, url, excerpt } = &*post;
    let mut title = &title[..];
    let mut url = url.as_ref().and_then(|u| {
        if u == "" {
            None
        } else if !u.contains(":") && !u.starts_with("//") {
            Some(format!("https://{}", u))
        } else {
            Some(u.to_owned())
        }
    });
    let e;
    let mut excerpt = excerpt.as_ref().and_then(|k| if k == "" { None } else { Some(&k[..]) });
    if let (None, Some(url_)) = (excerpt, &url) {
        if let Ok(url_) = Url::parse(url_) {
            e = extract_excerpt::extract_excerpt_url(url_);
            if let Some(ref e) = e {
                if e.body != "" {
                    excerpt = Some(&e.body);
                }
                if title == "" {
                    title = &e.title;
                }
                if e.url.as_str() != url.as_ref().map(|s| &s[..]).unwrap_or("") {
                    url = Some(e.url.to_string());
                }
            }
        }
    }
    match conn.create_post(&NewPost {
        url: url.as_ref().map(|s| &s[..]),
        submitted_by: user.id,
        visible: user.trust_level > 0,
        title, excerpt
    }) {
        Ok(post) => Ok(Redirect::to(post.uuid.to_string())),
        Err(e) => {
            warn!("{:?}", e);
            Err(Status::InternalServerError)
        },
    }
}

#[get("/login")]
fn login_form(config: State<SiteConfig>) -> impl Responder<'static> {
    Template::render("login", &TemplateContext {
        title: Cow::Borrowed("log in"),
        parent: "layout",
        config: config.clone(),
        ..default()
    })
}

#[derive(FromForm)]
struct UserForm {
    username: String,
    password: String,
}

#[post("/login", data = "<post>")]
fn login(conn: MoreInterestingConn, post: Form<UserForm>, mut cookies: Cookies) -> impl Responder<'static> {
    match conn.authenticate_user(&UserAuth {
        username: &post.username,
        password: &post.password,
    }) {
        Some(user) => {
            if user.trust_level < 0 {
                return Flash::error(Redirect::to("/"), "Sorry. Not sorry. You're banned.");
            }
            cookies.add_private(Cookie::new("user_id", user.id.to_string()));
            Flash::success(Redirect::to("/"), "Congrats, you're in!")
        },
        None => {
            Flash::error(Redirect::to("/"), "Incorrect username or password")
        },
    }
}

#[post("/logout")]
fn logout(mut cookies: Cookies) -> impl Responder<'static> {
    if let Some(cookie) = cookies.get_private("user_id") {
        cookies.remove_private(cookie);
    }
    Redirect::to("/")
}

struct UserInfoName(String);

impl<'a> FromParam<'a> for UserInfoName {
    type Error = &'a RawStr;

    fn from_param(param: &'a RawStr) -> Result<UserInfoName, &'a RawStr> {
        match param.starts_with("@") {
            true => Ok(UserInfoName(param[1..].to_string())),
            false => Err(param)
        }
    }
}

#[get("/<username>")]
fn get_user_info(conn: MoreInterestingConn, user: Option<User>, username: UserInfoName, config: State<SiteConfig>, flash: Option<FlashMessage>) -> Result<impl Responder<'static>, Status> {
    let user = user.unwrap_or(User::default());
    let user_info = if let Ok(user_info) = conn.get_user_by_username(&username.0) {
        user_info
    } else {
        return Err(Status::NotFound);
    };
    let posts = if let Ok(posts) = conn.get_post_info_recent_by_user(user.id, user_info.id) {
        posts
    } else {
        return Err(Status::InternalServerError);
    };
    Ok(Template::render("index", &TemplateContext {
        title: Cow::Owned(username.0),
        parent: "layout",
        alert: flash.map(|f| f.msg().to_owned()),
        config: config.clone(),
        posts, user,
        ..default()
    }))
}

#[get("/<uuid>", rank = 1)]
fn get_comments(conn: MoreInterestingConn, user: Option<User>, uuid: Base32, config: State<SiteConfig>, flash: Option<FlashMessage>) -> Result<impl Responder<'static>, Status> {
    let user = user.unwrap_or(User::default());
    if let Ok(post_info) = conn.get_post_info_by_uuid(user.id, uuid) {
        let comments = conn.get_comments_from_post(post_info.id, user.id).unwrap_or_else(|e| {
            warn!("Failed to get comments: {:?}", e);
            Vec::new()
        });
        let post_id = post_info.id;
        let title = Cow::Owned(post_info.title.clone());
        Ok(Template::render("comments", &TemplateContext {
            posts: vec![post_info],
            parent: "layout",
            alert: flash.map(|f| f.msg().to_owned()),
            starred_by: conn.get_post_starred_by(post_id).unwrap_or(Vec::new()),
            config: config.clone(),
            comments, user, title,
            ..default()
        }))
    } else if conn.check_invite_token_exists(uuid) && user.id == 0 {
        Ok(Template::render("signup", &TemplateContext {
            title: Cow::Borrowed("signup"),
            parent: "layout",
            invite_token: Some(uuid),
            config: config.clone(),
            ..default()
        }))
    } else {
        Err(Status::NotFound)
    }
}

#[derive(FromForm)]
struct CommentForm {
    text: String,
    post: Base32,
}

#[post("/comment", data = "<comment>")]
fn post_comment(conn: MoreInterestingConn, user: User, comment: Form<CommentForm>) -> Option<impl Responder<'static>> {
    let post_info = conn.get_post_info_by_uuid(user.id, comment.post).into_option()?;
    let visible = user.trust_level > 0;
    conn.comment_on_post(NewComment {
        post_id: post_info.id,
        text: &comment.text,
        created_by: user.id,
        visible,
    }).into_option()?;
    Some(Flash::success(
        Redirect::to(uri!(get_comments: comment.post)),
        if visible { "Your comment has been posted" } else { "Your comment will be posted after a mod gets a chance to look at it" }
    ))
}

#[derive(FromForm)]
struct SignupForm {
    username: String,
    password: String,
    invite_token: Option<Base32>,
}

#[post("/signup", data = "<form>")]
fn signup(conn: MoreInterestingConn, form: Form<SignupForm>, mut cookies: Cookies, config: State<SiteConfig>) -> Result<impl Responder<'static>, Status> {
    if form.username == "" || form.username == "anonymous" {
        return Err(Status::BadRequest);
    }
    let invited_by = if let Some(invite_token) = form.invite_token {
        if let Ok(invite_token) = conn.consume_invite_token(invite_token) {
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
    if let Ok(user) = conn.register_user(&NewUser {
        username: &form.username,
        password: &form.password,
        invited_by,
    }) {
        if invited_by.as_ref().and_then(|user_id| conn.get_user_by_id(*user_id).ok()).map(|user| user.trust_level).unwrap_or(-1) >= 2 {
            conn.change_user_trust_level(user.id, 1).expect("if logging in worked, then so should changing trust level");
        }
        cookies.add_private(Cookie::new("user_id", user.id.to_string()));
        return Ok(Flash::success(Redirect::to("/"), "Congrats, you're in!"));
    }
    Err(Status::BadRequest)
}

#[get("/signup")]
fn get_public_signup(flash: Option<FlashMessage>, config: State<SiteConfig>) -> Result<impl Responder<'static>, Status> {
    if !config.enable_public_signup {
        return Err(Status::NotFound);
    }
    Ok(Template::render("signup", &TemplateContext {
        title: Cow::Borrowed("sign up"),
        parent: "layout",
        alert: flash.map(|f| f.msg().to_owned()),
        config: config.clone(),
        ..default()
    }))
}

#[get("/settings")]
fn get_settings(_conn: MoreInterestingConn, user: User, flash: Option<FlashMessage>, config: State<SiteConfig>) -> impl Responder<'static> {
    Template::render("settings", &TemplateContext {
        title: Cow::Borrowed("settings"),
        parent: "layout",
        alert: flash.map(|f| f.msg().to_owned()),
        config: config.clone(),
        user,
        ..default()
    })
}

#[derive(FromForm)]
struct DarkModeForm {
    active: bool,
}

#[post("/set-dark-mode", data="<form>")]
fn set_dark_mode<'a>(conn: MoreInterestingConn, user: User, form: Form<DarkModeForm>) -> impl Responder<'static> {
    match conn.set_dark_mode(user.id, form.active) {
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
fn set_big_mode<'a>(conn: MoreInterestingConn, user: User, form: Form<DarkModeForm>) -> impl Responder<'static> {
    match conn.set_big_mode(user.id, form.active) {
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
fn create_invite<'a>(conn: MoreInterestingConn, user: User, config: State<SiteConfig>) -> impl Responder<'static> {
    match conn.create_invite_token(user.id) {
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

#[get("/tags")]
fn get_tags(conn: MoreInterestingConn, user: Option<User>, config: State<SiteConfig>) -> impl Responder<'static> {
    let user = user.unwrap_or(User::default());
    assert!((user.id == 0) ^ (user.username != ""));
    let tags = conn.get_all_tags().unwrap_or(Vec::new());
    Template::render("tags", &TemplateContext {
        title: Cow::Borrowed("all tags"),
        parent: "layout",
        config: config.clone(),
        tags, user,
        ..default()
    })
}

#[get("/@")]
fn invite_tree(conn: MoreInterestingConn, user: Option<User>, config: State<SiteConfig>) -> impl Responder<'static> {
    let user = user.unwrap_or(User::default());
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
        handle_invite_tree(&mut raw_html, &conn.get_invite_tree(), 0);
    }
    raw_html.push_str("</ul>");
    Template::render("users", &TemplateContext {
        title: Cow::Borrowed("user invite tree"),
        parent: "layout",
        config: config.clone(),
        user, raw_html,
        ..default()
    })
}

#[get("/edit-tags")]
fn get_edit_tags(_conn: MoreInterestingConn, user: Moderator, flash: Option<FlashMessage>, config: State<SiteConfig>) -> impl Responder<'static> {
    Template::render("edit-tags", &TemplateContext {
        title: Cow::Borrowed("add or edit tags"),
        parent: "layout",
        user: user.0,
        alert: flash.map(|f| f.msg().to_owned()),
        config: config.clone(),
        ..default()
    })
}

#[derive(FromForm)]
struct EditTagsForm {
    name: String,
    description: Option<String>,
}

#[post("/edit-tags", data = "<form>")]
fn edit_tags(conn: MoreInterestingConn, _user: Moderator, form: Form<EditTagsForm>) -> impl Responder<'static> {
    match conn.create_or_update_tag(&NewTag {
        name: &form.name,
        description: form.description.as_ref().map(|d| &d[..])
    }) {
        Ok(_) => {
            Flash::success(Redirect::to(uri!(get_edit_tags)), "Updated site tags")
        }
        Err(e) => {
            debug!("Unable to update site tags: {:?}", e);
            Flash::error(Redirect::to(uri!(get_edit_tags)), "Unable to update site tags")
        }
    }
}

#[derive(FromForm)]
struct GetEditPost {
    post: Base32,
}

#[get("/edit-post?<post..>")]
fn get_edit_post(conn: MoreInterestingConn, user: Moderator, flash: Option<FlashMessage>, post: Form<GetEditPost>, config: State<SiteConfig>) -> Option<impl Responder<'static>> {
    let post_info = conn.get_post_info_by_uuid(user.0.id, post.post).ok()?;
    Some(Template::render("edit-post", &TemplateContext {
        title: Cow::Borrowed("edit post"),
        parent: "layout",
        user: user.0,
        alert: flash.map(|f| f.msg().to_owned()),
        config: config.clone(),
        posts: vec![post_info],
        ..default()
    }))
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
fn edit_post(conn: MoreInterestingConn, user: Moderator, form: Form<EditPostForm>) -> Result<impl Responder<'static>, Status> {
    let post_info = if let Ok(post_info) = conn.get_post_info_by_uuid(user.0.id, form.post) {
        post_info
    } else {
        return Err(Status::NotFound);
    };
    let post_id = post_info.id;
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
        match conn.delete_post(post_id) {
            Ok(_) => {
                conn.mod_log_delete_post(
                    user.0.id,
                    post_info.uuid,
                    &post_info.title,
                    post_info.url.as_ref().map(|x| &x[..]).unwrap_or(""),
                    post_info.excerpt.as_ref().map(|x| &x[..]).unwrap_or(""),
                ).expect("if updating the post worked, then so should logging");
                return Ok(Flash::success(Redirect::to(uri!(get_mod_queue)), "Deleted post"))
            },
            Err(e) => {
                warn!("{:?}", e);
                return Err(Status::InternalServerError)
            },
        }
    } else {
        match conn.update_post(post_id, &NewPost {
            title: &form.title,
            url: url.as_ref().map(|s| &s[..]),
            submitted_by: user.0.id,
            excerpt: excerpt.as_ref().map(|s| &s[..]),
            visible: user.0.trust_level >= 3,
        }) {
            Ok(_) => {
                conn.mod_log_edit_post(
                    user.0.id,
                    post_info.uuid,
                    &post_info.title,
                    &form.title,
                    post_info.url.as_ref().map(|x| &x[..]).unwrap_or(""),
                    url.as_ref().map(|x| &x[..]).unwrap_or(""),
                    post_info.excerpt.as_ref().map(|x| &x[..]).unwrap_or(""),
                    form.excerpt.as_ref().map(|x| &x[..]).unwrap_or(""),
                ).expect("if updating the post worked, then so should logging");
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
fn get_edit_comment(conn: MoreInterestingConn, user: User, flash: Option<FlashMessage>, comment: Form<GetEditComment>, config: State<SiteConfig>) -> Option<impl Responder<'static>> {
    let comment = conn.get_comment_by_id(comment.comment).ok()?;
    if user.trust_level < 3 && comment.created_by != user.id {
        return None;
    }
    Some(Template::render("edit-comment", &TemplateContext {
        title: Cow::Borrowed("edit comment"),
        parent: "layout",
        alert: flash.map(|f| f.msg().to_owned()),
        config: config.clone(),
        comment: Some(comment),
        user,
        ..default()
    }))
}

#[derive(FromForm)]
struct EditCommentForm {
    comment: i32,
    text: String,
    delete: bool,
}

#[post("/edit-comment", data = "<form>")]
fn edit_comment(conn: MoreInterestingConn, user: User, form: Form<EditCommentForm>) -> Result<impl Responder<'static>, Status> {
    let comment = conn.get_comment_by_id(form.comment).map_err(|_| Status::NotFound)?;
    let post = conn.get_post_info_from_comment(user.id, form.comment).map_err(|_| Status::NotFound)?;
    if user.trust_level < 3 && comment.created_by != user.id {
        return Err(Status::NotFound);
    }
    if form.delete && user.trust_level >= 3 {
        match conn.delete_comment(comment.id) {
            Ok(_) => {
                conn.mod_log_delete_comment(
                    user.id,
                    comment.id,
                    post.uuid,
                    &comment.text,
                ).expect("if updating the comment worked, then so should logging");
                Ok(Flash::success(Redirect::to(uri!(get_mod_queue)), "Deleted comment"))
            },
            Err(e) => {
                warn!("{:?}", e);
                Err(Status::InternalServerError)
            },
        }
    } else {
        match conn.update_comment(post.id, form.comment, &form.text) {
            Ok(_) => {
                if user.trust_level >= 3 && comment.created_by != user.id {
                    conn.mod_log_edit_comment(
                        user.id,
                        comment.id,
                        post.uuid,
                        &comment.text,
                        &form.text,
                    ).expect("if updating the comment worked, then so should logging");
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
struct ChangePasswordForm {
    old_password: String,
    new_password: String,
}

#[post("/change-password", data = "<form>")]
fn change_password(conn: MoreInterestingConn, user: User, form: Form<ChangePasswordForm>) -> Result<impl Responder<'static>, Status> {
    if form.new_password == "" {
        return Err(Status::BadRequest);
    }
    if conn.authenticate_user(&UserAuth {
        username: &user.username,
        password: &form.old_password,
    }).is_none() {
        return Err(Status::BadRequest);
    }
    match conn.change_user_password(user.id, &form.new_password) {
        Ok(()) => Ok(Flash::success(Redirect::to(uri!(get_settings)), "Done!")),
        Err(_) => Err(Status::BadRequest),
    }
}

#[get("/mod-queue")]
fn get_mod_queue(conn: MoreInterestingConn, user: Moderator, flash: Option<FlashMessage>, config: State<SiteConfig>) -> Result<impl Responder<'static>, Status> {
    let mod_a_comment: bool = rand::random();
    if mod_a_comment {
        if let Some(comment_info) = conn.find_moderated_comment(user.0.id) {
            let post_info = conn.get_post_info_from_comment(user.0.id, comment_info.id).unwrap();
            return Ok(Template::render("mod-queue", &TemplateContext {
                title: Cow::Borrowed("moderate comment"),
                parent: "layout",
                alert: flash.map(|f| f.msg().to_owned()),
                config: config.clone(),
                comments: vec![comment_info],
                posts: vec![post_info],
                user: user.0,
                ..default()
            }))
        }
    }
    if let Some(post_info) = conn.find_moderated_post(user.0.id) {
        return Ok(Template::render("mod-queue", &TemplateContext {
            title: Cow::Borrowed("moderate post"),
            parent: "layout",
            alert: flash.map(|f| f.msg().to_owned()),
            config: config.clone(),
            posts: vec![post_info],
            user: user.0,
            ..default()
        }))
    }
    Ok(Template::render("mod-queue", &TemplateContext {
        title: Cow::Borrowed("moderator queue is empty!"),
        parent: "layout",
        alert: flash.map(|f| f.msg().to_owned()),
        config: config.clone(),
        user: user.0,
        ..default()
    }))
}

#[derive(FromForm)]
struct ModeratePostForm {
    post: Base32,
    action: String,
}

#[post("/moderate-post", data = "<form>")]
fn moderate_post(conn: MoreInterestingConn, user: Moderator, form: Form<ModeratePostForm>) -> Result<impl Responder<'static>, Status> {
    let post_info = if let Ok(post_info) = conn.get_post_info_by_uuid(user.0.id, form.post) {
        post_info
    } else {
        return Err(Status::NotFound);
    };
    let post_id = post_info.id;
    if form.action == "approve" {
        match conn.approve_post(post_id) {
            Ok(_) => {
                conn.mod_log_approve_post(
                    user.0.id,
                    post_info.uuid,
                    &post_info.title,
                    post_info.url.as_ref().map(|x| &x[..]).unwrap_or(""),
                    post_info.excerpt.as_ref().map(|x| &x[..]).unwrap_or(""),
                ).expect("if updating the post worked, then so should logging");
                Ok(Flash::success(Redirect::to(uri!(get_mod_queue)), "Approved post"))
            },
            Err(e) => {
                warn!("{:?}", e);
                Err(Status::InternalServerError)
            },
        }
    } else {
        match conn.delete_post(post_id) {
            Ok(_) => {
                conn.mod_log_delete_post(
                    user.0.id,
                    post_info.uuid,
                    &post_info.title,
                    post_info.url.as_ref().map(|x| &x[..]).unwrap_or(""),
                    post_info.excerpt.as_ref().map(|x| &x[..]).unwrap_or(""),
                ).expect("if updating the post worked, then so should logging");
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
struct ModerateCommentForm {
    comment: i32,
    action: String,
}

#[get("/rebake-all-posts")]
fn rebake_all_posts(conn: MoreInterestingConn, _user: Moderator) -> &'static str {
    std::thread::spawn(move || {
        let max = conn.maximum_post_id();
        for i in 0..=max {
            if let Ok(post) = conn.get_post_by_id(i) {
                let _ = conn.update_post(i, &NewPost {
                    title: &post.title[..],
                    url: post.url.as_ref().map(|t| &t[..]),
                    excerpt: post.excerpt.as_ref().map(|t| &t[..]),
                    submitted_by: post.submitted_by,
                    visible: post.visible
                });
            }
        }
    });
    "started"
}

#[post("/moderate-comment", data = "<form>")]
fn moderate_comment(conn: MoreInterestingConn, user: Moderator, form: Form<ModerateCommentForm>) -> Result<impl Responder<'static>, Status> {
    let comment_info = if let Ok(comment) = conn.get_comment_by_id(form.comment) {
        comment
    } else {
        return Err(Status::NotFound);
    };
    let post_info = conn.get_post_info_from_comment(user.0.id, comment_info.id).unwrap();
    if form.action == "approve" {
        match conn.approve_comment(comment_info.id) {
            Ok(_) => {
                conn.mod_log_approve_comment(
                    user.0.id,
                    comment_info.id,
                    post_info.uuid,
                    &comment_info.text,
                ).expect("if updating the comment worked, then so should logging");
                Ok(Flash::success(Redirect::to(uri!(get_mod_queue)), "Approved comment"))
            },
            Err(e) => {
                warn!("{:?}", e);
                Err(Status::InternalServerError)
            },
        }
    } else {
        match conn.delete_comment(comment_info.id) {
            Ok(_) => {
                conn.mod_log_delete_comment(
                    user.0.id,
                    comment_info.id,
                    post_info.uuid,
                    &comment_info.text,
                ).expect("if updating the comment worked, then so should logging");
                Ok(Flash::success(Redirect::to(uri!(get_mod_queue)), "Deleted comment"))
            },
            Err(e) => {
                warn!("{:?}", e);
                Err(Status::InternalServerError)
            },
        }
    }
}

#[get("/random")]
fn random(conn: MoreInterestingConn) -> Option<impl Responder<'static>> {
    let post = conn.random_post();
    if let Ok(Some(post)) = post {
        Some(Redirect::to(post.uuid.to_string()))
    } else {
        None
    }
}

#[get("/id/<id>")]
fn redirect_legacy_id(id: i64) -> impl Responder<'static> {
    Redirect::to(Base32::from(id).to_string())
}

fn count_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut Output
) -> HelperResult {
    if let Some(param) = h.param(0) {
        if let serde_json::Value::Array(param) = param.value() {
            out.write(&param.len().to_string()) ?;
        }
    }
    Ok(())
}

fn date_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut Output
) -> HelperResult {
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
    use chrono::{Utc, NaiveDateTime};
    use chrono_humanize::{Accuracy, HumanTime, Tense};
    if let Some(param) = h.param(0) {
        if let serde_json::Value::String(date) = param.value() {
            out.write("<local-date title=\"")?;
            out.write(&v_htmlescape::escape(&date).to_string())?;
            out.write("+00:00\">")?;
            if let Ok(dt) = NaiveDateTime::parse_from_str(&date, "%Y-%m-%dT%H:%M:%S%.f") {
                let h = HumanTime::from(dt - Utc::now().naive_utc());
                out.write(&v_htmlescape::escape(&h.to_text_en(Accuracy::Rough, Tense::Past)).to_string())?;
            } else {
                warn!("Invalid timestamp: {:?}", date);
                out.write(&v_htmlescape::escape(&date).to_string())?;
            }
            out.write("</local-date>")?;
        }
    }
    Ok(())
}

fn main() {
    //env_logger::init();
    rocket::ignite()
        .attach(MoreInterestingConn::fairing())
        .attach(fairing::AdHoc::on_attach("site config", |rocket| {
            let mut public_url = rocket.config().get_str("public_url").unwrap_or("http://localhost").to_owned();
            if !public_url.starts_with("http:") && !public_url.starts_with("https:") {
                public_url = "https://".to_owned() + &public_url;
            }
            let public_url = Url::parse(&public_url).expect("public_url configuration must be valid");
            let enable_user_directory = rocket.config().get_bool("enable_user_directory").unwrap_or(true);
            let enable_anonymous_submissions = rocket.config().get_bool("enable_anonymous_submissions").unwrap_or(false);
            let enable_public_signup = rocket.config().get_bool("enable_public_signup").unwrap_or(false);
            let site_title_html = rocket.config().get_str("site_title_html").unwrap_or("More Interesting").to_owned();
            let custom_css = rocket.config().get_str("custom_css").unwrap_or("").to_owned();
            let hide_text_post = rocket.config().get_bool("hide_text_post").unwrap_or(false);
            let hide_link_post = rocket.config().get_bool("hide_link_post").unwrap_or(false);
            let custom_footer_html = rocket.config().get_str("custom_footer_html").unwrap_or("").to_owned();
            let custom_header_html = rocket.config().get_str("custom_header_html").unwrap_or("").to_owned();
            Ok(rocket.manage(SiteConfig {
                enable_user_directory, public_url,
                site_title_html, custom_css,
                enable_anonymous_submissions,
                enable_public_signup,
                hide_text_post, hide_link_post,
                custom_header_html, custom_footer_html,
            }))
        }))
        .attach(fairing::AdHoc::on_attach("setup", |rocket| {
            let conn = MoreInterestingConn::get_one(&rocket).unwrap();
            if !conn.has_users().unwrap_or(true) {
                let config = rocket.config();
                let username = config.get_str("init_username").ok();
                let password = config.get_str("init_password").ok();
                if let (Some(username), Some(password)) = (username, password) {
                    let user = conn.register_user(&NewUser {
                        username: &username[..],
                        password: &password[..],
                        invited_by: None,
                    }).expect("registering the initial user should always succeed");
                    conn.change_user_trust_level(user.id, 4).expect("to make the initial user an admin");
                }
            }
            Ok(rocket)
        }))
        .mount("/", routes![index, login_form, login, logout, create_link_form, create_post_form, create, get_comments, vote, signup, get_settings, create_invite, invite_tree, change_password, post_comment, vote_comment, get_edit_tags, edit_tags, get_tags, edit_post, get_edit_post, edit_comment, get_edit_comment, set_dark_mode, set_big_mode, mod_log, get_user_info, get_mod_queue, moderate_post, moderate_comment, get_public_signup, rebake_all_posts, random, redirect_legacy_id, latest])
        .mount("/assets", StaticFiles::from("assets"))
        .attach(Template::custom(|engines| {
            engines.handlebars.register_helper("count", Box::new(count_helper));
            engines.handlebars.register_helper("date", Box::new(date_helper));
        }))
        .attach(PidFileFairing)
        .launch();
}
