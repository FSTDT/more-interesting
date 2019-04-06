CREATE TABLE moderation (
  id SERIAL PRIMARY KEY,
  payload JSONB NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT NOW(),
  created_by INTEGER NOT NULL REFERENCES users(id)
);

ALTER TABLE posts
  ADD COLUMN updated_at TIMESTAMP NOT NULL DEFAULT NOW();

ALTER TABLE posts
  ADD COLUMN rejected BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE comments
  ADD COLUMN updated_at TIMESTAMP NOT NULL DEFAULT NOW();

ALTER TABLE comments
  ADD COLUMN rejected BOOLEAN NOT NULL DEFAULT FALSE;

SELECT diesel_manage_updated_at('posts');

SELECT diesel_manage_updated_at('comments');
