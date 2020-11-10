DROP TRIGGER add_post_index_insert on posts;
DROP TRIGGER add_post_index_update on posts;
DROP FUNCTION add_post_index();

CREATE FUNCTION add_post_index() RETURNS trigger AS $emp_stamp$
BEGIN
  DELETE FROM post_search_index WHERE post_id = NEW.id;
  INSERT INTO post_search_index (post_id, search_index)
    VALUES (NEW.id, setweight(to_tsvector(coalesce(NEW.title,'')), 'A') || setweight(to_tsvector(coalesce(NEW.excerpt,'')), 'D'));
  RETURN NEW;
END;
$emp_stamp$ LANGUAGE plpgsql;

CREATE TRIGGER add_post_index_insert BEFORE INSERT ON posts FOR EACH ROW EXECUTE PROCEDURE add_post_index();
CREATE TRIGGER add_post_index_update BEFORE UPDATE OF excerpt ON posts FOR EACH ROW EXECUTE PROCEDURE add_post_index();

TRUNCATE TABLE post_search_index;

INSERT INTO post_search_index (post_id, search_index)
SELECT id, setweight(to_tsvector(coalesce(title,'')), 'A') || setweight(to_tsvector(coalesce(excerpt,'')), 'D')
FROM posts;


DROP TRIGGER add_post_word_freq_insert ON posts;
DROP FUNCTION add_post_word_freq();

CREATE FUNCTION add_post_word_freq() RETURNS trigger AS $emp_stamp$
BEGIN
  INSERT INTO post_word_freq (word, num)
  SELECT 
    word, ndoc
  FROM TS_STAT(CONCAT('SELECT setweight(to_tsvector(coalesce(title,'''')), ''A'') || setweight(to_tsvector(coalesce(excerpt,'''')), ''D'') FROM posts WHERE id = ', NEW.id))
  ON CONFLICT (word) DO UPDATE SET
    num = post_word_freq.num + 1
  ;
  RETURN NEW;
END;
$emp_stamp$ LANGUAGE plpgsql;

CREATE TRIGGER add_post_word_freq_insert BEFORE INSERT ON posts FOR EACH ROW EXECUTE PROCEDURE add_post_word_freq();

TRUNCATE TABLE post_word_freq;
INSERT INTO post_word_freq (word, num)
SELECT 
  word, ndoc AS num
FROM TS_STAT(CONCAT('SELECT setweight(to_tsvector(coalesce(title,'''')), ''A'') || setweight(to_tsvector(coalesce(excerpt,'''')), ''D'') FROM posts WHERE visible AND NOT private'))
;
