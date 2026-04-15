use std::env;
use std::fs::create_dir_all;
use std::io::prelude::*;
use std::{fs::File, thread, time::Duration};

use rusqlite::Connection;

use crate::route::MapRoutes;
use crate::{Store, log};

const ROUTES_FILE: &str = "data/routes.json";
const DB_FILE: &str = "data/scores.db";

/// Opens (or creates) the SQLite database and runs schema migrations.
pub fn init_db() -> Connection {
    create_dir_all("data").expect("Failed to create data directory");
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
            PRIMARY KEY (map_name, route_slug, uid),
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
pub fn start_save_cron(store: Store) {
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

    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(cron_interval_minutes * 60));

            match create_dir_all("data") {
                Ok(_) => (),
                Err(err) => {
                    log::error(&format!("Failed creating data directory [{}].", err));
                    std::process::exit(3);
                }
            };

            let routes = store.routes_list.read().clone();
            let mut buffer = match File::create(ROUTES_FILE) {
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
            match buffer.write_all(str.as_bytes()) {
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
pub fn load_state(store: Store) {
    let mut file = match File::open(ROUTES_FILE) {
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
    match file.read_to_string(&mut data) {
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

    let mut routes_lock = store.routes_list.write();
    let mut maps_lock = store.maps_list.write();
    for (map_name, routes) in serialized {
        maps_lock.push(map_name.clone());
        routes_lock.insert(map_name, routes);
    }

    log::info(&format!(
        "Loaded routes list from \"{}\" file.",
        ROUTES_FILE
    ));
}
