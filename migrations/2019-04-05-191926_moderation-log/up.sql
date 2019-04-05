CREATE TABLE moderation (
  id SERIAL PRIMARY KEY,
  payload JSONB NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT NOW(),
  created_by INTEGER NOT NULL REFERENCES users(id)
);

ALTER TABLE posts
  ADD COLUMN updated_at TIMESTAMP NOT NULL DEFAULT NOW();

SELECT diesel_manage_updated_at('posts');
