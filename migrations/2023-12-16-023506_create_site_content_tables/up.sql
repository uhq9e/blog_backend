CREATE TABLE novels (
    id SERIAL PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT NULL,
    object_id INTEGER NULL REFERENCES site_storage(id) ON DELETE SET NULL,
    created_by INTEGER NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);
