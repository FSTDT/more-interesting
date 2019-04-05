DROP TABLE moderation;

DROP TRIGGER set_updated_at ON posts;

ALTER TABLE posts
  DROP COLUMN updated_at;
