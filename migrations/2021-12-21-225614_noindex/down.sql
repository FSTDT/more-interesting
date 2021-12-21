ALTER TABLE posts
  DROP COLUMN noindex;
ALTER TABLE posts
  ADD COLUMN authored_by_submitter BOOL NOT NULL DEFAULT 'f';