ALTER TABLE posts
  ADD COLUMN anon BOOL NOT NULL DEFAULT 'f';

UPDATE posts
  SET anon = 't'
  WHERE submitted_by = (
    SELECT id FROM users WHERE username = 'anonymous'
  );
