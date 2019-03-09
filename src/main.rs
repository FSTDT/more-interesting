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

use rocket::request::Form;
use rocket::response::{Responder, Redirect};
use rocket::http::{Cookies, Cookie};
use models::MoreInterestingConn;
use models::User;
use models::NewUser;
use rocket_contrib::serve::StaticFiles;
use rocket_contrib::templates::Template;
use serde::Serialize;
use std::borrow::Cow;
use crate::models::PostInfo;

#[derive(Serialize)]
struct TemplateContext {
    title: Cow<'static, str>,
    posts: Vec<PostInfo>,
    username: String,
    parent: &'static str,
}

#[get("/")]
fn index(conn: MoreInterestingConn, user: Option<User>) -> impl Responder<'static> {
    let (username, user_id) = user.map(|u| (u.username, u.id)).unwrap_or((String::new(), 0));
    Template::render("index", &TemplateContext {
        title: Cow::Borrowed("home"),
        posts: conn.get_recent_posts_with_starred_bit(user_id).unwrap(),
        parent: "layout",
        username,
    })
}

#[get("/-submit")]
fn create_form(user: User) -> impl Responder<'static> {
    Template::render("submit", &TemplateContext {
        title: Cow::Borrowed("submit"),
        posts: vec![],
        username: user.username,
        parent: "layout",
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
        posts: vec![],
        username: String::new(),
        parent: "layout",
    })
}

#[derive(FromForm)]
struct UserForm {
    username: String,
    password: String,
}

#[post("/-login", data = "<post>")]
fn login(conn: MoreInterestingConn, post: Form<UserForm>, mut cookies: Cookies) -> impl Responder<'static> {
    match conn.authenticate_user(&NewUser {
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
fn get_comments(conn: MoreInterestingConn, user: Option<User>, uuid: String) -> impl Responder<'static> {
    let (username, user_id) = user.map(|u| (u.username, u.id)).unwrap_or((String::new(), 0));
    let uuid = uuid.parse().unwrap();
    Template::render("index", &TemplateContext {
        title: Cow::Borrowed("home"),
        posts: vec![conn.get_post_info_by_uuid(user_id, uuid).unwrap()],
        username: username,
        parent: "layout",
    })
}

#[get("/-setup")]
fn setup(conn: MoreInterestingConn) -> impl Responder<'static> {
    if !conn.has_no_users().unwrap_or(false) {
        "setup"
    } else {
        let username = std::env::var("MORE_INTERESTING_INIT_USERNAME").ok();
        let password = std::env::var("MORE_INTERESTING_INIT_PASSWORD").ok();
        if let (Some(username), Some(password)) = (username, password) {
            conn.register_user(&NewUser {
                username: &username[..],
                password: &password[..],
            }).unwrap();
            "setup"
        } else {
            "cannot run without MORE_INTERESTING_INIT_USERNAME and MORE_INTERESTING_INIT_PASSWORD env variables"
        }
    }
}

fn main() {
    rocket::ignite()
        .attach(MoreInterestingConn::fairing())
        .attach(Template::fairing())
        .mount("/", routes![index, login_form, login, logout, create_form, create, setup, get_comments])
        .mount("/-assets", StaticFiles::from("assets"))
        .launch();
}
