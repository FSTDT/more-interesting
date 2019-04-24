CREATE TABLE domains (
  id SERIAL PRIMARY KEY,
  banned BOOLEAN NOT NULL DEFAULT 'f',
  hostname VARCHAR NOT NULL CHECK (hostname <> ''),
  is_www BOOLEAN NOT NULL DEFAULT 'f',
  is_https BOOLEAN NOT NULL DEFAULT 'f'
);

CREATE INDEX idx_domains_name ON domains(hostname);

ALTER TABLE posts ADD COLUMN domain_id INTEGER NULL DEFAULT NULL REFERENCES domains(id);
