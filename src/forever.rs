use rocket::Request;
use rocket::response::{self, Responder, Response};

pub struct CacheForever<T>(pub T);

impl<'r, T: Responder<'r>> Responder<'r> for CacheForever<T> {
    fn respond_to(self, req: &Request) -> response::Result<'r> {
        let mut build = Response::build();
        build.merge(self.0.respond_to(req)?);
        build.raw_header("cache-control", "cache-control: public, max-age=30000000");
        build.ok()
    }
}

