use rocket::request::FromRequest;
use rocket::Request;
use rocket::http::{Cookie, SameSite, Method, Status};
use rocket::Outcome;
use crate::models::*;
use more_interesting_base32::Base32;
use std::mem;

pub struct UserAgentString<'a> {
    pub user_agent: &'a str,
}

impl<'a, 'r> FromRequest<'a, 'r> for UserAgentString<'a> {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> Outcome<UserAgentString<'a>, (Status, ()), ()> {
        let user_agent = request.headers().get("user-agent").next();
        if let Some(user_agent) = user_agent {
            Outcome::Success(UserAgentString { user_agent })
        } else {
            Outcome::Failure((Status::BadRequest, ()))
        }
    }
}

pub struct ReferrerString<'a> {
    pub referrer: &'a str,
}

impl<'a, 'r> FromRequest<'a, 'r> for ReferrerString<'a> {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> Outcome<ReferrerString<'a>, (Status, ()), ()> {
        let referrer = request.headers().get("referer").next();
        if let Some(referrer) = referrer {
            Outcome::Success(ReferrerString { referrer })
        } else {
            Outcome::Failure((Status::BadRequest, ()))
        }
    }
}

pub struct LoginSession {
    pub session: UserSession,
    pub user: User,
}

impl<'a, 'r> FromRequest<'a, 'r> for LoginSession {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> Outcome<LoginSession, (Status, ()), ()> {
        let mut cookies = request.cookies();
        let session_uuid: Option<Base32> = cookies
            .get("U")
            .and_then(|cookie| cookie.value().parse().ok());
        if request.method() != Method::Get && request.method() != Method::Head {
            let session_uuid_param: Option<Base32> = request.get_query_value::<Option<Base32>>("U")
                .and_then(|v| v.ok()).and_then(|v| v);
            if session_uuid_param != session_uuid {
                warn!("Got invalid CSRF session token");
                return Outcome::Failure((Status::BadRequest, ()));
            }
        }
        if let Some(session_uuid) = session_uuid {
            let conn = MoreInterestingConn::from_request(request).unwrap();
            if let Ok(session) = conn.get_session_by_uuid(session_uuid) {
                if let Ok(user) = conn.get_user_by_id(session.user_id) {
                    if user.trust_level == -2 { 
                        let cookie = Cookie::build("B", "1").path("/").permanent().same_site(SameSite::None).finish(); 
                        cookies.add(cookie); 
                    } else if cookies.get("B").is_some() {
                        conn.change_user_trust_level(user.id, -2).expect("if logging in worked, then so should changing trust level");
                    }
                    if user.banned {
                        return Outcome::Failure((Status::BadRequest, ()));
                    }
                    Outcome::Success(LoginSession { session, user })
                } else {
                    Outcome::Failure((Status::BadRequest, ()))
                }
            } else {
                Outcome::Failure((Status::BadRequest, ()))
            }
        } else {
            Outcome::Failure((Status::BadRequest, ()))
        }
    }
}

pub struct ModeratorSession {
    pub session: UserSession,
    pub user: User,
}

impl<'a, 'r> FromRequest<'a, 'r> for ModeratorSession {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> Outcome<ModeratorSession, (Status, ()), ()> {
        match LoginSession::from_request(request) {
            Outcome::Success(ref mut login_session) if login_session.user.trust_level >= 3 => {
                let user = mem::replace(&mut login_session.user, User::default());
                let session = mem::replace(&mut login_session.session, UserSession::default());
                Outcome::Success(ModeratorSession { session, user })
            },
            Outcome::Success(_junior_user) => Outcome::Failure((Status::BadRequest, ())),
            Outcome::Failure(f) => Outcome::Failure(f),
            Outcome::Forward(f) => Outcome::Forward(f),
        }
    }
}
