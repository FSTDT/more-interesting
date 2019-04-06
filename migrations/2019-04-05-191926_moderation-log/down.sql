DROP TABLE moderation;

DROP TRIGGER set_updated_at ON posts;
DROP TRIGGER set_updated_at ON comments;

ALTER TABLE posts
  DROP COLUMN updated_at;

ALTER TABLE comments
  DROP COLUMN updated_at;

ALTER TABLE posts
  DROP COLUMN rejected;

ALTER TABLE comments
  DROP COLUMN rejected;
