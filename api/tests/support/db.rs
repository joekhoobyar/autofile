use autofile_api::run_migrations;
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::{AsyncDieselConnectionManager, bb8};
use testcontainers::ContainerAsync;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

pub struct TestDatabase {
    _container: ContainerAsync<Postgres>,
    pub pool: bb8::Pool<AsyncPgConnection>,
}

impl TestDatabase {
    pub async fn new() -> Self {
        let container = Postgres::default()
            .start()
            .await
            .expect("postgres should start");
        let port = container
            .get_host_port_ipv4(5432)
            .await
            .expect("postgres port should be mapped");
        let database_url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");

        run_migrations(&database_url).await;

        let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url.clone());
        let pool = bb8::Pool::builder()
            .build(config)
            .await
            .expect("pool should build");

        Self {
            _container: container,
            pool,
        }
    }
}
