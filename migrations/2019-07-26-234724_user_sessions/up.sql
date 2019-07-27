CREATE TABLE user_sessions (
  uuid BIGINT PRIMARY KEY,
  created_at TIMESTAMP NOT NULL DEFAULT NOW(),
  user_agent TEXT NOT NULL,
  user_id INTEGER NOT NULL REFERENCES users(id)
);
