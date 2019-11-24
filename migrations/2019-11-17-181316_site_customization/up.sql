CREATE TABLE site_customization (
    name VARCHAR NOT NULL PRIMARY KEY CHECK (name <> ''),
    value VARCHAR NOT NULL
);

INSERT INTO site_customization (name, value) VALUES
    ('title', 'More Interesting'),
    ('css', ''),
    ('custom_footer_html', ''),
    ('front_notice_html', 'Welcome to our site!'),
    ('comments_placeholder_html', 'To post a comment, you''ll need to <a href=/login>Sign in</a>.'),
    ('link_submit_notice_html', ''),
    ('blog_post_notice_html', ''),
    ('message_send_notice_html', ''),
    ('post_score_text', 'has {} stars...'),
    ('comment_score_text', 'has {} stars...');
