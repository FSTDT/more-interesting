DROP TRIGGER add_post_index_insert ON posts;
DROP TRIGGER add_post_index_update ON posts;
CREATE TRIGGER add_post_index_insert AFTER INSERT ON posts FOR EACH ROW EXECUTE PROCEDURE add_post_index();
CREATE TRIGGER add_post_index_update AFTER UPDATE OF excerpt ON posts FOR EACH ROW EXECUTE PROCEDURE add_post_index();
