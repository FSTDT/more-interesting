use rocket::request::FromRequest;
use rocket::Request;
use rocket::http::Status;
use rocket::Outcome;
use crate::models::*;

impl<'a, 'r> FromRequest<'a, 'r> for User {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> Outcome<User, (Status, ()), ()> {
        let mut cookies = request.cookies();
        let user_id = cookies
            .get_private("user_id")
            .and_then(|cookie| cookie.value().parse().ok());
        if let Some(user_id) = user_id {
            let conn = MoreInterestingConn::from_request(request).unwrap();
            if let Ok(user) = conn.get_user_by_id(user_id) {
                Outcome::Success(user)
            } else {
                Outcome::Failure((Status::BadRequest, ()))
            }
        } else {
            Outcome::Failure((Status::BadRequest, ()))
        }
    }
}
