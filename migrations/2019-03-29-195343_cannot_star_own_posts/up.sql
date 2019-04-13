CREATE FUNCTION check_stars_post() RETURNS trigger AS $emp_stamp$
  DECLARE
    post_id INTEGER;
  BEGIN
  post_id := NEW.post_id;
  IF NEW.user_id = (SELECT posts.submitted_by FROM posts WHERE post_id = posts.id) THEN
    RAISE EXCEPTION 'users cannot star their own posts';
  END IF;
  RETURN NEW;
END;
$emp_stamp$ LANGUAGE plpgsql;

CREATE FUNCTION check_stars_comment() RETURNS trigger AS $emp_stamp$
  DECLARE
    comment_id INTEGER;
  BEGIN
  comment_id := NEW.comment_id;
  IF NEW.user_id = (SELECT "comments".created_by FROM "comments" WHERE comment_id = "comments".id) THEN
    RAISE EXCEPTION 'users cannot star their own posts';
  END IF;
  RETURN NEW;
END;
$emp_stamp$ LANGUAGE plpgsql;

CREATE TRIGGER no_self_stars_post BEFORE INSERT ON stars FOR EACH ROW EXECUTE PROCEDURE check_stars_post();
CREATE TRIGGER no_self_stars_comment BEFORE INSERT ON comment_stars FOR EACH ROW EXECUTE PROCEDURE check_stars_comment();
