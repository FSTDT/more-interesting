CREATE INDEX post_search_index_gin_idx ON post_search_index USING gin(search_index);

CREATE TABLE post_word_freq
(
  word VARCHAR PRIMARY KEY,
  num INTEGER
);

 CREATE FUNCTION add_post_word_freq() RETURNS trigger AS $emp_stamp$
 BEGIN
   INSERT INTO post_word_freq (word, num)
   SELECT 
     word, ndoc
   FROM TS_STAT(CONCAT('SELECT to_tsvector(excerpt) FROM posts WHERE id = ', NEW.id))
   ON CONFLICT (word) DO UPDATE SET
     num = post_word_freq.num + 1
   ;
   RETURN NEW;
 END;
 $emp_stamp$ LANGUAGE plpgsql;

CREATE TRIGGER add_post_word_freq_insert BEFORE INSERT ON posts FOR EACH ROW EXECUTE PROCEDURE add_post_word_freq();

INSERT INTO post_word_freq (word, num)
SELECT 
  word, ndoc AS num
FROM TS_STAT(CONCAT('SELECT to_tsvector(excerpt) FROM posts WHERE visible AND NOT private'))
;

