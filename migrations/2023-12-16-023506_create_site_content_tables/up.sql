CREATE TABLE novels (
    id SERIAL PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT NULL,
    url TEXT NULL,
    author_name TEXT NOT NULL,
    author_url TEXT NULL,
    nsfw BOOLEAN NOT NULL DEFAULT FALSE,
    tags TEXT[] NOT NULL DEFAULT '{}',
    object_id INTEGER NULL REFERENCES site_storage(id) ON DELETE SET NULL,
    created_by INTEGER NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);
