// @generated automatically by Diesel CLI.

diesel::table! {
    authors (id) {
        id -> Int4,
        name -> Text,
        urls -> Nullable<Array<Nullable<Text>>>,
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
    image_items_grouped (id) {
        id -> Int4,
        image_item_id -> Int4,
        date -> Date,
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

diesel::table! {
    novels (id) {
        id -> Int4,
        title -> Text,
        description -> Nullable<Text>,
        url -> Nullable<Text>,
        author_name -> Text,
        author_url -> Nullable<Text>,
        nsfw -> Bool,
        tags -> Array<Nullable<Text>>,
        object_id -> Nullable<Int4>,
        created_by -> Nullable<Int4>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    site_storage (id) {
        id -> Int4,
        file_name -> Text,
        key -> Text,
        size -> Int8,
        hash -> Text,
        kind -> Int2,
        mime_type -> Text,
        created_by -> Nullable<Int4>,
        created_at -> Timestamptz,
    }
}

diesel::joinable!(image_items -> authors (author_id));
diesel::joinable!(image_items_grouped -> image_items (image_item_id));
diesel::joinable!(image_items_local_files -> image_items (image_item_id));
diesel::joinable!(image_items_local_files -> local_files (local_file_id));
diesel::joinable!(novels -> site_storage (object_id));

diesel::allow_tables_to_appear_in_same_query!(
    authors,
    image_items,
    image_items_grouped,
    image_items_local_files,
    local_files,
    novels,
    site_storage,
);
