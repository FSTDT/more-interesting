CREATE TABLE "comment_readpoints" (
  user_id INTEGER NOT NULL REFERENCES users(id),
  post_id INTEGER NOT NULL REFERENCES posts(id),
  comment_readpoint INTEGER NOT NULL,
  PRIMARY KEY (user_id, post_id)
);
