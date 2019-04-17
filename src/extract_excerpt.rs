use url::Url;
use kuchiki::traits::*;
use kuchiki::{NodeDataRef, ElementData, parse_html};
use html5ever::serialize::{serialize, SerializeOpts};

pub struct Excerpt {
    pub title: String,
    pub body: String,
}

pub fn extract_excerpt_url(url: Url) -> Option<Excerpt> {
    if let Ok(h) = readability::extractor::scrape(url.as_str()) {
        let body = to_plain_text(&h.content);
        Some(Excerpt {
            title: h.title,
            body,
        })
    } else {
        None
    }
}

pub fn extract_excerpt(html: &str, url: &Url) -> Option<Excerpt> {
    if let Ok(h) = readability::extractor::extract(&mut html.as_bytes(), url) {
        let body = to_plain_text(&h.content);
        Some(Excerpt {
            title: h.title,
            body,
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
