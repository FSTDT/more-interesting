use rocket_contrib::databases::diesel::PgConnection;
use diesel::prelude::*;
use diesel::result::Error as DieselError;
use chrono::NaiveDateTime;
use crate::schema::{users, posts, stars, invite_tokens, comments, comment_stars};
use crate::password::{password_hash, password_verify, PasswordResult};
use serde::Serialize;
use crate::base32::Base32;
use std::cmp::max;
use ordered_float::OrderedFloat;
use std::collections::HashMap;

#[derive(Queryable, Serialize)]
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
}

#[derive(Queryable)]
pub struct User {
    pub id: i32,
    pub banned: bool,
    pub trust_level: i32,
    pub username: String,
    pub password_hash: Vec<u8>,
    pub created_at: NaiveDateTime,
    pub invited_by: Option<i32>,
}

pub struct NewPost<'a> {
    pub title: &'a str,
    pub url: Option<&'a str>,
    pub submitted_by: i32,
}

#[derive(Serialize)]
pub struct PostInfo {
    pub id: i32,
    pub uuid: Base32,
    pub title: String,
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
}

#[derive(Queryable)]
pub struct Comment {
    pub id: i32,
    pub text: String,
    pub html: String,
    pub visible: bool,
    pub post_id: i32,
    pub created_at: NaiveDateTime,
    pub created_by: i32,
}

pub struct NewComment<'a> {
    pub post_id: i32,
    pub text: &'a str,
    pub created_by: i32,
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
#[table_name="comment_stars"]
pub struct NewStarComment {
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
                self::posts::dsl::comment_count,
                self::posts::dsl::authored_by_submitter,
                self::posts::dsl::created_at,
                self::posts::dsl::submitted_by,
                self::stars::dsl::post_id.nullable(),
                self::users::dsl::username,
            ))
            .filter(visible.eq(true))
            .order_by((initial_stellar_time.desc(), self::posts::dsl::created_at.desc()))
            .limit(400)
            .get_results::<(i32, Base32, String, Option<String>, bool, i32, i32, i32, bool, NaiveDateTime, i32, Option<i32>, String)>(&self.0)?
            .into_iter()
            .map(|t| tuple_to_post_info(t, self.get_current_stellar_time()))
            .collect();
        all.sort_by_key(|info| OrderedFloat(-info.hotness));
        Ok(all)
    }
    pub fn get_post_info_by_uuid(&self, user_id_param: i32, uuid_param: Base32) -> Result<PostInfo, DieselError> {
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
                self::posts::dsl::comment_count,
                self::posts::dsl::authored_by_submitter,
                self::posts::dsl::created_at,
                self::posts::dsl::submitted_by,
                self::stars::dsl::post_id.nullable(),
                self::users::dsl::username,
            ))
            .filter(visible.eq(true))
            .filter(uuid.eq(uuid_param.into_i64()))
            .first::<(i32, Base32, String, Option<String>, bool, i32, i32, i32, bool, NaiveDateTime, i32, Option<i32>, String)>(&self.0)?, self.get_current_stellar_time()))
    }
    pub fn get_current_stellar_time(&self) -> i32 {
        use self::stars::dsl::*;
        // the stars table should be limited by the i32 limits, but diesel doesn't know that
        stars.count().get_result(&self.0).unwrap_or(0) as i32
    }
    pub fn create_post(&self, new_post: &NewPost) -> Result<Post, DieselError> {
        #[derive(Insertable)]
        #[table_name="posts"]
        struct CreatePost<'a> {
            title: &'a str,
            uuid: i64,
            url: Option<&'a str>,
            submitted_by: i32,
            initial_stellar_time: i32,
        }
        diesel::insert_into(posts::table)
            .values(CreatePost {
                title: new_post.title,
                url: new_post.url,
                uuid: rand::random(),
                submitted_by: new_post.submitted_by,
                initial_stellar_time: self.get_current_stellar_time(),
            })
            .get_result(&self.0)
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
    pub fn comment_on_post(&self, new_post: NewComment) -> Result<Comment, DieselError> {
        #[derive(Insertable)]
        #[table_name="comments"]
        struct CreateComment<'a> {
            text: &'a str,
            html: &'a str,
            post_id: i32,
            created_by: i32,
        }
        struct Data<'a>(&'a MoreInterestingConn);
        impl<'a> crate::prettify::Data for Data<'a> {
            fn check_comment_ref(&mut self, _id: i32) -> bool {
                false
            }
            fn check_hash_tag(&mut self, _tag: &str) -> bool {
                false
            }
            fn check_username(&mut self, _username: &str) -> bool {
                false
            }
        }
        let html_and_stuff = crate::prettify::prettify(new_post.text, &mut Data(self));
        self.update_comment_count_on_post(new_post.post_id, 1)?;
        diesel::insert_into(comments::table)
            .values(CreateComment{
                text: new_post.text,
                html: &html_and_stuff.string,
                post_id: new_post.post_id,
                created_by: new_post.created_by,
            })
            .get_result(&self.0)
    }
    pub fn get_comments_from_post(&self, post_id_param: i32, user_id_param: i32) -> Result<Vec<CommentInfo>, DieselError> {
        use self::comments::dsl::*;
        use self::comment_stars::dsl::*;
        use self::users::dsl::*;
        let all: Vec<CommentInfo> = comments
            .left_outer_join(comment_stars.on(comment_id.eq(self::comments::dsl::id).and(user_id.eq(user_id_param))))
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
                self::users::dsl::username,
            ))
            .filter(visible.eq(true))
            .filter(self::comments::dsl::post_id.eq(post_id_param))
            .order_by(self::comments::dsl::created_at)
            .limit(400)
            .get_results::<(i32, String, String, bool, i32, NaiveDateTime, i32, Option<i32>, String)>(&self.0)?
            .into_iter()
            .map(|t| tuple_to_comment_info(self, t))
            .collect();
        Ok(all)
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
}

fn tuple_to_post_info((id, uuid, title, url, visible, initial_stellar_time, score, comment_count, authored_by_submitter, created_at, submitted_by, starred_post_id, submitted_by_username): (i32, Base32, String, Option<String>, bool, i32, i32, i32, bool, NaiveDateTime, i32, Option<i32>, String), current_stellar_time: i32) -> PostInfo {
    PostInfo {
        id, uuid, title, url, visible, score, authored_by_submitter, created_at,
        submitted_by, submitted_by_username, comment_count,
        starred_by_me: starred_post_id.is_some(),
        hotness: compute_hotness(initial_stellar_time, current_stellar_time, score, authored_by_submitter)
    }
}

fn tuple_to_comment_info(conn: &MoreInterestingConn, (id, text, html, visible, post_id, created_at, created_by, starred_comment_id, created_by_username): (i32, String, String, bool, i32, NaiveDateTime, i32, Option<i32>, String)) -> CommentInfo {
    CommentInfo {
        id, text, html, visible, post_id, created_at, created_by, created_by_username,
        starred_by: conn.get_comment_starred_by(id).unwrap_or(Vec::new()),
        starred_by_me: starred_comment_id.is_some(),
    }
}

fn compute_hotness(initial_stellar_time: i32, current_stellar_time: i32, score: i32, authored_by_submitter: bool) -> f64 {
    let gravity = 1.33;
    let boost = if authored_by_submitter { 0.33 } else { 0.0 };
    let stellar_age = max(current_stellar_time - initial_stellar_time, 0) as f64;
    (boost + (score as f64) + 1.0) / (stellar_age + 1.0).powf(gravity)
}
