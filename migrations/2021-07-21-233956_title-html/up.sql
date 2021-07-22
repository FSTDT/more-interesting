ALTER TABLE posts
  ADD COLUMN title_html VARCHAR NULL DEFAULT NULL;
UPDATE posts SET excerpt_html = NULL WHERE created_at < '2021-01-01';
