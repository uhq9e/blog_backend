use crate::{
    create_s3_client, db,
    models::{ImageItemLocalFile, LocalFile},
    schema,
    utils::TransactionError,
    BUCKET,
};
use aws_sdk_s3::operation::delete_object::DeleteObjectError;
use clokwerk::{AsyncScheduler, Job, TimeUnits};
use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection, RunQueryDsl};
use log::info;
use std::time::Duration;

pub async fn init(db_url: String) -> () {
    let mut scheduler = AsyncScheduler::new();

    // Clear unreferenced objects
    let db_url_a = db_url.to_owned();
    scheduler.every(1.day()).at("00:00").run(move || {
        let db_url = db_url_a.to_owned();
        let mut local_files: Vec<(LocalFile, Option<ImageItemLocalFile>)> = vec![];
        async move {
            let pool = db::establish_connection(db_url).await;
            let mut conn = pool.get().await.unwrap();
            let s3_client = create_s3_client().await;
            local_files = schema::local_files::table
                .left_join(schema::image_items_local_files::table)
                .select((
                    LocalFile::as_select(),
                    Option::<ImageItemLocalFile>::as_select(),
                ))
                .load::<(LocalFile, Option<ImageItemLocalFile>)>(&mut conn)
                .await
                .unwrap();

            let unreferenced_objects = local_files
                .iter()
                .filter(|v| v.1.is_none())
                .collect::<Vec<&(LocalFile, Option<ImageItemLocalFile>)>>();

            for unreferenced_object in unreferenced_objects {
                let s3_client = s3_client.to_owned();
                let _ = conn
                    .transaction::<(), TransactionError<DeleteObjectError>, _>(|conn| {
                        let unreferenced_object = unreferenced_object.to_owned();
                        async move {
                            diesel::delete(schema::local_files::table)
                                .filter(schema::local_files::id.eq(&unreferenced_object.0.id))
                                .execute(conn)
                                .await
                                .map_err(|err| TransactionError::ResultError(err))?;

                            s3_client
                                .delete_object()
                                .bucket(BUCKET)
                                .key(&unreferenced_object.0.path)
                                .send()
                                .await
                                .map_err(|err| TransactionError::SdkError(err))?;

                            Ok(())
                        }
                        .scope_boxed()
                    })
                    .await;
            }
        }
    });

    //Re-group image items
    let db_url_b = db_url.to_owned();
    scheduler.every(1.day()).at("00:00").run(move || {
        let db_url = db_url_b.to_owned();
        async move {
            let pool = db::establish_connection(db_url).await;
            let mut conn = pool.get().await.unwrap();

            conn.transaction::<(), diesel::result::Error, _>(|conn| {
                async move {
                    let image_items = schema::image_items::table
                        .select(schema::image_items::all_columns)
                        .load::<crate::models::ImageItem>(conn)
                        .await?;

                    diesel::delete(schema::image_items_grouped::table)
                        .execute(conn)
                        .await?;

                    diesel::insert_into(schema::image_items_grouped::table)
                        .values(
                            image_items
                                .iter()
                                .map(|i| {
                                    (
                                        schema::image_items_grouped::image_item_id.eq(i.id),
                                        schema::image_items_grouped::date.eq(i.date),
                                    )
                                })
                                .collect::<Vec<_>>(),
                        )
                        .execute(conn)
                        .await?;

                    Ok(())
                }
                .scope_boxed()
            })
            .await
            .unwrap();
        }
    });

    tokio::spawn(async move {
        loop {
            scheduler.run_pending().await;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    info!("Schedule jobs started");
}
