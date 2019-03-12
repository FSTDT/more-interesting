use rocket_contrib::databases::diesel::PgConnection;
use diesel::prelude::*;
use diesel::result::Error as DieselError;
use chrono::NaiveDateTime;
use crate::schema::{users, posts, stars};
use crate::password::{password_hash, password_verify, PasswordResult};
use serde::Serialize;
use crate::base128::Base128;
use uuid::Uuid;

#[derive(Queryable, Serialize)]
pub struct Post {
    pub id: i32,
    pub uuid: Base128,
    pub title: String,
    pub url: Option<String>,
    pub visible: bool,
    pub score: i32,
    pub created_at: NaiveDateTime,
    pub submitted_by: i32,
}

#[derive(Queryable)]
pub struct User {
    pub id: i32,
    pub hardbanned: bool,
    pub shadowbanned: bool,
    pub username: String,
    pub password_hash: Vec<u8>,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable)]
#[table_name="posts"]
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
    pub score: i32,
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

pub struct NewUser<'a> {
    pub username: &'a str,
    pub password: &'a str,
}

#[database("more_interesting")]
pub struct MoreInterestingConn(PgConnection);

impl MoreInterestingConn {
    pub fn get_recent_posts_with_starred_bit(&self, user_id_param: i32) -> Result<Vec<PostInfo>, DieselError> {
        use self::posts::dsl::*;
        use self::stars::dsl::*;
        use self::users::dsl::*;
        // This is probably the slow way to do it.
        Ok(posts.left_outer_join(stars.on(post_id.eq(self::posts::dsl::id).and(user_id.eq(user_id_param))))
            .inner_join(users)
            .select((
                self::posts::dsl::id,
                self::posts::dsl::uuid,
                self::posts::dsl::title,
                self::posts::dsl::url,
                self::posts::dsl::visible,
                self::posts::dsl::score,
                self::posts::dsl::created_at,
                self::posts::dsl::submitted_by,
                self::stars::dsl::post_id.nullable(),
                self::users::dsl::username,
            ))
            .filter(visible.eq(true))
            .limit(50)
            .get_results::<(i32, Base128, String, Option<String>, bool, i32, NaiveDateTime, i32, Option<i32>, String)>(&self.0)?
            .into_iter()
            .map(tuple_to_post_info)
            .collect())
    }
    pub fn get_post_info_by_uuid(&self, user_id_param: i32, uuid_param: Base128) -> Result<PostInfo, DieselError> {
        use self::posts::dsl::*;
        use self::stars::dsl::*;
        use self::users::dsl::*;
        // This is probably the slow way to do it.
        // It's also a bunch of duplicate code.
        Ok(tuple_to_post_info(posts.left_outer_join(stars.on(post_id.eq(self::posts::dsl::id).and(user_id.eq(user_id_param))))
            .inner_join(users)
            .select((
                self::posts::dsl::id,
                self::posts::dsl::uuid,
                self::posts::dsl::title,
                self::posts::dsl::url,
                self::posts::dsl::visible,
                self::posts::dsl::score,
                self::posts::dsl::created_at,
                self::posts::dsl::submitted_by,
                self::stars::dsl::post_id.nullable(),
                self::users::dsl::username,
            ))
            .filter(visible.eq(true))
            .filter(uuid.eq(uuid_param.into_uuid()))
            .first::<(i32, Base128, String, Option<String>, bool, i32, NaiveDateTime, i32, Option<i32>, String)>(&self.0)?))
    }
    pub fn create_post(&self, new_post: &NewPost) -> Result<Post, DieselError> {
        diesel::insert_into(posts::table)
            .values(new_post)
            .get_result(&self.0)
    }
    pub fn add_star(&self, new_star: &NewStar) -> Result<(), DieselError> {
        diesel::insert_into(stars::table)
            .values(new_star)
            .execute(&self.0)
            .map(|i| { assert_eq!(i, 1); })
    }
    pub fn rm_star(&self, new_star: &NewStar) -> Result<(), DieselError> {
        use self::stars::dsl::*;
        diesel::delete(
            stars
                .filter(user_id.eq(new_star.user_id))
                .filter(post_id.eq(new_star.post_id))
        )
            .execute(&self.0)
            .map(|i| { assert_eq!(i, 1); })
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
    pub fn authenticate_user(&self, new_user: &NewUser) -> Option<User> {
        let mut user = self.get_user_by_username(new_user.username).ok()?;
        if password_verify(new_user.password, &mut user.password_hash[..]) == PasswordResult::Passed {
            Some(user)
        } else {
            None
        }
    }
}

fn tuple_to_post_info((id, uuid, title, url, visible, score, created_at, submitted_by, starred_post_id, submitted_by_username): (i32, Base128, String, Option<String>, bool, i32, NaiveDateTime, i32, Option<i32>, String)) -> PostInfo {
    PostInfo {
        id, uuid, title, url, visible, score, created_at, submitted_by, submitted_by_username,
        starred_by_me: starred_post_id.is_some()
    }
}
