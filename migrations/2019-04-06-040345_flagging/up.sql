CREATE TABLE flags (
                     user_id INTEGER NOT NULL REFERENCES users(id),
                     post_id INTEGER NOT NULL REFERENCES posts(id),
                     created_at TIMESTAMP NOT NULL DEFAULT NOW(),
                     PRIMARY KEY (user_id, post_id)
);

CREATE TABLE comment_flags (
                     user_id INTEGER NOT NULL REFERENCES users(id),
                     comment_id INTEGER NOT NULL REFERENCES "comments"(id),
                     created_at TIMESTAMP NOT NULL DEFAULT NOW(),
                     PRIMARY KEY (user_id, comment_id)
);
