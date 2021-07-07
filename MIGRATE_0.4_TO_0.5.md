This is a fairly large app that got migrated from Rocket 0.4 to 0.5. Here's a bit of info about how I did it.

# rocket_contrib

This package got split in half, forking rocket_sync_db_pools and rocket_dyn_templates. It was this patch to the Cargo.toml file.

```diff
diff --git a/Cargo.toml b/Cargo.toml
index 098eb0a..dc3924a 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -7,7 +7,8 @@ license = "MIT OR Apache-2.0"
 
 [dependencies]
 rocket = { git = "https://github.com/SergioBenitez/Rocket" }
-rocket_contrib = { git = "https://github.com/SergioBenitez/Rocket", default-features = false, features = ["diesel_postgres_pool", "serve", "handlebars_templates"] }
+rocket_sync_db_pools = { git = "https://github.com/SergioBenitez/Rocket", default-features = false, features = ["diesel_postgres_pool"] }
+rocket_dyn_templates = { git = "https://github.com/SergioBenitez/Rocket", default-features = false, features = ["handlebars"] }
 diesel = { version = "1.4.3", features = ["postgres", "chrono", "serde_json"] }
 diesel_full_text_search = "1.0.1"
 chrono = { version = "0.4.11", features = ["serde"] }
diff --git a/libraries/more-interesting-base32/Cargo.toml b/libraries/more-interesting-base32/Cargo.toml
index 5b5a9e3..707df6b 100644
--- a/libraries/more-interesting-base32/Cargo.toml
+++ b/libraries/more-interesting-base32/Cargo.toml
@@ -10,7 +10,7 @@ path = "lib.rs"
 
 [dependencies]
 rocket = { git = "https://github.com/SergioBenitez/Rocket" }
-rocket_contrib = { git = "https://github.com/SergioBenitez/Rocket", default-features = false, features = ["diesel_postgres_pool", "serve", "handlebars_templates"] }
+rocket_sync_db_pools = { git = "https://github.com/SergioBenitez/Rocket", default-features = false, features = ["diesel_postgres_pool"] }
 diesel = { version = "1.4.3", features = ["postgres", "chrono", "serde_json"] }
 serde = { version = "1.0.104", features = ["derive"] }
 byteorder = "1.3"
```

Notice that the old `serve` feature is now gone. It has been replaced with the `FileServer` struct in core: https://github.com/SergioBenitez/Rocket/tree/master/core/lib/src/fs

# Turning off nightly

    rustup override unset

# Switching base32 to the new form traits


```
error[E0432]: unresolved import `rocket::request::FromFormValue`
 --> libraries/more-interesting-base32/lib.rs:7:34
  |
7 | use rocket::request::{FromParam, FromFormValue};
```

Okay, so apparently FromFormValue became [FromFormField](https://api.rocket.rs/master/rocket/derive.FromFormField.html)?

INSERT DIFF of more-interesting-base32/lib.rs HERE

# Switching to the rocket::launch helper

My initial patch was to add `#[rocket::launch]` to main, rename main to launch, mark all the responders async, remove the `launch()` call from the end of the rocket call in launch, and hope for the best.

So, this conversion:

```diff
-fn main() {
+#[rocket::launch]
+fn launch() -> rocket::Rocket {
```

It didn't work, but it got me started.

* rocket::http::Cookies -> rocket::http::CookieJar
* rocket::Outcome -> rocket::outcome::Outcome
* `s@rocket_contrib::databases::@rocket_sync_db_pools::@g`
* `s@rocket::Rocket@rocket::Rocket<Rocket::Orbit>@g`
  * "launch" fairings become "liftoff" fairings
* A bunch of types had lifetime parameters added to them. I just let the compiler tell me where they went.
  * `s@<FlashMessage>@<FlashMessage<'_>>@g`
  * `s@CookieJar,@CookieJar<'_>,@g`
  * `s@UserAgentString,@UserAgentString<'_>,@g`
  * `s@ReferrerString,@ReferrerString<'_>,@g`
* FromRequest now has one lifetime for the trait, `'r`. The second lifetime is gone.
  * You should follow the example from https://api.rocket.rs/master/rocket/request/trait.FromRequest.html to implement it with rocket::async_trait
* Flash::msg -> Flash::message
  * `s@f.msg@f.message@g`
* You don't use Form tags for `..`-style query strings
  * `s@Form<MaybeRedirect>@MaybeRedirect@g`
  * `s@Option<Form<IndexParams>>@Option<IndexParams>@g`
  * `s@Option<Form<SearchCommentsParams>>@Option<SearchCommentsParams>@g`
  * `s@Option<Form<ModLogParams>>@Option<ModLogParams>@g`
  * `s@Option<Form<GetEditPost>>@Option<GetEditPost>@g`
  * `s@Option<Form<GetEditComment>>@Option<GetEditComment>@g`
  * `s@Option<Form<GetReplyComment>>@Option<GetReplyComment>@g`

* Switch from manually pulling off config values, to using the new serde-based system


    -    .attach(fairing::AdHoc::on_attach("site config", |rocket| {
    -        let mut public_url = rocket.config().get_str("public_url").unwrap_or("http://localhost").to_owned();
    -        if !public_url.starts_with("http:") && !public_url.starts_with("https:") {
    -            public_url = "https://".to_owned() + &public_url;
    -        }
    -        let public_url = Url::parse(&public_url).expect("public_url configuration must be valid");
    -        let enable_user_directory = rocket.config().get_bool("enable_user_directory").unwrap_or(true);
    -        let enable_anonymous_submissions = rocket.config().get_bool("enable_anonymous_submissions").unwrap_or(false);
    -        let enable_public_signup = rocket.config().get_bool("enable_public_signup").unwrap_or(false);
    -        let hide_text_post = rocket.config().get_bool("hide_text_post").unwrap_or(false);
    -        let hide_link_post = rocket.config().get_bool("hide_link_post").unwrap_or(false);
    -        let body_format = match rocket.config().get_str("body_format").unwrap_or("") {
    -            "bbcode" => models::BodyFormat::BBCode,
    -            _ => models::BodyFormat::Plain,
    -        };
    -        Ok(rocket.manage(SiteConfig {
    -            enable_user_directory, public_url,
    -            enable_anonymous_submissions,
    -            enable_public_signup,
    -            hide_text_post, hide_link_post,
    -            body_format,
    -        }))
    -    }))
    +    .attach(fairing::AdHoc::config::<SiteConfig>())

All of the logic in there instead got put into a custom Deserialize implementation.

* The configuration, while still stored in ad-hoc state, is now behind a reference:
  * `s@State<SiteConfig>@&State<SiteConfig>@g`
  * `s@config.clone@config.inner().clone@g`
* The cookie jar also needs to be accessed as a reference
  * `s@mut cookies: CookieJar@cookies: &CookieJar@g`

* The database stuff needs to use the `run()` handler, https://api.rocket.rs/master/rocket_sync_db_pools/index.html#handlers
  * add `async` to all of the helper methods, and move them all so that the synchronous part uses the PgConnection directly
  * The publicly-visible "async" functions are just wrappers on top of the synchronous diesel-based query code