CREATE TABLE blocked_regexes (
  id SERIAL PRIMARY KEY,
  regex VARCHAR NOT NULL CHECK (regex <> '')
);