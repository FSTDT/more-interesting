ALTER TABLE users ADD COLUMN identicon INTEGER NOT NULL DEFAULT RANDOM() * (2^31);
