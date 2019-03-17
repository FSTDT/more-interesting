use rocket_contrib::databases::diesel::PgConnection;
use diesel::prelude::*;
use diesel::result::Error as DieselError;
use chrono::NaiveDateTime;
use crate::schema::{users, posts, stars};
use crate::password::{password_hash, password_verify, PasswordResult};
use serde::Serialize;
use crate::base128::Base128;
use uuid::Uuid;
use std::cmp::{min, max};
use ordered_float::OrderedFloat;

#[derive(Queryable, Serialize)]
pub struct Post {
    pub id: i32,
    pub uuid: Base128,
    pub title: String,
    pub url: Option<String>,
    pub visible: bool,
    pub initial_stellar_time: i32,
    pub score: i32,
    pub authored_by_submitter: bool,
    pub created_at: NaiveDateTime,
    pub submitted_by: i32,
}

#[derive(Queryable)]
pub struct User {
    pub id: i32,
    pub banned: bool,
    pub trust_level: i32,
    pub username: String,
    pub password_hash: Vec<u8>,
    pub created_at: NaiveDateTime,
}

pub struct NewPost<'a> {
    pub title: &'a str,
    pub url: Option<&'a str>,
    pub submitted_by: i32,
}

#[derive(Serialize)]
pub struct PostInfo {
    pub id: i32,
    pub uuid: Base128,
    pub title: String,
    pub url: Option<String>,
    pub visible: bool,
    pub hotness: f64,
    pub score: i32,
    pub authored_by_submitter: bool,
    pub created_at: NaiveDateTime,
    pub submitted_by: i32,
    pub submitted_by_username: String,
    pub starred_by_me: bool,
}

#[derive(Insertable)]
#[table_name="stars"]
pub struct NewStar {
    pub user_id: i32,
    pub post_id: i32,
}

pub struct UserAuth<'a> {
    pub username: &'a str,
    pub password: &'a str,
}

#[derive(Insertable)]
#[table_name="posts"]
struct CreatePost<'a> {
    title: &'a str,
    url: Option<&'a str>,
    submitted_by: i32,
    initial_stellar_time: i32,
}

#[database("more_interesting")]
pub struct MoreInterestingConn(PgConnection);

impl MoreInterestingConn {
    pub fn get_recent_posts_with_starred_bit(&self, user_id_param: i32) -> Result<Vec<PostInfo>, DieselError> {
        use self::posts::dsl::*;
        use self::stars::dsl::*;
        use self::users::dsl::*;
        let mut all: Vec<PostInfo> = posts
            .left_outer_join(stars.on(post_id.eq(self::posts::dsl::id).and(user_id.eq(user_id_param))))
            .inner_join(users)
            .select((
                self::posts::dsl::id,
                self::posts::dsl::uuid,
                self::posts::dsl::title,
                self::posts::dsl::url,
                self::posts::dsl::visible,
                self::posts::dsl::initial_stellar_time,
                self::posts::dsl::score,
                self::posts::dsl::authored_by_submitter,
                self::posts::dsl::created_at,
                self::posts::dsl::submitted_by,
                self::stars::dsl::post_id.nullable(),
                self::users::dsl::username,
            ))
            .filter(visible.eq(true))
            .order_by(initial_stellar_time)
            .limit(400)
            .get_results::<(i32, Base128, String, Option<String>, bool, i32, i32, bool, NaiveDateTime, i32, Option<i32>, String)>(&self.0)?
            .into_iter()
            .map(|t| tuple_to_post_info(t, self.get_current_stellar_time()))
            .collect();
        all.sort_by_key(|info| OrderedFloat(-info.hotness));
        Ok(all)
    }
    pub fn get_post_info_by_uuid(&self, user_id_param: i32, uuid_param: Base128) -> Result<PostInfo, DieselError> {
        use self::posts::dsl::*;
        use self::stars::dsl::*;
        use self::users::dsl::*;
        // This is a bunch of duplicate code.
        Ok(tuple_to_post_info(posts.left_outer_join(stars.on(post_id.eq(self::posts::dsl::id).and(user_id.eq(user_id_param))))
            .inner_join(users)
            .select((
                self::posts::dsl::id,
                self::posts::dsl::uuid,
                self::posts::dsl::title,
                self::posts::dsl::url,
                self::posts::dsl::visible,
                self::posts::dsl::initial_stellar_time,
                self::posts::dsl::score,
                self::posts::dsl::authored_by_submitter,
                self::posts::dsl::created_at,
                self::posts::dsl::submitted_by,
                self::stars::dsl::post_id.nullable(),
                self::users::dsl::username,
            ))
            .filter(visible.eq(true))
            .filter(uuid.eq(uuid_param.into_uuid()))
            .first::<(i32, Base128, String, Option<String>, bool, i32, i32, bool, NaiveDateTime, i32, Option<i32>, String)>(&self.0)?, self.get_current_stellar_time()))
    }
    pub fn get_current_stellar_time(&self) -> i32 {
        use self::stars::dsl::*;
        // the stars table should be limited by the i32 limits, but diesel doesn't know that
        stars.count().get_result(&self.0).unwrap_or(0) as i32
    }
    pub fn create_post(&self, new_post: &NewPost) -> Result<Post, DieselError> {
        diesel::insert_into(posts::table)
            .values(CreatePost {
                title: new_post.title,
                url: new_post.url,
                submitted_by: new_post.submitted_by,
                initial_stellar_time: self.get_current_stellar_time(),
            })
            .get_result(&self.0)
    }
    pub fn add_star(&self, new_star: &NewStar) {
        let affected_rows = diesel::insert_into(stars::table)
            .values(new_star)
            .execute(&self.0)
            .unwrap_or(0);
        // affected rows will be 1 if inserted, or 0 otherwise
        self.update_score_on_post(new_star.post_id, affected_rows as i32);
    }
    pub fn rm_star(&self, new_star: &NewStar) {
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
    }
    fn update_score_on_post(&self, post_id_value: i32, count_value: i32) {
        use self::posts::dsl::*;
        diesel::update(posts.find(post_id_value)).set(score.eq(score + count_value))
            .execute(&self.0)
            .expect("if adding a star worked, then so should updating the post");
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
    pub fn register_user(&self, new_user: &UserAuth) -> Result<User, DieselError> {
        #[derive(Insertable)]
        #[table_name="users"]
        struct NewUser<'a> {
            username: &'a str,
            password_hash: &'a [u8],
        }
        if new_user.username.as_bytes()[0] == b'-' {
            panic!("usernames may not start with hyphens");
        }
        let password_hash = password_hash(new_user.password);
        diesel::insert_into(users::table)
            .values(NewUser {
                username: new_user.username,
                password_hash: &password_hash[..],
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
}

fn tuple_to_post_info((id, uuid, title, url, visible, initial_stellar_time, score, authored_by_submitter, created_at, submitted_by, starred_post_id, submitted_by_username): (i32, Base128, String, Option<String>, bool, i32, i32, bool, NaiveDateTime, i32, Option<i32>, String), current_stellar_time: i32) -> PostInfo {
    PostInfo {
        id, uuid, title, url, visible, score, authored_by_submitter, created_at,
        submitted_by, submitted_by_username,
        starred_by_me: starred_post_id.is_some(),
        hotness: compute_hotness(initial_stellar_time, current_stellar_time, score, authored_by_submitter)
    }
}

fn compute_hotness(initial_stellar_time: i32, current_stellar_time: i32, score: i32, authored_by_submitter: bool) -> f64 {
    let gravity = 1.33;
    let boost = if authored_by_submitter { 0.33 } else { 0.0 };
    let stellar_age = max(current_stellar_time - initial_stellar_time, 0) as f64;
    (boost + (score as f64) + 1.0) / (stellar_age + 1.0).powf(gravity)
}
