ALTER TABLE posts ADD COLUMN banner_title VARCHAR NULL DEFAULT NULL CHECK (banner_title <> '');
ALTER TABLE posts ADD COLUMN banner_desc VARCHAR NULL DEFAULT NULL CHECK (banner_desc <> '');
