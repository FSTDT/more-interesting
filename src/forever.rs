use rocket::Request;
use rocket::response::{self, Responder, Response};

pub struct CacheForever<T>(pub T);

impl<'r, 'o: 'r, T: Responder<'r, 'o> + Send> Responder<'r, 'o> for CacheForever<T> {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'o> {
        let mut build = Response::build();
        build.merge(self.0.respond_to(req)?);
        build.raw_header("cache-control", "public, max-age=30000000");
        build.ok()
    }
}

