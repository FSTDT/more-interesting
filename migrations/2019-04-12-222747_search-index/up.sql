CREATE TABLE post_search_index
(
  post_id INTEGER PRIMARY KEY REFERENCES posts(id),
  search_index TSVECTOR NOT NULL
);

CREATE FUNCTION add_post_index() RETURNS trigger AS $emp_stamp$
BEGIN
  DELETE FROM post_search_index WHERE post_id = NEW.id;
  INSERT INTO post_search_index (post_id, search_index)
    VALUES (NEW.id, to_tsvector(CONCAT(NEW.title, ' ', NEW.excerpt)));
  RETURN NEW;
END;
$emp_stamp$ LANGUAGE plpgsql;

CREATE TRIGGER add_post_index_insert AFTER INSERT ON posts FOR EACH ROW EXECUTE PROCEDURE add_post_index();
CREATE TRIGGER add_post_index_update AFTER UPDATE OF excerpt ON posts FOR EACH ROW EXECUTE PROCEDURE add_post_index();

INSERT INTO post_search_index (post_id, search_index)
SELECT id, to_tsvector(CONCAT(title, ' ', excerpt))
FROM posts;
