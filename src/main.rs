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

use rocket::request::{Form, FlashMessage};
use rocket::response::{Responder, Redirect, Flash};
use rocket::http::{Cookies, Cookie};
use models::MoreInterestingConn;
use models::User;
use models::UserAuth;
use rocket::http::Status;
use rocket_contrib::serve::StaticFiles;
use rocket_contrib::templates::{Template, handlebars};
use serde::Serialize;
use std::borrow::Cow;
use crate::models::{PostInfo, NewStar, NewUser, CommentInfo, NewPost, NewComment, NewStarComment};
use base32::Base32;
use rocket::Config;
use url::Url;
use std::collections::HashMap;
use v_htmlescape::escape;
use handlebars::{Helper, Handlebars, Context, RenderContext, Output, HelperResult};

#[derive(Serialize, Default)]
struct TemplateContext {
    title: Cow<'static, str>,
    posts: Vec<PostInfo>,
    starred_by: Vec<String>,
    comments: Vec<CommentInfo>,
    username: String,
    parent: &'static str,
    alert: Option<String>,
    invite_token: Option<Base32>,
    raw_html: String,
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
            Some(b) if b == Base32::zero() => Ok(Redirect::to(uri!(index))),
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
* `http://example.instance/-add-star` is an internal URL.

The leading hyphen on a lot of these URLs is there to distinguish between reserved words and
potential post IDs.
*/

#[derive(FromForm)]
struct AddStarForm {
    post: Base32,
}

#[post("/-add-star?<redirect..>", data = "<post>")]
fn add_star(conn: MoreInterestingConn, user: User, redirect: Form<MaybeRedirect>, post: Form<AddStarForm>) -> Result<impl Responder<'static>, Status> {
    let post = conn.get_post_info_by_uuid(user.id, post.post).map_err(|_| Status::NotFound)?;
    conn.add_star(&NewStar {
        user_id: user.id,
        post_id: post.id
    });
    redirect.maybe_redirect()
}

#[derive(FromForm)]
struct RmStarForm {
    post: Base32,
}

#[post("/-rm-star?<redirect..>", data = "<post>")]
fn rm_star(conn: MoreInterestingConn, user: User, redirect: Form<MaybeRedirect>, post: Form<RmStarForm>) -> Result<impl Responder<'static>, Status> {
    let post = conn.get_post_info_by_uuid(user.id, post.post).map_err(|_| Status::NotFound)?;
    conn.rm_star(&NewStar {
        user_id: user.id,
        post_id: post.id
    });
    redirect.maybe_redirect()
}

#[derive(FromForm)]
struct AddStarCommentForm {
    comment: i32,
}

#[post("/-add-star-comment?<redirect..>", data = "<comment>")]
fn add_star_comment(conn: MoreInterestingConn, user: User, redirect: Form<MaybeRedirect>, comment: Form<AddStarCommentForm>) -> Result<impl Responder<'static>, Status> {
    conn.add_star_comment(&NewStarComment {
        user_id: user.id,
        comment_id: comment.comment,
    });
    redirect.maybe_redirect()
}

#[derive(FromForm)]
struct RmStarCommentForm {
    comment: i32,
}

#[post("/-rm-star-comment?<redirect..>", data = "<comment>")]
fn rm_star_comment(conn: MoreInterestingConn, user: User, redirect: Form<MaybeRedirect>, comment: Form<RmStarCommentForm>) -> Result<impl Responder<'static>, Status> {
    conn.rm_star_comment(&NewStarComment {
        user_id: user.id,
        comment_id: comment.comment,
    });
    redirect.maybe_redirect()
}

#[get("/")]
fn index(conn: MoreInterestingConn, user: Option<User>, flash: Option<FlashMessage>) -> impl Responder<'static> {
    let (username, user_id) = user.map(|u| (u.username, u.id)).unwrap_or((String::new(), 0));
    Template::render("index", &TemplateContext {
        title: Cow::Borrowed("home"),
        posts: conn.get_recent_posts_with_starred_bit(user_id).expect("getting hot posts should always work"),
        parent: "layout",
        alert: flash.map(|f| f.msg().to_owned()),
        username,
        ..default()
    })
}

#[get("/-submit")]
fn create_form(user: User) -> impl Responder<'static> {
    Template::render("submit", &TemplateContext {
        title: Cow::Borrowed("submit"),
        username: user.username,
        parent: "layout",
        ..default()
    })
}

#[derive(FromForm)]
struct NewPostForm {
    title: String,
    url: Option<String>,
}

#[post("/-submit", data = "<post>")]
fn create(user: User, conn: MoreInterestingConn, post: Form<NewPostForm>) -> Result<impl Responder<'static>, Status> {
    let NewPostForm { title, url } = &*post;
    match conn.create_post(&NewPost {
        title: &title,
        url: url.as_ref().map(|s| &s[..]),
        submitted_by: user.id,
    }) {
        Ok(_) => Ok(Redirect::to(uri!(index))),
        Err(e) => {
            warn!("{:?}", e);
            Err(Status::InternalServerError)
        },
    }
}

#[get("/-login")]
fn login_form() -> impl Responder<'static> {
    Template::render("login", &TemplateContext {
        title: Cow::Borrowed("log in"),
        parent: "layout",
        ..default()
    })
}

#[derive(FromForm)]
struct UserForm {
    username: String,
    password: String,
}

#[post("/-login", data = "<post>")]
fn login(conn: MoreInterestingConn, post: Form<UserForm>, mut cookies: Cookies) -> impl Responder<'static> {
    match conn.authenticate_user(&UserAuth {
        username: &post.username,
        password: &post.password,
    }) {
        Some(user) => {
            cookies.add_private(Cookie::new("user_id", user.id.to_string()));
            Flash::success(Redirect::to(uri!(index)), "Congrats, you're in!")
        },
        None => {
            Flash::error(Redirect::to(uri!(index)), "Incorrect username or password")
        },
    }
}

#[post("/-logout")]
fn logout(mut cookies: Cookies) -> impl Responder<'static> {
    if let Some(cookie) = cookies.get_private("user_id") {
        cookies.remove_private(cookie);
    }
    Redirect::to(uri!(index))
}

#[get("/<uuid>")]
fn get_comments(conn: MoreInterestingConn, user: Option<User>, uuid: Base32) -> Result<impl Responder<'static>, Status> {
    let (username, user_id) = user.map(|u| (u.username, u.id)).unwrap_or((String::new(), 0));
    // username != "" should indicate that the user is logged in.
    // user_id == 0 should indicate that the user is not logged in.
    //
    // | user_id == 0      | username != ""    | xor |
    // |-------------------|-------------------|-----|
    // | t (not logged in) | t (logged in)     | f   |
    // | f (logged in)     | t (logged in)     | t   |
    // | t (not logged in) | f (not logged in) | t   |
    // | f (logged in)     | f (not logged in) | f   |
    //
    // Make sure that these two values are consistent.
    assert!((user_id == 0) ^ (username != ""));
    if let Ok(post_info) = conn.get_post_info_by_uuid(user_id, uuid) {
        let comments = conn.get_comments_from_post(post_info.id, user_id).unwrap_or_else(|e| {
            warn!("Failed to get comments: {:?}", e);
            Vec::new()
        });
        let post_id = post_info.id;
        Ok(Template::render("comments", &TemplateContext {
            title: Cow::Borrowed("home"),
            posts: vec![post_info],
            parent: "layout",
            starred_by: conn.get_post_starred_by(post_id).unwrap_or(Vec::new()),
            comments, username,
            ..default()
        }))
    } else if conn.check_invite_token_exists(uuid) && user_id == 0 {
        Ok(Template::render("signup", &TemplateContext {
            title: Cow::Borrowed("signup"),
            parent: "layout",
            invite_token: Some(uuid),
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

#[post("/-comment", data = "<comment>")]
fn post_comment(conn: MoreInterestingConn, user: User, comment: Form<CommentForm>) -> Option<impl Responder<'static>> {
    let post_info = conn.get_post_info_by_uuid(user.id, comment.post).into_option()?;
    conn.comment_on_post(NewComment {
        post_id: post_info.id,
        text: &comment.text,
        created_by: user.id,
    }).into_option()?;
    Some(Redirect::to(uri!(get_comments: comment.post)))
}

#[get("/-setup")]
fn setup(conn: MoreInterestingConn) -> impl Responder<'static> {
    if !conn.has_users().unwrap_or(true) {
        let config = Config::active().unwrap_or_else(|_| Config::development());
        let username = config.get_str("init_username").ok();
        let password = config.get_str("init_password").ok();
        if let (Some(username), Some(password)) = (username, password) {
            conn.register_user(&NewUser {
                username: &username[..],
                password: &password[..],
                invited_by: None,
            }).expect("registering the initial user should always succeed");
        }
    }
    Flash::success(Redirect::to(uri!(login_form)), format!("Ready."))
}

#[derive(FromForm)]
struct ConsumeInviteForm {
    username: String,
    password: String,
    invite_token: Base32,
}

#[post("/-consume-invite", data = "<form>")]
fn consume_invite(conn: MoreInterestingConn, form: Form<ConsumeInviteForm>, mut cookies: Cookies) -> Result<impl Responder<'static>, Status> {
    if let Ok(invite_token) = conn.consume_invite_token(form.invite_token) {
        if let Ok(user) = conn.register_user(&NewUser {
            username: &form.username,
            password: &form.password,
            invited_by: Some(invite_token.invited_by),
        }) {
            cookies.add_private(Cookie::new("user_id", user.id.to_string()));
            return Ok(Flash::success(Redirect::to(uri!(index)), "Congrats, you're in!"));
        }
    }
    Err(Status::BadRequest)
}

#[get("/-settings")]
fn get_settings(_conn: MoreInterestingConn, user: User, flash: Option<FlashMessage>) -> impl Responder<'static> {
    Template::render("settings", &TemplateContext {
        title: Cow::Borrowed("settings"),
        parent: "layout",
        username: user.username,
        alert: flash.map(|f| f.msg().to_owned()),
        ..default()
    })
}

#[post("/-create-invite")]
fn create_invite<'a>(conn: MoreInterestingConn, user: User) -> impl Responder<'static> {
    match conn.create_invite_token(user.id) {
        Ok(invite_token) => {
            let config = Config::active().unwrap_or_else(|_| Config::development());
            let public_url = Url::parse(config.get_str("public_url").unwrap_or("http://localhost")).expect("public_url configuration must be valid");
            let created_invite_url = public_url.join(&invite_token.uuid.to_string()).expect("base128 is a valid relative URL");
            Flash::success(Redirect::to(uri!(get_settings)), format!("To invite them, send them this link: {}", created_invite_url))
        }
        Err(e) => {
            dbg!(e);
            Flash::error(Redirect::to(uri!(get_settings)), "Failed to create invite")
        }
    }
}

#[get("/@")]
fn invite_tree(conn: MoreInterestingConn, user: Option<User>) -> impl Responder<'static> {
    let (username, user_id) = user.map(|u| (u.username, u.id)).unwrap_or((String::new(), 0));
    assert!((user_id == 0) ^ (username != ""));
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
    handle_invite_tree(&mut raw_html, &conn.get_invite_tree(), 0);
    raw_html.push_str("</ul>");
    Template::render("users", &TemplateContext {
        title: Cow::Borrowed("user invite tree"),
        parent: "layout",
        username, raw_html,
        ..default()
    })
}


#[derive(FromForm)]
struct ChangePasswordForm {
    old_password: String,
    new_password: String,
}

#[post("/-change-password", data = "<form>")]
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

fn main() {
    //env_logger::init();
    rocket::ignite()
        .attach(MoreInterestingConn::fairing())
        .mount("/", routes![index, login_form, login, logout, create_form, create, setup, get_comments, add_star, rm_star, consume_invite, get_settings, create_invite, invite_tree, change_password, post_comment, add_star_comment, rm_star_comment])
        .mount("/-assets", StaticFiles::from("assets"))
        .attach(Template::custom(|engines| {
            engines.handlebars.register_helper("count", Box::new(count_helper));
        }))
        .launch();
}
