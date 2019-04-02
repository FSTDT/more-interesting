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

use rocket::request::{Form, FlashMessage};
use rocket::response::{Responder, Redirect, Flash};
use rocket::http::{Cookies, Cookie};
pub use models::MoreInterestingConn;
use models::User;
use models::UserAuth;
use rocket::http::Status;
use rocket_contrib::serve::StaticFiles;
use rocket_contrib::templates::{Template, handlebars};
use serde::Serialize;
use std::borrow::Cow;
use crate::models::{PostInfo, NewStar, NewUser, CommentInfo, NewPost, NewComment, NewStarComment, NewTag, Tag};
use base32::Base32;
use url::Url;
use std::collections::HashMap;
use v_htmlescape::escape;
use handlebars::{Helper, Handlebars, Context, RenderContext, Output, HelperResult};
use crate::pid_file_fairing::PidFileFairing;
use rocket::fairing;
use rocket::State;
use session::SeniorUser;

#[derive(Serialize, Default)]
struct TemplateContext {
    title: Cow<'static, str>,
    posts: Vec<PostInfo>,
    starred_by: Vec<String>,
    comments: Vec<CommentInfo>,
    user: User,
    parent: &'static str,
    alert: Option<String>,
    invite_token: Option<Base32>,
    raw_html: String,
    tags: Vec<Tag>,
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
}

#[post("/vote?<redirect..>", data = "<post>")]
fn vote(conn: MoreInterestingConn, user: User, redirect: Form<MaybeRedirect>, post: Form<VoteForm>) -> Result<impl Responder<'static>, Status> {
    let result = match (post.add_star, post.rm_star) {
        (Some(post), None) => {
            let post = conn.get_post_info_by_uuid(user.id, post).map_err(|_| Status::NotFound)?;
            conn.add_star(&NewStar {
                user_id: user.id,
                post_id: post.id,
            })
        }
        (None, Some(post)) => {
            let post = conn.get_post_info_by_uuid(user.id, post).map_err(|_| Status::NotFound)?;
            conn.rm_star(&NewStar {
                user_id: user.id,
                post_id: post.id,
            })
        }
        _ => false,
    };
    if result {
        redirect.maybe_redirect()
    } else {
        Err(Status::BadRequest)
    }
}

#[derive(FromForm)]
struct VoteCommentForm {
    add_star_comment: Option<i32>,
    rm_star_comment: Option<i32>,
}

#[post("/vote-comment?<redirect..>", data = "<comment>")]
fn vote_comment(conn: MoreInterestingConn, user: User, redirect: Form<MaybeRedirect>, comment: Form<VoteCommentForm>) -> Result<impl Responder<'static>, Status> {
    let result = match (comment.add_star_comment, comment.rm_star_comment) {
        (Some(comment), None) => conn.add_star_comment(&NewStarComment{
            user_id: user.id,
            comment_id: comment,
        }),
        (None, Some(comment)) => conn.rm_star_comment(&NewStarComment{
            user_id: user.id,
            comment_id: comment,
        }),
        _ => false,
    };
    if result {
        redirect.maybe_redirect()
    } else {
        Err(Status::BadRequest)
    }
}

#[derive(FromForm)]
struct IndexParams {
    tag: Option<String>,
}

#[get("/?<params..>")]
fn index(conn: MoreInterestingConn, user: Option<User>, flash: Option<FlashMessage>, params: Option<Form<IndexParams>>) -> impl Responder<'static> {
    let user = user.unwrap_or(User::default());
    let posts = if let Some(tag_name) = params.as_ref().and_then(|params| params.tag.as_ref()) {
        conn.get_tag_by_name(tag_name)
            .and_then(|tag| conn.get_post_info_recent_by_tag(user.id, tag.id))
            .unwrap_or(Vec::new())
    }  else {
        conn.get_post_info_recent(user.id).unwrap_or(Vec::new())
    };
    Template::render("index", &TemplateContext {
        title: Cow::Borrowed("home"),
        parent: "layout",
        alert: flash.map(|f| f.msg().to_owned()),
        user, posts,
        ..default()
    })
}

#[get("/submit")]
fn create_form(user: User) -> impl Responder<'static> {
    Template::render("submit", &TemplateContext {
        title: Cow::Borrowed("submit"),
        parent: "layout",
        user,
        ..default()
    })
}

#[derive(FromForm)]
struct NewPostForm {
    title: String,
    url: Option<String>,
}

#[post("/submit", data = "<post>")]
fn create(user: User, conn: MoreInterestingConn, post: Form<NewPostForm>) -> Result<impl Responder<'static>, Status> {
    let NewPostForm { title, url } = &*post;
    let url = url.as_ref().and_then(|u| {
        if u == "" {
            None
        } else if !u.contains(":") && !u.starts_with("//") {
            Some(format!("https://{}", u))
        } else {
            Some(u.to_owned())
        }
    });
    match conn.create_post(&NewPost {
        title: &title,
        url: url.as_ref().map(|s| &s[..]),
        submitted_by: user.id,
    }) {
        Ok(_) => Ok(Redirect::to("/")),
        Err(e) => {
            warn!("{:?}", e);
            Err(Status::InternalServerError)
        },
    }
}

#[get("/login")]
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

#[post("/login", data = "<post>")]
fn login(conn: MoreInterestingConn, post: Form<UserForm>, mut cookies: Cookies) -> impl Responder<'static> {
    match conn.authenticate_user(&UserAuth {
        username: &post.username,
        password: &post.password,
    }) {
        Some(user) => {
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

#[get("/<uuid>")]
fn get_comments(conn: MoreInterestingConn, user: Option<User>, uuid: Base32) -> Result<impl Responder<'static>, Status> {
    let user = user.unwrap_or(User::default());
    if let Ok(post_info) = conn.get_post_info_by_uuid(user.id, uuid) {
        let comments = conn.get_comments_from_post(post_info.id, user.id).unwrap_or_else(|e| {
            warn!("Failed to get comments: {:?}", e);
            Vec::new()
        });
        let post_id = post_info.id;
        Ok(Template::render("comments", &TemplateContext {
            title: Cow::Borrowed("home"),
            posts: vec![post_info],
            parent: "layout",
            starred_by: conn.get_post_starred_by(post_id).unwrap_or(Vec::new()),
            comments, user,
            ..default()
        }))
    } else if conn.check_invite_token_exists(uuid) && user.id == 0 {
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

#[post("/comment", data = "<comment>")]
fn post_comment(conn: MoreInterestingConn, user: User, comment: Form<CommentForm>) -> Option<impl Responder<'static>> {
    let post_info = conn.get_post_info_by_uuid(user.id, comment.post).into_option()?;
    conn.comment_on_post(NewComment {
        post_id: post_info.id,
        text: &comment.text,
        created_by: user.id,
    }).into_option()?;
    Some(Redirect::to(uri!(get_comments: comment.post)))
}

#[derive(FromForm)]
struct ConsumeInviteForm {
    username: String,
    password: String,
    invite_token: Base32,
}

#[post("/consume-invite", data = "<form>")]
fn consume_invite(conn: MoreInterestingConn, form: Form<ConsumeInviteForm>, mut cookies: Cookies) -> Result<impl Responder<'static>, Status> {
    if let Ok(invite_token) = conn.consume_invite_token(form.invite_token) {
        if let Ok(user) = conn.register_user(&NewUser {
            username: &form.username,
            password: &form.password,
            invited_by: Some(invite_token.invited_by),
        }) {
            cookies.add_private(Cookie::new("user_id", user.id.to_string()));
            return Ok(Flash::success(Redirect::to("/"), "Congrats, you're in!"));
        }
    }
    Err(Status::BadRequest)
}

#[get("/settings")]
fn get_settings(_conn: MoreInterestingConn, user: User, flash: Option<FlashMessage>) -> impl Responder<'static> {
    Template::render("settings", &TemplateContext {
        title: Cow::Borrowed("settings"),
        parent: "layout",
        alert: flash.map(|f| f.msg().to_owned()),
        user,
        ..default()
    })
}

#[post("/create-invite")]
fn create_invite<'a>(conn: MoreInterestingConn, user: User, public_url: State<PublicUrl>) -> impl Responder<'static> {
    match conn.create_invite_token(user.id) {
        Ok(invite_token) => {
            let public_url = &public_url.0;
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
fn get_tags(conn: MoreInterestingConn, user: Option<User>) -> impl Responder<'static> {
    let user = user.unwrap_or(User::default());
    assert!((user.id == 0) ^ (user.username != ""));
    let tags = conn.get_all_tags().unwrap_or(Vec::new());
    Template::render("tags", &TemplateContext {
        title: Cow::Borrowed("user invite tree"),
        parent: "layout",
        tags, user,
        ..default()
    })
}

#[get("/@")]
fn invite_tree(conn: MoreInterestingConn, user: Option<User>) -> impl Responder<'static> {
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
    handle_invite_tree(&mut raw_html, &conn.get_invite_tree(), 0);
    raw_html.push_str("</ul>");
    Template::render("users", &TemplateContext {
        title: Cow::Borrowed("user invite tree"),
        parent: "layout",
        user, raw_html,
        ..default()
    })
}

#[get("/edit-tags")]
fn get_edit_tags(_conn: MoreInterestingConn, user: SeniorUser, flash: Option<FlashMessage>) -> impl Responder<'static> {
    Template::render("edit-tags", &TemplateContext {
        title: Cow::Borrowed("add or edit tags"),
        parent: "layout",
        user: user.0,
        alert: flash.map(|f| f.msg().to_owned()),
        ..default()
    })
}

#[derive(FromForm)]
struct EditTagsForm {
    name: String,
    description: Option<String>,
}

#[post("/edit-tags", data = "<form>")]
fn edit_tags(conn: MoreInterestingConn, _user: SeniorUser, form: Form<EditTagsForm>) -> impl Responder<'static> {
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
    use chrono::{Duration, Utc, NaiveDateTime};
    use chrono_humanize::{Accuracy, HumanTime, Tense};
    if let Some(param) = h.param(0) {
        if let serde_json::Value::String(date) = param.value() {
            out.write("<local-date title=\"")?;
            out.write(&v_htmlescape::escape(&date).to_string())?;
            out.write("+00:00\">")?;
            if let Ok(dt) = NaiveDateTime::parse_from_str(&date, "%Y-%m-%dT%H:%M:%S%.f") {
                let h = HumanTime::from(dt - Utc::now().naive_utc());
                out.write(&v_htmlescape::escape(&h).to_text_en(Accuracy::Rough, Tense::Past))?;
            } else {
                warn!("Invalid timestamp: {:?}", date);
                out.write(&v_htmlescape::escape(&date).to_string())?;
            }
            out.write("</local-date>")?;
        }
    }
    Ok(())
}

struct PublicUrl(Url);

fn main() {
    //env_logger::init();
    rocket::ignite()
        .attach(MoreInterestingConn::fairing())
        .attach(fairing::AdHoc::on_attach("public url", |rocket| {
            let mut public_url = rocket.config().get_str("public_url").unwrap_or("http://localhost").to_owned();
            if !public_url.starts_with("http:") && !public_url.starts_with("https:") {
                public_url = "https://".to_owned() + &public_url;
            }
            let public_url = Url::parse(&public_url).expect("public_url configuration must be valid");
            Ok(rocket.manage(PublicUrl(public_url)))
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
        .mount("/", routes![index, login_form, login, logout, create_form, create, get_comments, vote, consume_invite, get_settings, create_invite, invite_tree, change_password, post_comment, vote_comment, get_edit_tags, edit_tags, get_tags])
        .mount("/assets", StaticFiles::from("assets"))
        .attach(Template::custom(|engines| {
            engines.handlebars.register_helper("count", Box::new(count_helper));
            engines.handlebars.register_helper("date", Box::new(date_helper));
        }))
        .attach(PidFileFairing)
        .launch();
}
