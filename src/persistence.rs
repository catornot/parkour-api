use rusqlite::Connection;
use std::{env, time::Duration};
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
};

use crate::route::MapRoutes;
use crate::{Store, log};

const ROUTES_FILE: &str = "data/routes.json";
const DB_FILE: &str = "data/scores.db";

/// Opens (or creates) the SQLite database and runs schema migrations.
pub async fn init_db() -> Connection {
    fs::create_dir_all("data")
        .await
        .expect("Failed to create data directory");
    let conn = Connection::open(DB_FILE).expect("Failed to open SQLite database");

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS users (
            uid  TEXT PRIMARY KEY,
            name TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS scores (
            map_name   TEXT NOT NULL,
            route_slug TEXT NOT NULL,
            uid        TEXT NOT NULL,
            time       REAL NOT NULL,
            recording TEXT NOT NULL,
            PRIMARY KEY (map_name, route_slug, uid, recording),
            FOREIGN KEY (uid) REFERENCES users(uid)
        );",
    )
    .expect("Failed to initialize database schema");

    // Migration for existing databases without the timestamp column
    let _ =
        conn.execute_batch("ALTER TABLE scores ADD COLUMN timestamp INTEGER NOT NULL DEFAULT 0;");

    conn
}

/// Starts a thread that will save store state to JSON files every few seconds.
///
/// The time between two consecutive saves is 15 minutes by default, and can be
/// customized with the `PARKOUR_API_SAVE_TIMER` environment variable.
///
/// Saves routes to JSON on a background thread at a configurable interval.
pub async fn start_save_cron(store: Store) {
    let cron_interval_minutes: u64 = match env::var("PARKOUR_API_SAVE_TIMER") {
        Ok(s) => {
            log::info(&format!("Timer argument found ({} minutes).", s));
            s.parse::<u64>().unwrap()
        }
        Err(_) => {
            log::info("No timer argument was found, defaulting to 15 minutes.");
            15
        }
    };

    tokio::task::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(cron_interval_minutes * 60)).await;

            match fs::create_dir_all("data").await {
                Ok(_) => (),
                Err(err) => {
                    log::error(&format!("Failed creating data directory [{}].", err));
                    std::process::exit(3);
                }
            };

            let routes = store.routes_list.read().await.clone();
            let mut buffer = match fs::File::create(ROUTES_FILE).await {
                Ok(file) => file,
                Err(err) => {
                    log::error(&format!(
                        "\"{}\" file could not be created [{}].",
                        ROUTES_FILE, err
                    ));
                    std::process::exit(3);
                }
            };
            let str = match serde_json::to_string(&routes) {
                Ok(str) => str,
                Err(err) => {
                    log::error(&format!("Failed serializing routes list [{}].", err));
                    std::process::exit(3);
                }
            };
            match buffer.write_all(str.as_bytes()).await {
                Ok(_) => (),
                Err(err) => {
                    log::error(&format!("Failed writing routes list to file [{}].", err));
                    std::process::exit(3);
                }
            };
            log::info("Saved routes to local file.");
        }
    });
}

/// Loads routes from JSON file into store on startup.
pub async fn load_state(store: Store) {
    let mut file = match fs::File::open(ROUTES_FILE).await {
        Ok(file) => file,
        Err(_) => {
            log::info(&format!(
                "\"{}\" file does not exist, starting with empty routes.",
                ROUTES_FILE
            ));
            return;
        }
    };

    let mut data = String::new();
    match file.read_to_string(&mut data).await {
        Ok(_) => (),
        Err(err) => {
            log::error(&format!(
                "Failed reading \"{}\" file [{}].",
                ROUTES_FILE, err
            ));
            std::process::exit(2);
        }
    };

    let serialized: MapRoutes = match serde_json::from_str::<MapRoutes>(&data) {
        Ok(data) => data,
        Err(err) => {
            log::error(&format!("Failed deserializing routes list [{}].", err));
            std::process::exit(2);
        }
    };

    let mut routes_lock = store.routes_list.write().await;
    let mut maps_lock = store.maps_list.write().await;
    for (map_name, routes) in serialized {
        maps_lock.push(map_name.clone());
        routes_lock.insert(map_name, routes);
    }

    log::info(&format!(
        "Loaded routes list from \"{}\" file.",
        ROUTES_FILE
    ));
}
