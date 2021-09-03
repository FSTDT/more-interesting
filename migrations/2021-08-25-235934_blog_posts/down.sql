DROP INDEX idx_posts_homepage;

ALTER TABLE posts
  DROP COLUMN blog_post;
CREATE INDEX idx_posts_homepage ON posts (initial_stellar_time DESC, created_at DESC);
