use rocket::request::FromRequest;
use rocket::Request;
use rocket::http::Status;
use rocket::Outcome;
use crate::models::*;
use serde::Serialize;

#[derive(Clone, Default, Serialize)]
pub struct Customization {
    pub title: String,
    pub css: String,
    pub custom_footer_html: String,
    pub front_notice_html: String,
    pub comments_placeholder_html: String,
    pub link_submit_notice_html: String,
    pub blog_post_notice_html: String,
    pub message_send_notice_html: String,
    pub post_score_text: String,
    pub comment_score_text: String,
}

impl<'a, 'r> FromRequest<'a, 'r> for Customization {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> Outcome<Customization, (Status, ()), ()> {
        let conn = MoreInterestingConn::from_request(request).unwrap();
        let customization_variables = match conn.get_customizations() {
            Ok(v) => v,
            Err(e) => {
                warn!("Failed to get customization variables: {:?}", e);
                return Outcome::Failure((Status::InternalServerError, ()));
            }
        };
        let mut customization = Customization::default();
        for variable in customization_variables {
            match &variable.name[..] {
                "title" => customization.title = variable.value,
                "css" => customization.css = variable.value,
                "custom_footer_html" => customization.custom_footer_html = variable.value,
                "front_notice_html" => customization.front_notice_html = variable.value,
                "comments_placeholder_html" => customization.comments_placeholder_html = variable.value,
                "link_submit_notice_html" => customization.link_submit_notice_html = variable.value,
                "blog_post_notice_html" => customization.blog_post_notice_html = variable.value,
                "post_score_text" => customization.post_score_text = variable.value,
                "comment_score_text" => customization.comment_score_text = variable.value,
                "message_send_notice_html" => customization.message_send_notice_html = variable.value,
                _ => (),
            }
        }
        Outcome::Success(customization)
    }
}
