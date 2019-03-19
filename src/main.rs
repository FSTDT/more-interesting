#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate rocket_contrib;

mod schema;
mod models;
mod password;
mod session;
mod base128;

use rocket::request::{Form, FlashMessage};
use rocket::response::{Responder, Redirect, Flash};
use rocket::http::{Cookies, Cookie};
use models::MoreInterestingConn;
use models::User;
use models::UserAuth;
use rocket::http::Status;
use rocket_contrib::serve::StaticFiles;
use rocket_contrib::templates::Template;
use serde::Serialize;
use std::borrow::Cow;
use crate::models::{PostInfo, NewStar, NewUser};
use base128::Base128;
use rocket::Config;
use url::Url;
use rocket::http::uri::Origin;
use std::collections::HashMap;
use v_htmlescape::escape;

#[derive(Serialize, Default)]
struct TemplateContext {
    title: Cow<'static, str>,
    posts: Vec<PostInfo>,
    username: String,
    parent: &'static str,
    alert: Option<String>,
    invite_token: Option<Base128>,
    raw_html: String,
}

fn default<T: Default>() -> T { T::default() }

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
    post: Base128,
}

#[post("/-add-star", data = "<post>")]
fn add_star(conn: MoreInterestingConn, user: User, post: Form<AddStarForm>) -> impl Responder<'static> {
    let post = conn.get_post_info_by_uuid(user.id, post.post).unwrap();
    conn.add_star(&NewStar {
        user_id: user.id,
        post_id: post.id
    });
    Redirect::to(uri!(index))
}

#[derive(FromForm)]
struct RmStarForm {
    post: Base128,
}

#[post("/-rm-star", data = "<post>", rank=1)]
fn rm_star(conn: MoreInterestingConn, user: User, post: Form<RmStarForm>) -> impl Responder<'static> {
    let post = conn.get_post_info_by_uuid(user.id, post.post).unwrap();
    conn.rm_star(&NewStar {
        user_id: user.id,
        post_id: post.id
    });
    Redirect::to(uri!(index))
}

#[get("/")]
fn index(conn: MoreInterestingConn, user: Option<User>, flash: Option<FlashMessage>) -> impl Responder<'static> {
    let (username, user_id) = user.map(|u| (u.username, u.id)).unwrap_or((String::new(), 0));
    Template::render("index", &TemplateContext {
        title: Cow::Borrowed("home"),
        posts: conn.get_recent_posts_with_starred_bit(user_id).unwrap(),
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
fn create(user: User, conn: MoreInterestingConn, post: Form<NewPostForm>) -> impl Responder<'static> {
    let NewPostForm { title, url } = &*post;
    match conn.create_post(&models::NewPost {
        title: &title,
        url: url.as_ref().map(|s| &s[..]),
        submitted_by: user.id,
    }) {
        Ok(_) => Redirect::to(uri!(index)),
        Err(e) => panic!("{:?}", e),
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
            Redirect::to(uri!(index))
        },
        None => Redirect::to(uri!(login)),
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
fn get_comments(conn: MoreInterestingConn, user: Option<User>, uuid: Base128) -> Result<impl Responder<'static>, Status> {
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
        Ok(Template::render("index", &TemplateContext {
            title: Cow::Borrowed("home"),
            posts: vec![post_info],
            parent: "layout",
            username,
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

#[get("/-setup")]
fn setup(conn: MoreInterestingConn) -> impl Responder<'static> {
    if conn.has_users().unwrap_or(true) {
        "setup"
    } else {
        let username = std::env::var("MORE_INTERESTING_INIT_USERNAME").ok();
        let password = std::env::var("MORE_INTERESTING_INIT_PASSWORD").ok();
        if let (Some(username), Some(password)) = (username, password) {
            conn.register_user(&NewUser {
                username: &username[..],
                password: &password[..],
                invited_by: None,
            }).unwrap();
            "setup"
        } else {
            "cannot run without MORE_INTERESTING_INIT_USERNAME and MORE_INTERESTING_INIT_PASSWORD env variables"
        }
    }
}

#[derive(FromForm)]
struct ConsumeInviteForm {
    username: String,
    password: String,
    invite_token: Base128,
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
fn get_settings(conn: MoreInterestingConn, user: User, flash: Option<FlashMessage>) -> impl Responder<'static> {
    Template::render("settings", &TemplateContext {
        title: Cow::Borrowed("settings"),
        parent: "layout",
        username: user.username,
        alert: flash.map(|f| f.msg().to_owned()),
        ..default()
    })
}

#[post("/-create-invite")]
fn create_invite<'a>(conn: MoreInterestingConn, user: User, origin: &'a Origin<'a>) -> impl Responder<'static> {
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

fn main() {
    rocket::ignite()
        .attach(MoreInterestingConn::fairing())
        .attach(Template::fairing())
        .mount("/", routes![index, login_form, login, logout, create_form, create, setup, get_comments, add_star, rm_star, consume_invite, get_settings, create_invite, invite_tree, change_password])
        .mount("/-assets", StaticFiles::from("assets"))
        .launch();
}
