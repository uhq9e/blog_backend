CREATE TABLE authors (
    id SERIAL4 PRIMARY KEY,
    name TEXT NOT NULL,
    urls TEXT[] NULL
);

CREATE TABLE image_items (
    id SERIAL4 PRIMARY KEY,
    "date" date NOT NULL DEFAULT CURRENT_DATE,
    author_id INT4 NULL REFERENCES authors(id) ON DELETE SET NULL
);

CREATE TABLE social_posts (
    id SERIAL4 PRIMARY KEY,
    "type" INT4 NOT NULL,
    url text NOT NULL,
    image_item_id INT4 NOT NULL REFERENCES image_items(id) ON DELETE CASCADE
);

CREATE TABLE local_files (
    id SERIAL4 PRIMARY KEY,
    file_name TEXT NULL,
    "path" TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    image_item_id INT4 NOT NULL REFERENCES image_items(id) ON DELETE CASCADE
);

CREATE TABLE image_collections (
	id SERIAL4 PRIMARY KEY,
    "description" TEXT NULL,
	"date" DATE NOT NULL DEFAULT CURRENT_DATE
);

CREATE TABLE image_collections_image_items (
    image_collection_id INT4 NOT NULL REFERENCES image_collections(id) ON DELETE CASCADE,
    image_item_id INT4 NOT NULL REFERENCES image_items(id) ON DELETE CASCADE,
    PRIMARY KEY (image_collection_id, image_item_id)
);

