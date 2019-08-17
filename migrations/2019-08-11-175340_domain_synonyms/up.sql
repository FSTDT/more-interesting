CREATE TABLE domain_synonyms (
    from_hostname VARCHAR NOT NULL PRIMARY KEY CHECK (from_hostname <> ''),
    to_domain_id INTEGER NOT NULL REFERENCES domains(id)
);
