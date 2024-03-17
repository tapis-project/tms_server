use sqlx::{migrate::MigrateDatabase, Sqlite, Pool};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::str::FromStr;

use log::{info, error};
use crate::utils::errors::Errors;

// Database constants.
const DB_URL: &str = "sqlite://tms.db";
const POOL_MIN_CONNECTIONS: u32 = 2;
const POOL_MAX_CONNECTIONS: u32 = 8;

// EXAMPLE row 
// #[derive(Clone, FromRow, Debug)]
// struct TmsAppRow {
//     app_name: String,
//     client_id: String,
//     client_secret: String,
// }

pub async fn init_db() -> Pool<Sqlite> {

    if !Sqlite::database_exists(DB_URL).await.unwrap_or(false) {
        info!("Creating database {}", DB_URL);
        match Sqlite::create_database(DB_URL).await {
            Ok(_) => info!("Create db success"),
            Err(error) => {
                //let msg = format!("{}\n   {}", Errors::TOMLParseError(config_file_abs), e);
                let msg = Errors::TMSError(format!("database {} create error: {}", DB_URL, error));
                error!("{}", msg);
                panic!("{}", msg);
            }
        }
    } else {
        info!("Database already exists");
    }

    // The synchronous setting 3 means EXTRA, the strongest durability setting.
    // The automatic index setting avoids temporary index creation on a connection
    // when sqlite thinks one would be useful.  Instead, know your usage patterns!
    //
    // Update: It doesn't seem to matter what value we set in the pragmas, the database
    // seems to be created with compiled-in defaults.  Setting the env variables below
    // when doing "cargo clean;cargo build" does not change things either.  There must 
    // be another way to affect the compile options for libsqlite3-sys.
    //    SQLITE_DEFAULT_AUTOMATIC_INDEX=0
    //    SQLITE_DEFAULT_SYNCHRONOUS=3
    //    SQLITE_DEFAULT_WAL_SYNCHRONOUS=3
    let options = SqliteConnectOptions::from_str(DB_URL)
        .expect("Unable to create connection db options")
        .journal_mode(SqliteJournalMode::Wal)
        .pragma("automatic_index", "0")
        .pragma("synchronous", "3")
        .foreign_keys(true);
        
    // Create the database connection pool.    
    let db = SqlitePoolOptions::new()
        .min_connections(POOL_MIN_CONNECTIONS)
        .max_connections(POOL_MAX_CONNECTIONS)
        .connect_with(options).await
        .expect("Unable to create connection db");

    // Locate the migration files.
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").expect("No manifest directory");
    let migrations = std::path::Path::new(&crate_dir).join("./migrations");

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
    return db;

}
