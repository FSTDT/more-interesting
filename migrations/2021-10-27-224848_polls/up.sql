CREATE TABLE "polls" (
	id SERIAL PRIMARY KEY,
	post_id INTEGER NOT NULL REFERENCES posts(id),
    title TEXT NOT NULL,
    open BOOL NOT NULL DEFAULT 't',
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    created_by INTEGER NOT NULL REFERENCES users(id)
);
CREATE TABLE "poll_choices" (
	id SERIAL PRIMARY KEY,
    poll_id INTEGER NOT NULL REFERENCES polls(id),
    title TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    created_by INTEGER NOT NULL REFERENCES users(id)
);
CREATE TABLE "poll_votes" (
	id SERIAL PRIMARY KEY,
	user_id INTEGER NOT NULL REFERENCES users(id),
	choice_id INTEGER NOT NULL REFERENCES poll_choices(id),
	score INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_votes_user ON poll_votes (user_id, choice_id);
