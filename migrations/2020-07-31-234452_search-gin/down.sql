DROP INDEX post_search_index_gin_idx;

DROP TABLE post_word_freq;

DROP TRIGGER add_post_word_freq_insert ON posts;

DROP FUNCTION add_post_word_freq;
