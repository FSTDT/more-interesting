CREATE TABLE tags (
  id SERIAL PRIMARY KEY,
  name VARCHAR NOT NULL,
  description VARCHAR NULL,
  CHECK (name ~ '^[^#@\s]+$' AND name <> ''),
  created_at TIMESTAMP NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

SELECT diesel_manage_updated_at('tags');

CREATE TABLE post_tagging (
  post_id INTEGER NOT NULL REFERENCES posts(id),
  tag_id INTEGER NOT NULL REFERENCES tags(id),
  PRIMARY KEY (post_id, tag_id)
);

CREATE UNIQUE INDEX idx_tags_name ON tags (name);

UPDATE users SET trust_level = 4 WHERE id = 1;
