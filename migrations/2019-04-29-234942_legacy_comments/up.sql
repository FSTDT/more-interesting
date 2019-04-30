CREATE TABLE legacy_comments (
    id SERIAL PRIMARY KEY,
    post_id INTEGER NOT NULL REFERENCES posts(id),
    author VARCHAR NOT NULL,
    text VARCHAR NOT NULL,
    html VARCHAR NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_legacy_comments ON legacy_comments(post_id);

SELECT diesel_manage_updated_at('legacy_comments');
