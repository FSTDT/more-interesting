use url::Url;
use kuchiki::traits::*;
use kuchiki::{NodeDataRef, ElementData, parse_html};
use html5ever::serialize::{serialize, SerializeOpts};
use std::time::Duration;
use std::io::Read;

pub struct Excerpt {
    pub title: String,
    pub body: String,
    pub url: Url,
}

pub fn extract_excerpt_url(url: Url) -> Option<Excerpt> {
    let client = reqwest::Client::builder()
        .timeout(Duration::new(30, 0))
        .build()
        .ok()?;
    const UA: &'static str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:66.0) Gecko/20100101 Firefox/66.0";
    let mut res = client.get(url.clone())
        .header(reqwest::header::USER_AGENT, UA)
        .send()
        .ok()?;
    if res.status().is_success() {
        let u = res.url().clone();
        extract_excerpt(&mut res, u)
    } else {
        None
    }
}

pub fn extract_excerpt<R: Read>(mut reader: R, url: Url) -> Option<Excerpt> {
    if let Ok(h) = readability::extractor::extract(&mut reader, &url) {
        let body = to_plain_text(&h.content);
        Some(Excerpt {
            title: h.title,
            url, body,
        })
    } else {
        None
    }
}

fn to_plain_text(html: &str) -> String {
    let document = parse_html().one(html);
    let mut out = String::new();
    for child in document.select("p, pre, table").expect("valid CSS selectors should be fine") {
        extract_text(&child, &mut out);
    }
    out.trim().to_owned()
}

fn extract_text(el: &NodeDataRef<ElementData>, out: &mut String) {
    if &*el.name.local == "p" {
        extract_text_all(&el, out);
        out.push_str("\n\n");
    } else if &*el.name.local == "pre" {
        out.push_str("<code>");
        out.push_str(&el.text_contents());
        out.push_str("</code>\n\n");
    } else if &*el.name.local == "table" {
        out.push_str("<table>");
        let mut ret_val = Vec::new();
        serialize(&mut ret_val, el.as_node(), SerializeOpts::default())
            .expect("Writing to a string shouldn't fail (expect on OOM)");
        out.push_str(&String::from_utf8(ret_val)
            .expect("html5ever only supports UTF8"));
        out.push_str("</table>\n\n");
    } else {
        extract_text_all(&el, out);
    }
}
fn extract_text_all(element: &NodeDataRef<ElementData>, out: &mut String) {
    for child in element.as_node().children() {
        if let Some(el) = child.clone().into_element_ref() {
            extract_text(&el, out);
        } else {
            out.push_str(&child.text_contents());
        }
    }
}
