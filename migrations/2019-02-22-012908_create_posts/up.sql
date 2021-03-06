CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE users (
  id SERIAL PRIMARY KEY,
  banned BOOLEAN NOT NULL DEFAULT 'f',
  -- 0=able to post, but highly rate-limited
  -- 1=looser rate limit
  -- 2=able to change titles and tags on other people's posts
  -- 3=moderator
  -- 4=admin
  -- this is a somewhat flatter hierarchy than what Discourse user,
  -- but MI is designed for smaller communities
  trust_level INTEGER NOT NULL DEFAULT 0,
  username VARCHAR NOT NULL,
  CHECK (username ~ '^[^#@\s]+$' AND username <> ''),
  password_hash BYTEA NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT NOW(),
  invited_by INTEGER NULL DEFAULT NULL REFERENCES users(id)
);

CREATE TABLE invite_tokens (
  uuid BIGINT PRIMARY KEY,
  created_at TIMESTAMP NOT NULL DEFAULT NOW(),
  invited_by INTEGER NOT NULL REFERENCES users(id)
);

CREATE TABLE posts (
  -- This ID exists for use in foreign keys only,
  -- since it's small and very fast to look up.
  id SERIAL PRIMARY KEY,
  -- This UUID is what should actually be exposed through the URL and web interface.
  -- Since it's random, users can't guess the URLs of hidden posts.
  uuid BIGINT NOT NULL,
  title VARCHAR NOT NULL,
  url VARCHAR NULL,
  visible BOOLEAN NOT NULL DEFAULT 't',
  initial_stellar_time INTEGER NOT NULL DEFAULT 0,
  score INTEGER NOT NULL DEFAULT 0,
  comment_count INTEGER NOT NULL DEFAULT 0,
  authored_by_submitter BOOLEAN NOT NULL DEFAULT 'f',
  created_at TIMESTAMP NOT NULL DEFAULT NOW(),
  submitted_by INTEGER NOT NULL REFERENCES users(id)
);

CREATE TABLE stars (
  user_id INTEGER NOT NULL REFERENCES users(id),
  post_id INTEGER NOT NULL REFERENCES posts(id),
  created_at TIMESTAMP NOT NULL DEFAULT NOW(),
  PRIMARY KEY (user_id, post_id)
);

CREATE TABLE "comments" (
  id SERIAL PRIMARY KEY,
  text VARCHAR NOT NULL,
  html VARCHAR NOT NULL,
  visible BOOLEAN NOT NULL DEFAULT 't',
  post_id INTEGER NOT NULL REFERENCES posts(id),
  created_at TIMESTAMP NOT NULL DEFAULT NOW(),
  created_by INTEGER NOT NULL REFERENCES users(id)
);

CREATE TABLE comment_stars (
  user_id INTEGER NOT NULL REFERENCES users(id),
  comment_id INTEGER NOT NULL REFERENCES "comments"(id),
  created_at TIMESTAMP NOT NULL DEFAULT NOW(),
  PRIMARY KEY (user_id, comment_id)
);

CREATE INDEX idx_stars_user ON stars (user_id);
CREATE INDEX idx_stars_post ON stars (post_id);
CREATE INDEX idx_comment_stars_user ON comment_stars (user_id);
CREATE INDEX idx_comment_stars_reply ON comment_stars (comment_id);
CREATE UNIQUE INDEX idx_users_username ON users (username);
CREATE UNIQUE INDEX idx_posts_uuid ON posts (uuid);
