#![forbid(unsafe_code)]

use std::str::FromStr;
use sqlx::{Pool, Postgres};
use sqlx::postgres::PgPoolOptions;

use log::{info, error};
use crate::RUNTIME_CTX;
use crate::utils::errors::Errors;
use crate::utils::config::TMS_DIRS;

// Database constants.
const POOL_MIN_CONNECTIONS: u32 = 2;
const POOL_MAX_CONNECTIONS: u32 = 8;

// ---------------------------------------------------------------------------
// init_db:
// ---------------------------------------------------------------------------
// See migrations directory for database schema definition.
pub async fn init_db(url: &str) -> Pool<Postgres> {

    // Create the DB connection pool
    let db :Pool<Postgres> = PgPoolOptions::new()
        .min_connections(POOL_MIN_CONNECTIONS).max_connections(POOL_MAX_CONNECTIONS)
        .connect(url).await.expect("Failed to connect to Postgres");

    // Locate the migration files.
    let tdir = &TMS_DIRS.migrations_dir;
    let migrations = std::path::Path::new(tdir);

    // Run the migrations.
    let migration_results = sqlx::migrate::Migrator::new(migrations)
        .await
        .expect("Migration failed")
        .run(&db)
        .await;
    match migration_results {
        Ok(_) => info!("Migration success"),
        Err(error) => {
            panic!("Migration run error: {}", error);
        }
    }
    info!("migration: {:?}", migration_results);
    db
}
