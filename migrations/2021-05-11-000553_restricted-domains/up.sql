CREATE TABLE domain_restrictions (
    domain_id INTEGER NOT NULL PRIMARY KEY REFERENCES domains(id),
    restriction_level INTEGER NOT NULL DEFAULT 0
);
