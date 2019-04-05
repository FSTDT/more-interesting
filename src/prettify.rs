use v_htmlescape::escape;
use std::fmt::{self, Display, Formatter};
use lazy_static::lazy_static;

const URL_PROTOCOLS: &[&str] = &["http:", "https:", "ftp:", "gopher:", "mailto:", "magnet:"];

lazy_static!{
    static ref CLEANER: ammonia::Builder<'static> = {
        let mut b = ammonia::Builder::default();
        b.add_allowed_classes("a", ["inner-link", "article-header-inner"][..].iter().cloned());
        b.tags(["a", "p"][..].iter().cloned().collect());
        b
    };
}

/// Prettify: transform plain text, as described in the readme, into HTML with links.
///
/// # Syntax
///
/// - Links are recognized by starting with `http:`, `https:`, `ftp:`, `gopher:`, `mailto:`, or `magnet:`.
///   If the link ends with punctuation, you can enclose it in `<` and `>` angle brackets.
/// - Emails can be written like <michael@notriddle.com>. They are not auto-linked outside of
///   angle brackets.
/// - You can link to users with `@`. In case a username ends with punctuation,
///   they can also be enclosed with angle brackets to clarify them.
/// - You can link to tags with `#`. Tags cannot be composed entirely of numbers.
///   In case a tag name ends in punctuation, they can also be enclosed in
///   angle brackets.
/// - You can link to another comment with `#`. There's nothing stopping you from putting
///   them in angle brackets, but since they're always entirely composed of digits,
///   there's no real reason to.
///
/// And that's it. There's intentionally no way to do things like make text bold or put
/// in headers. When syntax is used, the special characters still get written out to the
/// resulting HTML, further improving the learnability of the site.
///
/// # Parameters
///
/// - `text`: A plain text input (well, there are five special syntactic constructs)
/// - `data`: Used to check if particular usernames exist.
pub fn prettify_body<D: Data>(text: &str, data: &mut D) -> Output {
    let text = text.replace("\r\n", "\n");
    let mut text = &text[..];
    let mut ret_val = Output::with_capacity(text.len());
    ret_val.push_str("<p>");
    while let Some(c) = text.as_bytes().get(0) {
        match c {
            b'<' => {
                let (contents, brackets_count, count) = scan_angle_brackets(text);
                assert_ne!(brackets_count, 0);
                for _ in 0..brackets_count {
                    ret_val.push_str("&lt;");
                }
                if starts_with_url_protocol(contents) {
                    let html = escape(&contents).to_string();
                    ret_val.push_str("<a href=\"");
                    ret_val.push_str(&html);
                    ret_val.push_str("\">");
                    ret_val.push_str(&html);
                    ret_val.push_str("</a>");
                } else if contents.starts_with("www.") {
                    let html = escape(&contents).to_string();
                    ret_val.push_str("<a href=\"https://");
                    ret_val.push_str(&html);
                    ret_val.push_str("\">");
                    ret_val.push_str(&html);
                    ret_val.push_str("</a>");
                } else if contents.starts_with('@') {
                    maybe_write_username(&contents[1..], data, &mut ret_val, None);
                } else if contents.contains('@') {
                    let html = escape(&contents).to_string();
                    ret_val.push_str("<a href=\"mailto:");
                    ret_val.push_str(&html);
                    ret_val.push_str("\">");
                    ret_val.push_str(&html);
                    ret_val.push_str("</a>");
                } else if contents.starts_with('#') {
                    maybe_write_number_sign(&contents[1..], data, &mut ret_val, None);
                } else {
                    ret_val.push_str(&escape(&text[brackets_count..count]).to_string());
                }
                if contents != "" {
                    for _ in 0..brackets_count {
                        ret_val.push_str("&gt;");
                    }
                    text = &text[brackets_count+count..];
                } else {
                    text = &text[brackets_count..];
                }
            }
            b'@' => {
                let contents = scan_lexical_token(&text[1..], false);
                maybe_write_username(contents, data, &mut ret_val, None);
                text = &text[(1 + contents.len())..];
            }
            b'#' => {
                let contents = scan_lexical_token(&text[1..], false);
                maybe_write_number_sign(contents, data, &mut ret_val, None);
                text = &text[(1 + contents.len())..];
            }
            _ if starts_with_url_protocol(text) => {
                let contents = scan_lexical_token(text, true);
                let html = escape(contents).to_string();
                ret_val.push_str("<a href=\"");
                ret_val.push_str(&html);
                ret_val.push_str("\">");
                ret_val.push_str(&html);
                ret_val.push_str("</a>");
                text = &text[contents.len()..];
            }
            b'w' if text.starts_with("www.") => {
                let contents = scan_lexical_token(text, true);
                let html = escape(contents).to_string();
                ret_val.push_str("<a href=\"https://");
                ret_val.push_str(&html);
                ret_val.push_str("\">");
                ret_val.push_str(&html);
                ret_val.push_str("</a>");
                text = &text[contents.len()..];
            }
            b' ' => {
                ret_val.push_str(" ");
                text = &text[1..];
            }
            b'\n' => {
                if text.as_bytes().get(1) == Some(&b'\n') {
                    ret_val.push_str("\n\n<p>");
                    text = &text[2..];
                } else {
                    ret_val.push_str("\n");
                    text = &text[1..];
                }
            }
            _ => {
                let mut i = 1;
                fn is_normal(c: u8) -> bool {
                    match c {
                        b'<' | b'@' | b'#' | b' ' | b'\n' | b'*' | b'(' => false,
                        _ => true,
                    }
                }
                while text.as_bytes().get(i).cloned().map(is_normal).unwrap_or(false) {
                    if text.is_char_boundary(i) && (starts_with_url_protocol(&text[i..]) || text[i..].starts_with("www.")) {
                        break;
                    }
                    i += 1;
                }
                ret_val.push_str(&escape(&text[..i]).to_string());
                text = &text[i..];
            }
        }
    }
    ret_val.string = CLEANER.clean(&ret_val.string).to_string();
    ret_val
}

/// Prettify a title line: similar to `prettify_body`, but without paragraph breaks
pub fn prettify_title<D: Data>(mut text: &str, url: &str, data: &mut D) -> Output {
    let mut ret_val = Output::with_capacity(text.len());
    let link = format!("<a class=article-header-inner href=\"{}\">", url);
    ret_val.push_str(&link);
    while let Some(c) = text.as_bytes().get(0) {
        match c {
            b'<' => {
                let (contents, brackets_count, count) = scan_angle_brackets(text);
                assert_ne!(brackets_count, 0);
                for _ in 0..brackets_count {
                    ret_val.push_str("&lt;");
                }
                if contents.starts_with('@') {
                    maybe_write_username(&contents[1..], data, &mut ret_val, Some(&link));
                } else if contents.starts_with('#') {
                    maybe_write_number_sign(&contents[1..], data, &mut ret_val, Some(&link));
                } else {
                    ret_val.push_str(&escape(&text[brackets_count..count]).to_string());
                }
                if contents != "" {
                    for _ in 0..brackets_count {
                        ret_val.push_str("&gt;");
                    }
                    text = &text[brackets_count+count..];
                } else {
                    text = &text[brackets_count..];
                }
            }
            b'@' => {
                let contents = scan_lexical_token(&text[1..], false);
                maybe_write_username(contents, data, &mut ret_val, Some(&link));
                text = &text[(1 + contents.len())..];
            }
            b'#' => {
                let contents = scan_lexical_token(&text[1..], false);
                maybe_write_number_sign(contents, data, &mut ret_val, Some(&link));
                text = &text[(1 + contents.len())..];
            }
            b' ' => {
                ret_val.push_str(" ");
                text = &text[1..];
            }
            _ => {
                let mut i = 1;
                fn is_normal(c: u8) -> bool {
                    match c {
                        b'<' | b'@' | b'#' | b' ' | b'\n' | b'*' | b'(' => false,
                        _ => true,
                    }
                }
                while text.as_bytes().get(i).cloned().map(is_normal).unwrap_or(false) {
                    if text.is_char_boundary(i) && (starts_with_url_protocol(&text[i..]) || text[i..].starts_with("www.")) {
                        break;
                    }
                    i += 1;
                }
                ret_val.push_str(&escape(&text[..i]).to_string());
                text = &text[i..];
            }
        }
    }
    if ret_val.string.ends_with(&link) {
        ret_val.string.truncate(ret_val.string.len() - link.len());
    } else {
        ret_val.push_str("</a>");
    }
    ret_val.string = CLEANER.clean(&ret_val.string).to_string();
    ret_val
}

fn maybe_write_username<D: Data>(username_without_at: &str, data: &mut D, out: &mut Output, embedded: Option<&str>) {
    let html = escape(&username_without_at).to_string();
    if data.check_username(username_without_at) {
        out.usernames.push(username_without_at.to_owned());
        if embedded.is_some() {
            out.push_str("</a><a class=\"inner-link article-header-inner\" href=\"@");
        } else {
            out.push_str("<a href=\"@");
        }
        out.push_str(&html);
        out.push_str("\">@");
        out.push_str(&html);
        out.push_str("</a>");
        if let Some(embedded) = embedded {
            out.push_str(embedded);
        }
    } else {
        out.push_str("@");
        out.push_str(&html);
    }
}

fn starts_with_url_protocol(s: &str) -> bool {
    for p in URL_PROTOCOLS {
        if s.starts_with(p) { return true }
    }
    false
}

fn maybe_write_number_sign<D: Data>(number_without_sign: &str, data: &mut D, out: &mut Output, embedded: Option<&str>) {
    let html = escape(number_without_sign).to_string();
    match data.check_number_sign(number_without_sign) {
        NumberSign::Tag(tag) => {
            out.hash_tags.push(tag.to_owned());
            if embedded.is_some() {
                out.push_str("</a><a class=\"inner-link article-header-inner\" href=\"/?tag=");
            } else {
                out.push_str("<a href=\"/?tag=");
            }
            out.push_str(&html);
            out.push_str("\">#");
            out.push_str(&html);
            out.push_str("</a>");
            if let Some(embedded) = embedded {
                out.push_str(embedded);
            }
        }
        NumberSign::Comment(id) => {
            out.comment_refs.push(id);
            if embedded.is_some() {
                out.push_str("</a><a class=\"inner-link article-header-inner\" href=\"#");
            } else {
                out.push_str("<a href=\"#");
            }
            out.push_str(&html);
            out.push_str("\">#");
            out.push_str(&html);
            out.push_str("</a>");
            if let Some(embedded) = embedded {
                out.push_str(embedded);
            }
        }
        NumberSign::None => {
            out.push_str("#");
            out.push_str(&html);
        }
    }
}

/// Given a string that starts with `<`, return the contents wrapped in angle brackets.
/// This function will return the empty string if it can't match them.
/// It also returns the empty string if the results are `<>`, but that's fine since that's
/// not a valid syntactic construct either.
fn scan_angle_brackets(input: &str) -> (&str, usize, usize) {
    assert_eq!(input.as_bytes().get(0), Some(&b'<'));
    let mut brackets_count = 1;
    loop {
        match input.as_bytes().get(brackets_count) {
            Some(&b'<') => brackets_count += 1,
            Some(&b'\n') | Some(&b' ') | None => return ("", brackets_count, brackets_count),
            _ => break,
        }
    }
    let mut characters_count = 0;
    'main: loop {
        match input.as_bytes().get(brackets_count + characters_count) {
            Some(&b'>') => {
                for i in 0..brackets_count {
                    match input.as_bytes().get(brackets_count + characters_count + i) {
                        Some(&b'>') => {},
                        _ => {
                            characters_count += i;
                            continue 'main;
                        },
                    }
                }
                break 'main;
            },
            Some(&b'\n') | Some(&b' ') | None => return ("", brackets_count, brackets_count),
            _ => characters_count += 1,
        }
    }
    (&input[brackets_count..characters_count+brackets_count], brackets_count, brackets_count + characters_count)
}

/// If we're at the beginning of a syntactical construct, such as a URL or an @mention,
/// find the rest of it. These are heuristics, but they should usually be accurate.
fn scan_lexical_token(input: &str, is_url: bool) -> &str {
    let mut count = 0;
    let mut stack = vec![];
    let mut bytes = input.bytes().peekable();
    while let Some(c) = bytes.next() {
        match c {
            b' ' | b'\n' | b'<' => break,
            b'#' | b'@' if !is_url => break,
            b'(' | b'[' => stack.push(c),
            b')' => {
                if stack.last() == Some(&b'(') {
                    stack.pop();
                } else {
                    match bytes.peek() {
                        Some(b' ') | Some(b'\n') | Some(b'<') | None => break,
                        _ => (),
                    }
                }
            }
            b']' => {
                if stack.last() == Some(&b'[') {
                    stack.pop();
                } else {
                    match bytes.peek() {
                        Some(b' ') | Some(b'\n') | Some(b'<') | None => break,
                        _ => (),
                    }
                }
            }
            b'.' | b',' | b'?' | b'\'' | b'"' | b'!' | b':' | b'*' => {
                match bytes.peek() {
                    Some(&b'\n') | Some(&b' ') | Some(&b'<') | Some(&b'.') | Some(&b',') | Some(&b'?') | Some(&b'\'') | Some(&b'"') | Some(&b'!') | Some(&b':') | Some(&b'*') | None => break,
                    _ => (),
                }
            }
            _ => {}
        }
        count += 1;
    }
    &input[..count]
}

pub enum NumberSign<'a> {
    Comment(i32),
    Tag(&'a str),
    None,
}

pub trait Data {
    fn check_comment_ref(&mut self, id: i32) -> bool;
    fn check_hash_tag(&mut self, tag: &str) -> bool;
    fn check_username(&mut self, username: &str) -> bool;
    fn check_number_sign<'a>(&mut self, number: &'a str) -> NumberSign<'a> {
        let id: Option<i32> = number.parse().ok();
        if let Some(id) = id {
            if self.check_comment_ref(id) {
                NumberSign::Comment(id)
            } else {
                NumberSign::None
            }
        } else {
            if self.check_hash_tag(number) {
                NumberSign::Tag(number)
            } else {
                NumberSign::None
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Output {
    pub string: String,
    pub usernames: Vec<String>,
    pub hash_tags: Vec<String>,
    pub comment_refs: Vec<i32>,
}

impl Output {
    pub fn with_capacity(cap: usize) -> Output {
        Output {
            string: String::with_capacity(cap),
            usernames: Vec::new(),
            hash_tags: Vec::new(),
            comment_refs: Vec::new(),
        }
    }
    pub fn push_str(&mut self, s: &str) {
        self.string.push_str(s);
    }
}

impl Display for Output {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        self.string.fmt(f)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_scan_angle_brackets() {
        let checks = &[
            ("<chk", ("", 1, 1)),
            ("<chk>", ("chk", 1, 4)),
            ("<chk>>", ("chk", 1, 4)),
            ("<<chk>>", ("chk", 2, 5)),
            ("<<ch>k>>", ("ch>k", 2, 6)),
            ("<>", ("", 1, 1)),
            ("<www.com>", ("www.com", 1, 8)),
            ("<test mess>", ("", 1, 1)),
            ("<<test mess>", ("", 2, 2)),
        ][..];
        for &(input, expected) in checks {
            assert_eq!(scan_angle_brackets(input), expected)
        }
    }
    #[test]
    fn test_scan_lexical_token_url() {
        let checks = &[
            ("www.com", "www.com"),
            ("www.com.", "www.com"),
            ("www.com/help", "www.com/help"),
            ("www.com/help.html.", "www.com/help.html"),
            ("www.com/Ace_(hardware)", "www.com/Ace_(hardware)"),
            ("www.com/Ace)", "www.com/Ace"),
            ("www.com/Ace)_hardware", "www.com/Ace)_hardware"),
            ("www.com/Ace_[hardware]", "www.com/Ace_[hardware]"),
            ("www.com/Ace]", "www.com/Ace"),
            ("www.com/Ace]_hardware", "www.com/Ace]_hardware"),
            ("ace hardware", "ace"),
            ("ace<http://www.com>", "ace"),
        ][..];
        for &(input, expected) in checks {
            assert_eq!(scan_lexical_token(input, true), expected);
        }
    }
    #[test]
    fn test_example_title() {
        let title = "this is a #test for #words here";

        struct MyData;
        impl Data for MyData {
            fn check_comment_ref(&mut self, id: i32) -> bool {
                id == 12345
            }
            fn check_hash_tag(&mut self, tag: &str) -> bool {
                tag == "words"
            }
            fn check_username(&mut self, username: &str) -> bool {
                username == "mentioning"
            }
        }

        let html = "<a class=article-header-inner href=\"url\">this is a #test for </a><a class=\"inner-link article-header-inner\" href=\"/?tag=words\">#words</a><a class=article-header-inner href=\"url\"> here</a>";

        assert_eq!(prettify_title(title, "url", &mut MyData).string, CLEANER.clean(html).to_string());
    }
    #[test]
    fn test_windows_newlines() {
        let comment = "test\r\n\r\npost";
        let html = "<p>test\n\n<p>post";
        struct MyData;
        impl Data for MyData {
            fn check_comment_ref(&mut self, id: i32) -> bool {
                id == 12345
            }
            fn check_hash_tag(&mut self, tag: &str) -> bool {
                tag == "words"
            }
            fn check_username(&mut self, username: &str) -> bool {
                username == "mentioning"
            }
        }
        assert_eq!(prettify_body(comment, &mut MyData).string, CLEANER.clean(html).to_string());
    }
    #[test]
    fn test_multiple_brackets() {
        let comment = "<<<http://example.com>>>";
        let html = "<p>&lt;&lt;&lt;<a href=\"http://example.com\">http://example.com</a>&gt;&gt;&gt;";
        struct MyData;
        impl Data for MyData {
            fn check_comment_ref(&mut self, id: i32) -> bool {
                id == 12345
            }
            fn check_hash_tag(&mut self, tag: &str) -> bool {
                tag == "words"
            }
            fn check_username(&mut self, username: &str) -> bool {
                username == "mentioning"
            }
        }
        assert_eq!(prettify_body(comment, &mut MyData).string, CLEANER.clean(html).to_string());
    }
    #[test]
    fn test_unicode() {
        let comment = "finger— inciting the two officers to fire";
        let html = "<p>finger— inciting the two officers to fire";
        struct MyData;
        impl Data for MyData {
            fn check_comment_ref(&mut self, id: i32) -> bool {
                id == 12345
            }
            fn check_hash_tag(&mut self, tag: &str) -> bool {
                tag == "words"
            }
            fn check_username(&mut self, username: &str) -> bool {
                username == "mentioning"
            }
        }
        assert_eq!(prettify_body(comment, &mut MyData).string, CLEANER.clean(html).to_string());
    }
    #[test]
    fn test_unicode_title() {
        let comment = "finger— inciting the two officers to fire";
        let html = "<a class=article-header-inner href=\"url\">finger— inciting the two officers to fire</a>";
        struct MyData;
        impl Data for MyData {
            fn check_comment_ref(&mut self, id: i32) -> bool {
                id == 12345
            }
            fn check_hash_tag(&mut self, tag: &str) -> bool {
                tag == "words"
            }
            fn check_username(&mut self, username: &str) -> bool {
                username == "mentioning"
            }
        }
        assert_eq!(prettify_title(comment, "url", &mut MyData).string, CLEANER.clean(html).to_string());
    }
    #[test]
    fn test_ends_with_hash_title() {
        let comment = "finger— inciting the two officers to fire #words";
        let html = "<a class=article-header-inner href=\"url\">finger— inciting the two officers to fire </a><a class=\"inner-link article-header-inner\" href=\"/?tag=words\">#words</a>";
        struct MyData;
        impl Data for MyData {
            fn check_comment_ref(&mut self, id: i32) -> bool {
                id == 12345
            }
            fn check_hash_tag(&mut self, tag: &str) -> bool {
                tag == "words"
            }
            fn check_username(&mut self, username: &str) -> bool {
                username == "mentioning"
            }
        }
        assert_eq!(prettify_title(comment, "url", &mut MyData).string, CLEANER.clean(html).to_string());
    }
    #[test]
    fn test_example() {
let comment = r####"Write my comment here.

@mentioning someone will ping them, just like on Twitter.

#12345 numbers will link to another comment on the same post.

#words are hash tags, just like on Twitter.

Consecutive line breaks are paragraph breaks, like in Markdown.

URL's are automatically linked, following similar rules to GitHub-flavored MD.
<URL> also works if your URL is too complex, but note that the angle brackets
will still be shown! It also includes GitHub's WWW special case, like
www.example.com, <www.example.com>, http://example.com, and <http://example.com>
will all get turned into links, but plain example.com will not."####;

let html = r####"<p>Write my comment here.

<p><a href="@mentioning">@mentioning</a> someone will ping them, just like on Twitter.

<p><a href="#12345">#12345</a> numbers will link to another comment on the same post.

<p><a href="/?tag=words">#words</a> are hash tags, just like on Twitter.

<p>Consecutive line breaks are paragraph breaks, like in Markdown.

<p>URL's are automatically linked, following similar rules to GitHub-flavored MD.
&lt;URL&gt; also works if your URL is too complex, but note that the angle brackets
will still be shown! It also includes GitHub's WWW special case, like
<a href="https://www.example.com">www.example.com</a>, &lt;<a href="https://www.example.com">www.example.com</a>&gt;, <a href="http://example.com">http://example.com</a>, and &lt;<a href="http://example.com">http://example.com</a>&gt;
will all get turned into links, but plain example.com will not."####;

struct MyData;
impl Data for MyData {
fn check_comment_ref(&mut self, id: i32) -> bool {
 id == 12345
}
fn check_hash_tag(&mut self, tag: &str) -> bool {
 tag == "words"
}
fn check_username(&mut self, username: &str) -> bool {
 username == "mentioning"
}
}

assert_eq!(prettify_body(comment, &mut MyData).string, CLEANER.clean(html).to_string());
    }
}
