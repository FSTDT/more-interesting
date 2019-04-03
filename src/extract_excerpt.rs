use url::Url;
use kuchiki::traits::*;
use kuchiki::{NodeDataRef, ElementData, parse_html};
use html5ever::serialize::{serialize, SerializeOpts};

pub fn extract_excerpt_url(url: Url) -> String {
    if let Ok(h) = readability::extractor::scrape(url.as_str()) {
        to_plain_text(&h.content)
    } else {
        String::new()
    }
}

pub fn extract_excerpt(html: &str, url: &Url) -> String {
    if let Ok(h) = readability::extractor::extract(&mut html.as_bytes(), url) {
        to_plain_text(&h.content)
    } else {
        String::new()
    }
}

fn to_plain_text(html: &str) -> String {
    let document = parse_html().one(html);
    let mut out = String::new();
    let mut links = Vec::new();
    for child in document.select("p, pre, table").expect("valid CSS selectors should be fine") {
        extract_text(&child, &mut out, &mut links);
    }
    for (i, link) in links.iter().enumerate() {
        out.push_str("\n[");
        out.push_str(&(i + 1).to_string());
        out.push_str("]: ");
        out.push_str(&link);
        out.push_str("\n")
    }
    out.trim().to_owned()
}

fn extract_text(el: &NodeDataRef<ElementData>, out: &mut String, links: &mut Vec<String>) {
    if &*el.name.local == "p" {
        extract_text_all(&el, out, links);
        out.push_str("\n\n");
    } else if &*el.name.local == "pre" {
        out.push_str("<pre>");
        out.push_str(&el.text_contents());
        out.push_str("</pre>\n\n");
    } else if &*el.name.local == "table" {
        out.push_str("<table>");
        let mut ret_val = Vec::new();
        serialize(&mut ret_val, el.as_node(), SerializeOpts::default())
            .expect("Writing to a string shouldn't fail (expect on OOM)");
        out.push_str(&String::from_utf8(ret_val)
            .expect("html5ever only supports UTF8"));
        out.push_str("</table>\n\n");
    } else if &*el.name.local == "a" && el.attributes.borrow().contains("href") {
        extract_text_all(&el, out, links);
        links.push(el.attributes.borrow().get("href").expect("we just checked").to_owned());
        out.push_str(" [");
        out.push_str(&links.len().to_string());
        out.push_str("]");
    } else {
        extract_text_all(&el, out, links);
    }
}
fn extract_text_all(element: &NodeDataRef<ElementData>, out: &mut String, links: &mut Vec<String>) {
    for child in element.as_node().children() {
        if let Some(el) = child.clone().into_element_ref() {
            extract_text(&el, out, links);
        } else {
            out.push_str(&child.text_contents());
        }
    }
}
