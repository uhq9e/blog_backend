// @generated automatically by Diesel CLI.

diesel::table! {
    api_tokens (token) {
        token -> Text,
        admin -> Bool,
        created_at -> Timestamptz,
    }
}

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
    image_collections_image_items (image_collection_id, image_item_id) {
        image_collection_id -> Int4,
        image_item_id -> Int4,
    }
}

diesel::table! {
    image_items (id) {
        id -> Int4,
        date -> Date,
        author_id -> Nullable<Int4>,
    }
}

diesel::table! {
    local_files (id) {
        id -> Int4,
        file_name -> Nullable<Text>,
        path -> Text,
        created_at -> Timestamptz,
        image_item_id -> Int4,
    }
}

diesel::table! {
    social_posts (id) {
        id -> Int4,
        #[sql_name = "type"]
        type_ -> Int4,
        url -> Text,
        image_item_id -> Int4,
    }
}

diesel::joinable!(image_collections_image_items -> image_collections (image_collection_id));
diesel::joinable!(image_collections_image_items -> image_items (image_item_id));
diesel::joinable!(image_items -> authors (author_id));
diesel::joinable!(local_files -> image_items (image_item_id));
diesel::joinable!(social_posts -> image_items (image_item_id));

diesel::allow_tables_to_appear_in_same_query!(
    api_tokens,
    authors,
    image_collections,
    image_collections_image_items,
    image_items,
    local_files,
    social_posts,
);
