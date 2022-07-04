extern crate tempdir;

use futures::Future;
use pg_embed::pg_enums::PgAuthMethod;
use pg_embed::pg_errors::PgEmbedError;
use pg_embed::pg_fetch::{PgFetchSettings, PG_V13};
use pg_embed::postgres::{PgEmbed, PgSettings};
use std::collections::HashMap;
use std::time::Duration;
use tempdir::TempDir;
use tokio_postgres::{Client, NoTls};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DBKey(pub u32);

pub async fn run_embedded_db<A, Fut: Future<Output = A>, F: Fn(&str) -> Fut>(
    db_name: &str,
    f: F,
) -> Result<A, PgEmbedError> {
    let tmp_dir = TempDir::new("embedded_db").unwrap();

    // Postgresql settings
    let pg_settings = PgSettings {
        database_dir: tmp_dir.path().to_path_buf(),
        port: 5432,
        user: "postgres".to_string(),
        password: "password".to_string(),
        // authentication method
        auth_method: PgAuthMethod::Plain,
        persistent: false, // don't keep any data
        // duration to wait before terminating process execution
        // pg_ctl start/stop and initdb timeout
        // if set to None the process will not be terminated
        timeout: Some(Duration::from_secs(15)),
        migration_dir: None,
    };

    // Postgresql binaries download settings
    let fetch_settings = PgFetchSettings {
        version: PG_V13,
        ..Default::default()
    };

    // Create a new instance
    let mut pg = PgEmbed::new(pg_settings, fetch_settings).await?;

    // Download, unpack, create password file and database cluster
    pg.setup().await.unwrap();

    // start postgresql database
    pg.start_db().await.unwrap();

    // create a new database
    // to enable migrations view the [Usage] section for details
    pg.create_database(db_name).await.unwrap();

    let res = f(&pg.full_db_uri("database_name")).await;

    // drop a database
    // to enable migrations view [Usage] for details
    pg.drop_database(db_name).await.unwrap();

    // stop postgresql database
    pg.stop_db().await.unwrap();

    Ok(res)
}

pub struct DB {
    client: Client,
}

impl DB {
    pub async fn with_db<A, Fut: Future<Output = A>, F: Fn(Self) -> Fut>(
        conn_str: &str,
        f: F,
    ) -> Result<A, tokio_postgres::Error> {
        let (client, connection) = tokio_postgres::connect(conn_str, NoTls).await?;

        // The connection object performs the actual communication with the database,
        // so spawn it off to run on its own.
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        Ok(f(DB { client }).await)
    }

    pub async fn init(&self, state: HashMap<DBKey, i64>) -> Result<(), tokio_postgres::Error> {
        self.client
            .execute(
                "CREATE TABLE IF NOT EXISTS Entries (
                                key INT NOT NULL,
                                val INT NOT NULL,
                                PRIMARY KEY (key)
                            )",
                &[],
            )
            .await?;

        for (k, v) in state.into_iter() {
            self.put(k, v).await?;
        }

        Ok(())
    }

    pub async fn get(&self, key: DBKey) -> Result<i64, tokio_postgres::Error> {
        let res = self
            .client
            .query_one(" SELECT val FROM Entries WHERE key=$1", &[&key.0])
            .await?;

        Ok(res.get("val"))
    }

    pub async fn put(&self, key: DBKey, v: i64) -> Result<(), tokio_postgres::Error> {
        let _ = self
            .client
            .execute(
                "INSERT INTO Entries (key, val) VALUES ($1, $2)",
                &[&key.0, &v],
            )
            .await?;

        Ok(())
    }
}
