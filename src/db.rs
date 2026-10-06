use crate::config::Config;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};
use std::{fs, str::FromStr};

pub async fn init_db(config: &Config) -> anyhow::Result<SqlitePool> {
    fs::create_dir_all(&config.data_dir)?;
    let db_url = format!("sqlite://{}", config.db_path.display());
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(
            SqliteConnectOptions::from_str(&db_url)?
                .create_if_missing(true)
                .foreign_keys(true),
        )
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}
