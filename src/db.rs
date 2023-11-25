use diesel::result::ConnectionError;
use diesel::r2d2::ConnectionManager;
use diesel_async::{RunQueryDsl, AsyncConnection, AsyncPgConnection, pooled_connection::AsyncDieselConnectionManager};
use bb8;

pub type Pool = bb8::Pool<AsyncDieselConnectionManager<AsyncPgConnection>>;

pub async fn establish_connection(database_url: String) -> Pool {
    let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
    bb8::Pool::builder()
        .max_size(15)
        .build(manager)
        .await
        .expect("创建连接池失败")
}
