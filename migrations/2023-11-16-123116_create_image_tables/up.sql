CREATE TABLE authors (
    id SERIAL4 PRIMARY KEY,
    name TEXT NOT NULL,
    urls TEXT[] NULL
);

CREATE TABLE image_items (
    id SERIAL4 PRIMARY KEY,
    urls TEXT[] NULL,
    "date" date NOT NULL DEFAULT CURRENT_DATE,
    nsfw boolean NOT NULL DEFAULT false,
    author_id INT4 NULL REFERENCES authors(id) ON DELETE SET NULL
);

CREATE TABLE local_files (
    id TEXT PRIMARY KEY,
    file_name TEXT NULL,
    "path" TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE image_items_local_files (
    id SERIAL4 NOT NULL PRIMARY KEY,
    image_item_id INT4 NOT NULL REFERENCES image_items(id) ON DELETE CASCADE,
    local_file_id TEXT NOT NULL REFERENCES local_files(id) ON DELETE CASCADE
);

CREATE TABLE image_items_grouped (
    image_item_id INT4 NOT NULL PRIMARY KEY REFERENCES image_items(id) ON DELETE CASCADE,
    "date" date NOT NULL
);

