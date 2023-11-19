use diesel::pg::PgConnection;
use diesel::r2d2::ConnectionManager;
use std::env;

pub type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;

pub fn establish_connection() -> Pool {
    let database_url = env::var("DATABASE_URL").expect("必须设置DATABASE_URL");

    let manager = ConnectionManager::<PgConnection>::new(database_url);
    r2d2::Pool::builder()
        .build(manager)
        .expect("创建连接池失败")
}
