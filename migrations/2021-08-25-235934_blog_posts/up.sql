ALTER TABLE posts
  ADD COLUMN blog_post BOOL NOT NULL DEFAULT 'f';
DROP INDEX idx_posts_homepage;
CREATE INDEX idx_posts_homepage ON posts (blog_post, initial_stellar_time DESC, created_at DESC);
