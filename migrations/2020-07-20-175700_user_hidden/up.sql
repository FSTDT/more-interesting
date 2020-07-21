CREATE TABLE post_hides (
    post_id INT NOT NULL REFERENCES posts(id),
    user_id INT NOT NULL REFERENCES users(id),
    PRIMARY KEY (post_id, user_id)
);
CREATE TABLE comment_hides (
    comment_id INT NOT NULL REFERENCES comments(id),
    user_id INT NOT NULL REFERENCES users(id),
    PRIMARY KEY (comment_id, user_id)
);
