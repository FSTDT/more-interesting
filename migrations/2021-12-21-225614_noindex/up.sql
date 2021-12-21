ALTER TABLE posts
  ADD COLUMN noindex BOOL NOT NULL DEFAULT 'f';
ALTER TABLE posts
  DROP COLUMN authored_by_submitter;