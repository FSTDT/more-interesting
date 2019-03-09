CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE users (
  id SERIAL PRIMARY KEY,
  hardbanned BOOLEAN NOT NULL DEFAULT 'f',
  shadowbanned BOOLEAN NOT NULL DEFAULT 'f',
  username VARCHAR NOT NULL,
  password_hash BYTEA NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE TABLE posts (
  -- This ID exists for use in foreign keys only,
  -- since it's small and very fast to look up.
  id SERIAL PRIMARY KEY,
  -- This UUID is what should actually be exposed through the URL and web interface.
  -- Since it's random, users can't guess the URLs of hidden posts.
  uuid UUID NOT NULL DEFAULT gen_random_uuid(),
  title VARCHAR NOT NULL,
  url VARCHAR NULL,
  visible BOOLEAN NOT NULL DEFAULT 't',
  score INTEGER NOT NULL DEFAULT 0,
  created_at TIMESTAMP NOT NULL DEFAULT NOW(),
  submitted_by INTEGER NOT NULL REFERENCES users(id)
);

CREATE TABLE stars (
  user_id INTEGER NOT NULL REFERENCES users(id),
  post_id INTEGER NOT NULL REFERENCES posts(id),
  created_at TIMESTAMP NOT NULL DEFAULT NOW(),
  PRIMARY KEY (user_id, post_id)
);

CREATE INDEX idx_stars_user ON stars (user_id);
CREATE INDEX idx_stars_post ON stars (post_id);
CREATE UNIQUE INDEX idx_users_username ON users (username);
CREATE UNIQUE INDEX idx_posts_uuid ON posts (uuid);
