// @generated automatically by Diesel CLI.

diesel::table! {
    authors (id) {
        id -> Int4,
        name -> Text,
        urls -> Nullable<Array<Nullable<Text>>>,
    }
}

diesel::table! {
    image_collections (id) {
        id -> Int4,
        description -> Nullable<Text>,
        date -> Date,
    }
}

diesel::table! {
    image_collections_image_items (id) {
        id -> Int4,
        image_collection_id -> Int4,
        image_item_id -> Int4,
    }
}

diesel::table! {
    image_items (id) {
        id -> Int4,
        urls -> Nullable<Array<Nullable<Text>>>,
        date -> Date,
        nsfw -> Bool,
        author_id -> Nullable<Int4>,
    }
}

diesel::table! {
    image_items_local_files (id) {
        id -> Int4,
        image_item_id -> Int4,
        local_file_id -> Text,
    }
}

diesel::table! {
    local_files (id) {
        id -> Text,
        file_name -> Nullable<Text>,
        path -> Text,
        created_at -> Timestamptz,
    }
}

diesel::joinable!(image_collections_image_items -> image_collections (image_collection_id));
diesel::joinable!(image_collections_image_items -> image_items (image_item_id));
diesel::joinable!(image_items -> authors (author_id));
diesel::joinable!(image_items_local_files -> image_items (image_item_id));
diesel::joinable!(image_items_local_files -> local_files (local_file_id));

diesel::allow_tables_to_appear_in_same_query!(
    authors,
    image_collections,
    image_collections_image_items,
    image_items,
    image_items_local_files,
    local_files,
);
