use askama::Template;
use serde::{Serialize, Serializer};
use crate::models::{Comment, CommentInfo, CommentSearchResult, LegacyCommentInfo, ModerationInfo, NotificationInfo, PostInfo, User, UserSession};
use crate::models::{Tag, CommentFlagInfo, PollInfo, PostFlagInfo, SiteCustomization, DomainSynonymInfo};
use crate::models::BlockedRegex;
use crate::customization::Customization;
use crate::SiteConfig;
use more_interesting_base32::Base32;
use chrono::{NaiveDate, Duration, Utc};

#[derive(Template)]
#[template(path = "index.html")]
pub struct Index {
    pub alert: String,
    pub user: User,
    pub session: UserSession,
    pub notifications: Vec<NotificationInfo>,
    pub customization: Customization,
    pub config: SiteConfig,
    pub next_search_page: i32,
    pub before_date_param: Option<NaiveDate>,
    pub after_date_param: Option<NaiveDate>,
    pub title: String,
    pub posts: Vec<PostInfo>,
    pub is_home: bool,
    pub tags: Vec<Tag>,
    pub tag_param: String,
    pub domain: String,
    pub keywords_param: String,
    pub title_param: String,
    pub extra_blog_posts: Vec<PostInfo>,
    pub noindex: bool,
}

#[derive(Template)]
#[template(path = "similar.html")]
pub struct Similar {
    pub alert: String,
    pub user: User,
    pub session: UserSession,
    pub notifications: Vec<NotificationInfo>,
    pub customization: Customization,
    pub config: SiteConfig,
    pub title: String,
    pub posts: Vec<PostInfo>,
    pub noindex: bool,
}

#[derive(Template)]
#[template(path = "subscriptions.html")]
pub struct Subscriptions {
    pub alert: String,
    pub user: User,
    pub session: UserSession,
    pub notifications: Vec<NotificationInfo>,
    pub customization: Customization,
    pub config: SiteConfig,
    pub before_date_param: Option<NaiveDate>,
    pub after_date_param: Option<NaiveDate>,
    pub title: String,
    pub posts: Vec<PostInfo>,
    pub is_home: bool,
    pub tags: Vec<Tag>,
    pub tag_param: String,
    pub domain: String,
    pub keywords_param: String,
    pub title_param: String,
    pub noindex: bool,
}

#[derive(Clone, Copy, FromFormField, Eq, PartialEq)]
pub enum Timespan {
    Day,
    Week,
    Month,
    Year,
    All,
}

#[derive(Template)]
#[template(path = "index-top.html")]
pub struct IndexTop {
    pub alert: String,
    pub user: User,
    pub session: UserSession,
    pub notifications: Vec<NotificationInfo>,
    pub customization: Customization,
    pub config: SiteConfig,
    pub next_search_page: i32,
    pub before_date_param: Option<NaiveDate>,
    pub after_date_param: Option<NaiveDate>,
    pub timespan: Timespan,
    pub title: String,
    pub posts: Vec<PostInfo>,
    pub is_home: bool,
    pub tags: Vec<Tag>,
    pub tag_param: String,
    pub domain: String,
    pub keywords_param: String,
    pub title_param: String,
    pub noindex: bool,
}

#[derive(Template)]
#[template(path = "index-new.html")]
pub struct IndexNew {
    pub alert: String,
    pub user: User,
    pub session: UserSession,
    pub notifications: Vec<NotificationInfo>,
    pub customization: Customization,
    pub config: SiteConfig,
    pub next_search_page: i32,
    pub before_date_param: Option<NaiveDate>,
    pub after_date_param: Option<NaiveDate>,
    pub title: String,
    pub posts: Vec<PostInfo>,
    pub is_home: bool,
    pub tags: Vec<Tag>,
    pub tag_param: String,
    pub domain: String,
    pub keywords_param: String,
    pub title_param: String,
    pub noindex: bool,
}

#[derive(Template)]
#[template(path = "index-latest.html")]
pub struct IndexLatest {
    pub alert: String,
    pub user: User,
    pub session: UserSession,
    pub notifications: Vec<NotificationInfo>,
    pub customization: Customization,
    pub config: SiteConfig,
    pub next_search_page: i32,
    pub before_date_param: Option<NaiveDate>,
    pub after_date_param: Option<NaiveDate>,
    pub title: String,
    pub posts: Vec<PostInfo>,
    pub is_home: bool,
    pub tags: Vec<Tag>,
    pub tag_param: String,
    pub domain: String,
    pub keywords_param: String,
    pub title_param: String,
    pub noindex: bool,
}

#[derive(Template)]
#[template(path = "index-random.html")]
pub struct IndexRandom {
    pub alert: String,
    pub user: User,
    pub session: UserSession,
    pub notifications: Vec<NotificationInfo>,
    pub customization: Customization,
    pub config: SiteConfig,
    pub next_search_page: i32,
    pub before_date_param: Option<NaiveDate>,
    pub after_date_param: Option<NaiveDate>,
    pub title: String,
    pub posts: Vec<PostInfo>,
    pub is_home: bool,
    pub tags: Vec<Tag>,
    pub tag_param: String,
    pub domain: String,
    pub keywords_param: String,
    pub title_param: String,
    pub noindex: bool,
}

#[derive(Template)]
#[template(path = "preview-post.html")]
pub struct PreviewPost {
    pub alert: String,
    pub title: String,
    pub user: User,
    pub session: UserSession,
    pub notifications: Vec<NotificationInfo>,
    pub customization: Customization,
    pub config: SiteConfig,
    pub post: PostInfo,
    pub excerpt: Option<String>,
    pub noindex: bool,
}

#[derive(Template)]
#[template(path = "preview-submit.html")]
pub struct PreviewSubmit {
    pub alert: String,
    pub user: User,
    pub session: UserSession,
    pub notifications: Vec<NotificationInfo>,
    pub customization: Customization,
    pub config: SiteConfig,
    pub title: String,
    pub post: PostInfo,
    pub excerpt: Option<String>,
    pub noindex: bool,
}

#[derive(Template)]
#[template(path = "submit.html")]
pub struct Submit {
    pub alert: String,
    pub user: User,
    pub session: UserSession,
    pub notifications: Vec<NotificationInfo>,
    pub customization: Customization,
    pub config: SiteConfig,
    pub title: String,
    pub post: PostInfo,
    pub excerpt: Option<String>,
    pub noindex: bool,
}

#[derive(Template)]
#[template(path = "message.html")]
pub struct Message {
    pub alert: String,
    pub user: User,
    pub session: UserSession,
    pub notifications: Vec<NotificationInfo>,
    pub customization: Customization,
    pub config: SiteConfig,
    pub title: String,
    pub noindex: bool,
}

#[derive(Template)]
#[template(path = "signup.html")]
pub struct Signup {
    pub alert: String,
    pub customization: Customization,
    pub config: SiteConfig,
    pub title: String,
    pub invite_token: Option<Base32>,
    pub user: User, // default
    pub session: UserSession, // default
    pub notifications: Vec<NotificationInfo>, // always empty
    pub noindex: bool,
}

#[derive(Template)]
#[template(path = "login.html")]
pub struct Login {
    pub title: String,
    pub alert: String,
    pub customization: Customization,
    pub config: SiteConfig,
    pub user: User, // default
    pub session: UserSession, // default
    pub notifications: Vec<NotificationInfo>, // always empty
    pub noindex: bool,
}

#[derive(Template)]
#[template(path = "search.html")]
pub struct Search {
    pub alert: String,
    pub title: &'static str,
    pub config: SiteConfig,
    pub next_search_page: i32,
    pub customization: Customization,
    pub before_date_param: Option<NaiveDate>,
    pub after_date_param: Option<NaiveDate>,
    pub user: User,
    pub tags: Vec<Tag>,
    pub session: UserSession,
    pub tag_param: String,
    pub domain: String,
    pub keywords_param: String,
    pub title_param: String,
    pub notifications: Vec<NotificationInfo>,
    pub noindex: bool,
}

#[derive(Template)]
#[template(path = "profile-posts.html")]
pub struct ProfilePosts {
    pub alert: String,
    pub user: User,
    pub session: UserSession,
    pub notifications: Vec<NotificationInfo>,
    pub customization: Customization,
    pub config: SiteConfig,
    pub title: String,
    pub posts: Vec<PostInfo>,
    pub noindex: bool,
    pub is_me: bool,
}

#[derive(Template)]
#[template(path = "index-comments.html")]
pub struct IndexComments {
    pub alert: String,
    pub config: SiteConfig,
    pub customization: Customization,
    pub title: String,
    pub user: User,
    pub comment_search_result: Vec<CommentSearchResult>,
    pub session: UserSession,
    pub noindex: bool,
    pub notifications: Vec<NotificationInfo>,
}

#[derive(Template)]
#[template(path = "profile-comments.html")]
pub struct ProfileComments {
    pub alert: String,
    pub config: SiteConfig,
    pub customization: Customization,
    pub is_me: bool,
    pub title: String,
    pub user: User,
    pub comment_search_result: Vec<CommentSearchResult>,
    pub session: UserSession,
    pub noindex: bool,
    pub notifications: Vec<NotificationInfo>,
}

#[derive(Template)]
#[template(path = "blog.html")]
pub struct Blog {
    pub alert: String,
    pub config: SiteConfig,
    pub next_search_page: i32,
    pub customization: Customization,
    pub before_date_param: Option<NaiveDate>,
    pub after_date_param: Option<NaiveDate>,
    pub title: String,
    pub user: User,
    pub posts: Vec<PostInfo>,
    pub session: UserSession,
    pub keywords_param: String,
    pub title_param: String,
    pub notifications: Vec<NotificationInfo>,
    pub noindex: bool,
}

#[derive(Template)]
#[template(path = "create-post.html")]
pub struct CreatePost {
    pub alert: String,
    pub title: String,
    pub excerpt: Option<String>,
    pub user: User,
    pub session: UserSession,
    pub notifications: Vec<NotificationInfo>,
    pub customization: Customization,
    pub config: SiteConfig,
    pub post: PostInfo,
    pub noindex: bool,
}

#[derive(Template)]
#[template(path = "edit-post.html")]
pub struct EditPost {
    pub title: String,
    pub alert: String,
    pub user: User,
    pub session: UserSession,
    pub notifications: Vec<NotificationInfo>,
    pub customization: Customization,
    pub config: SiteConfig,
    pub post_info: PostInfo,
    pub excerpt: Option<String>,
    pub noindex: bool,
}

#[derive(Template)]
#[template(path = "edit-comment.html")]
pub struct EditComment {
    pub title: String,
    pub alert: String,
    pub user: User,
    pub session: UserSession,
    pub notifications: Vec<NotificationInfo>,
    pub customization: Customization,
    pub config: SiteConfig,
    pub comment: Comment,
    pub noindex: bool,
}

#[derive(Template)]
#[template(path = "reply-comment.html")]
pub struct ReplyComment {
    pub title: String,
    pub alert: String,
    pub user: User,
    pub session: UserSession,
    pub notifications: Vec<NotificationInfo>,
    pub customization: Customization,
    pub config: SiteConfig,
    pub comment: CommentInfo,
    pub post: PostInfo,
    pub is_subscribed: bool,
    pub noindex: bool,
}

#[derive(Template)]
#[template(path = "mod-log.html")]
pub struct ModLog {
    pub title: String,
    pub alert: String,
    pub user: User,
    pub session: UserSession,
    pub log: Vec<ModerationInfo>,
    pub notifications: Vec<NotificationInfo>,
    pub customization: Customization,
    pub config: SiteConfig,
    pub noindex: bool,
}

#[derive(Template)]
#[template(path = "view-star.html")]
pub struct ViewStar {
    pub starred_by: Vec<String>,
    pub blog_post: bool,
    pub customization: Customization,
}

#[derive(Template)]
#[template(path = "view-star-comment.html")]
pub struct ViewStarComment {
    pub starred_by: Vec<String>,
    pub customization: Customization,
}

#[derive(Template)]
#[template(path = "tags.html")]
pub struct Tags {
    pub alert: String,
    pub title: String,
    pub user: User,
    pub session: UserSession,
    pub notifications: Vec<NotificationInfo>,
    pub customization: Customization,
    pub config: SiteConfig,
    pub tags: Vec<Tag>,
    pub noindex: bool,
}

#[derive(Template)]
#[template(path = "invite-tree.html")]
pub struct InviteTree {
    pub alert: String,
    pub title: String,
    pub user: User,
    pub session: UserSession,
    pub notifications: Vec<NotificationInfo>,
    pub customization: Customization,
    pub config: SiteConfig,
    pub raw_html: String,
    pub noindex: bool,
}

#[derive(Template)]
#[template(path = "faq.html")]
pub struct Faq {
    pub title: String,
    pub alert: String,
    pub user: User,
    pub session: UserSession,
    pub notifications: Vec<NotificationInfo>,
    pub customization: Customization,
    pub config: SiteConfig,
    pub raw_html: String,
    pub noindex: bool,
}

#[derive(Template)]
#[template(path = "settings.html")]
pub struct Settings {
    pub alert: String,
    pub user: User,
    pub session: UserSession,
    pub notifications: Vec<NotificationInfo>,
    pub customization: Customization,
    pub config: SiteConfig,
    pub title: String,
    pub noindex: bool,
}

#[derive(Template)]
#[template(path = "admin/tags.html")]
pub struct AdminTags {
    pub title: String,
    pub alert: String,
    pub page: AdminPageId,
    pub user: User,
    pub session: UserSession,
    pub customization: Customization,
    pub config: SiteConfig,
    pub tags: Vec<Tag>,
}

#[derive(Template)]
#[template(path = "admin/comment-flags.html")]
pub struct AdminCommentFlags {
    pub title: String,
    pub alert: String,
    pub page: AdminPageId,
    pub user: User,
    pub session: UserSession,
    pub customization: Customization,
    pub config: SiteConfig,
    pub comment_flags: Vec<CommentFlagInfo>,
}

#[derive(Template)]
#[template(path = "admin/customization.html")]
pub struct AdminCustomization {
    pub title: String,
    pub alert: String,
    pub page: AdminPageId,
    pub user: User,
    pub session: UserSession,
    pub customization: Customization,
    pub config: SiteConfig,
    pub site_customization: Vec<SiteCustomization>,
}

#[derive(Template)]
#[template(path = "admin/domains.html")]
pub struct AdminDomains {
    pub title: String,
    pub alert: String,
    pub page: AdminPageId,
    pub user: User,
    pub session: UserSession,
    pub customization: Customization,
    pub config: SiteConfig,
    pub domain_synonyms: Vec<DomainSynonymInfo>,
}

#[derive(Template)]
#[template(path = "admin/flags.html")]
pub struct AdminFlags {
    pub title: String,
    pub alert: String,
    pub page: AdminPageId,
    pub user: User,
    pub session: UserSession,
    pub customization: Customization,
    pub config: SiteConfig,
    pub post_flags: Vec<PostFlagInfo>,
}

#[derive(Template)]
#[template(path = "admin/users.html")]
pub struct AdminUsers {
    pub title: String,
    pub alert: String,
    pub page: AdminPageId,
    pub user: User,
    pub session: UserSession,
    pub customization: Customization,
    pub config: SiteConfig,
    pub username: String,
    pub users_list: Vec<User>,
}

#[derive(Template)]
#[template(path = "admin/blocked-regexes.html")]
pub struct AdminBlockedRegexes {
    pub title: String,
    pub alert: String,
    pub page: AdminPageId,
    pub user: User,
    pub session: UserSession,
    pub customization: Customization,
    pub config: SiteConfig,
    pub blocked_regexes: Vec<BlockedRegex>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum NavPageId {
    Home = 1,
    Latest = 2,
    Top = 3,
    New = 4,
    Random = 5,
}

impl ToString for NavPageId {
    fn to_string(&self) -> String {
        use NavPageId::*;
        (match *self {
            Home => ".",
            Latest => "latest",
            Top => "top",
            New => "new",
            Random => "random",
        }).to_string()
    }
}

impl Serialize for NavPageId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i32(*self as i32)
    }
}

impl Default for NavPageId {
    fn default() -> NavPageId {
        NavPageId::Home
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum AdminPageId {
    Tags = 1,
    Domains = 2,
    Customization = 3,
    Flags = 4,
    CommentFlags = 5,
    Users = 6,
    BlockedRegexes = 7,
}

impl Serialize for AdminPageId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i32(*self as i32)
    }
}

impl Default for AdminPageId {
    fn default() -> AdminPageId {
        AdminPageId::Tags
    }
}

#[derive(Template)]
#[template(path = "rss.xml")]
pub struct Rss {
    pub config: SiteConfig,
    pub customization: Customization,
    pub link: String,
    pub posts: Vec<PostInfo>,
}

#[derive(Template)]
#[template(path = "blog.rss.xml")]
pub struct BlogRss {
    pub config: SiteConfig,
    pub customization: Customization,
    pub link: String,
    pub posts: Vec<PostInfo>,
}

#[derive(Template)]
#[template(path = "comments.html")]
pub struct Comments {
    pub title: String,
    pub polls: Vec<PollInfo>,
    pub poll_count: usize,
    pub alert: String,
    pub notifications: Vec<NotificationInfo>,
    pub is_private: bool,
    pub is_subscribed: bool,
    pub comment_preview_text: String,
    pub comment_preview_html: String,
	pub post_info: PostInfo,
	pub user: User,
	pub session: UserSession,
    pub starred_by: Vec<String>,
    pub legacy_comments: Vec<LegacyCommentInfo>,
    pub comments: Vec<CommentInfo>,
    pub customization: Customization,
    pub config: SiteConfig,
    pub noindex: bool,
    pub locked: bool,
}

#[derive(Serialize)]
#[serde(tag = "ty", content = "inner", rename_all = "snake_case")]
pub enum ModQueueItem {
    Post {
        post: PostInfo,
        comments: Vec<CommentInfo>,
    },
    Comment {
        post: PostInfo,
        comment: CommentInfo,
    },
}

#[derive(Template)]
#[template(path = "mod-queue.html")]
pub struct ModQueue {
    pub alert: String,
    pub title: String,
    pub user: User,
    pub session: UserSession,
    pub notifications: Vec<NotificationInfo>,
    pub customization: Customization,
    pub config: SiteConfig,
    pub mod_queue: Vec<ModQueueItem>,
    pub noindex: bool,
}

pub fn replace<T: ToString>(template: &str, subst: T) -> String {
    template.replace("{}", &subst.to_string()).to_owned()
}

pub mod filters {
    use askama::Result as AskamaResult;
    use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
    use lazy_static::lazy_static;
    use regex::Regex;
    pub fn count<T>(t: &[T]) -> AskamaResult<usize> {
        Ok(t.len())
    }
    pub fn last<T: ToString>(t: &[T]) -> AskamaResult<String> {
        if let Some(subst) = t.last() {
            Ok(subst.to_string())
        } else {
            Ok(String::new())
        }
    }
    pub fn urlencode(param: &str) -> AskamaResult<String> {
        Ok(utf8_percent_encode(&param.to_string(), NON_ALPHANUMERIC).to_string())
    }
    pub fn usernamewrap(username: &str) -> AskamaResult<String> {
        lazy_static!{
            static ref BASIC_USERNAME: Regex = Regex::new(r"^[a-zA-Z0-9\-_]+$").unwrap();
        }
        let is_basic = BASIC_USERNAME.is_match(&username);
        if is_basic {
            Ok(format!("@{username}"))
        } else {
            Ok(format!("<@{username}>"))
        }
    }
}