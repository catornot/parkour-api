pub mod log;
pub mod map;
mod persistence;
pub mod route;
mod scoreboard;
mod scores;
pub mod slug;

use map::Maps;
use parking_lot::{Mutex, RwLock};
use persistence::{init_db, load_state, start_save_cron};
use route::MapRoutes;
use rusqlite::Connection;
use std::{env, sync::Arc};
use warp::Filter;

#[derive(Clone)]
pub struct Store {
    maps_list: Arc<RwLock<Maps>>,
    routes_list: Arc<RwLock<MapRoutes>>,
    db: Arc<Mutex<Connection>>,
}

impl Store {
    fn new(db: Connection) -> Self {
        Store {
            maps_list: Arc::new(RwLock::new(Vec::new())),
            routes_list: Arc::new(RwLock::new(std::collections::HashMap::new())),
            db: Arc::new(Mutex::new(db)),
        }
    }
}

#[tokio::main]
async fn main() {
    // Secret key
    let secret = match env::var("PARKOUR_API_SECRET") {
        Ok(s) => s,
        Err(err) => {
            log::error(&format!("No secret was found, exiting [{}].", err));
            std::process::exit(1);
        }
    };

    // Authentication control
    let header_value = Box::leak(secret.into_boxed_str());
    let accept_requests = warp::header::exact("authentication", header_value);

    let db = init_db();
    let store = Store::new(db);

    load_state(store.clone());
    start_save_cron(store.clone());

    // Routes
    let map_routes = map::get_routes(store.clone());
    let score_routes = scores::get_routes(store.clone());
    let map_route_routes = route::get_routes(store.clone());
    let scoreboard_route = scoreboard::get_routes(store.clone());

    let api_routes = map_routes.or(score_routes).or(map_route_routes);
    let api_routes = accept_requests.and(api_routes);

    let admin_page = warp::path("admin")
        .and(warp::path::end())
        .and(warp::fs::file("admin/admin.html"));

    let routes = api_routes.or(scoreboard_route).or(admin_page);
    warp::serve(routes).run(([0, 0, 0, 0], 3031)).await;
}
