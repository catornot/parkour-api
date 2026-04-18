use std::collections::HashMap;
use uuid::Uuid;
use warp::{hyper::StatusCode, Filter, Rejection, Reply};

use crate::Store;
use serde::{Deserialize, Serialize};

pub type MapRoutes = HashMap<String, Vec<MapRoute>>;

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Line {
    #[serde(serialize_with = "serialize_vector")]
    #[serde(deserialize_with = "deserialize_vector")]
    origin: [f64; 3],
    #[serde(serialize_with = "serialize_vector")]
    #[serde(deserialize_with = "deserialize_vector")]
    angles: [i64; 3],
    dimensions: [i64; 2],
    #[serde(serialize_with = "serialize_vector")]
    #[serde(deserialize_with = "deserialize_vector")]
    trigger: [[f64; 3]; 2],
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct RouteName {
    origin: [f64; 3],
    angles: [i64; 3],
    dimensions: [i64; 2],
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct LeaderboardSource {
    origin: [f64; 3],
    angles: [i64; 3],
    dimensions: [i64; 2],
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Leaderboard {
    origin: [f64; 3],
    angles: [i64; 3],
    dimensions: [i64; 2],
    source: LeaderboardSource,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Leaderboards {
    local: Leaderboard,
    world: Leaderboard,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct StartPosition {
    origin: [f64; 3],
    angles: [i64; 3],
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct EndPosition {
    origin: [f64; 3],
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Robot {
    origin: [f64; 3],
    angles: [i64; 3],
    talkable_radius: i64,
    animation: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct StartIndicator {
    coordinates: [f64; 3],
    trigger_radius: i64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct MapObject {
    coordinates: [f64; 3],
    angles: [f64; 3],
    scale: f64,
    model_name: String,
    hidden: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MapRoute {
    pub id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub default: bool,
    start_line: Line,
    finish_line: Line,
    leaderboards: Leaderboards,
    checkpoints: Vec<[f64; 3]>,
    start: StartPosition,
    end: EndPosition,
    ziplines: Vec<[[f64; 3]; 2]>,
    perks: Option<HashMap<String, String>>,
    robot: Robot,
    indicator: StartIndicator,
    route_name: RouteName,
    entities: Option<Vec<MapObject>>,
}

#[derive(Deserialize, Default)]
struct EditQuery {
    reason: Option<String>,
}

/// This middleware creates `MapRoute` payloads from POST request bodies.
///
pub fn post_json() -> impl Filter<Extract = (MapRoute,), Error = Rejection> + Clone {
    warp::body::content_length_limit(1024 * 16).and(warp::body::json())
}

/// Creates a map route, based on its map identifier.
///
async fn create_map_route(
    map_id: String,
    mut entry: MapRoute,
    store: Store,
) -> Result<impl Reply, Rejection> {
    // Check if provided map exists
    let routes_list = store.routes_list.read().clone();
    let map_routes = routes_list.get(&map_id);
    if map_routes.is_none() {
        return Ok(warp::reply::with_status(
            warp::reply::json(&"Map not found."),
            StatusCode::NOT_FOUND,
        ));
    }

    let mut routes = map_routes.unwrap().clone();
    let index = routes
        .iter()
        .position(|route| route.name == entry.name)
        .unwrap_or(usize::MAX);
    if index != usize::MAX {
        return Ok(warp::reply::with_status(
            warp::reply::json(&"{\"error\": \"Route name already used.\"}"),
            StatusCode::ALREADY_REPORTED,
        ));
    }

    // Insert new route
    let route_id = Uuid::new_v4().to_string();
    entry.id = Some(route_id.clone());
    if entry.perks.is_none() {
        entry.perks = Some(HashMap::new());
    }
    if entry.entities.is_none() {
        entry.entities = Some(Vec::new());
    }
    routes.push(entry);
    let mut write_lock = store.routes_list.write();
    write_lock.insert(map_id, routes);

    drop(routes_list);
    let mut routes_write = store.routes_list.write();
    let map_routes = routes_write.get_mut(&map_name).unwrap();

    if entry.default {
        for r in map_routes.iter_mut() {
            r.default = false;
        }
    }

    map_routes.push(entry.clone());

    crate::log::route_change("created", &map_name, &entry.name, &slug, None);

    Ok(warp::reply::with_status(
        warp::reply::json(&"Map route created."),
        StatusCode::CREATED,
    ))
}

async fn edit_map_route(
    map_name: String,
    route_slug: String,
    query: EditQuery,
    mut entry: MapRoute,
    store: Store,
) -> Result<impl Reply, Rejection> {
    let mut routes_list = store.routes_list.write();
    let map_routes = match routes_list.get_mut(&map_name) {
        Some(r) => r,
        None => {
            return Ok(warp::reply::with_status(
                warp::reply::json(&"Map not found."),
                StatusCode::NOT_FOUND,
            ));
        }
    };

    let idx = match map_routes
        .iter()
        .position(|r| slugify(&r.name) == route_slug)
    {
        Some(i) => i,
        None => {
            return Ok(warp::reply::with_status(
                warp::reply::json(&"Route not found."),
                StatusCode::NOT_FOUND,
            ));
        }
    };

    // Check if the new name conflicts with a different existing route.
    let new_slug = slugify(&entry.name);
    let conflict = map_routes
        .iter()
        .enumerate()
        .any(|(i, r)| i != idx && slugify(&r.name) == new_slug);
    if conflict {
        return Ok(warp::reply::with_status(
            warp::reply::json(&"Route name already used."),
            StatusCode::ALREADY_REPORTED,
        ));
    }

    if entry.perks.is_none() {
        entry.perks = Some(HashMap::new());
    }
    if entry.entities.is_none() {
        entry.entities = Some(Vec::new());
    }

    if entry.default {
        for (i, r) in map_routes.iter_mut().enumerate() {
            if i != idx {
                r.default = false;
            }
        }
    }

    map_routes[idx] = entry.clone();

    crate::log::route_change(
        "edited",
        &map_name,
        &entry.name,
        &new_slug,
        query.reason.as_deref(),
    );

    Ok(warp::reply::with_status(
        warp::reply::json(&new_slug),
        StatusCode::OK,
    ))
}

async fn get_map_routes(map_name: String, store: Store) -> Result<impl Reply, Rejection> {
    let routes_read_lock = store.routes_list.read();
    if !routes_read_lock.contains_key(&map_id) {
        return Ok(warp::reply::with_status(
            warp::reply::json(&"Route not found."),
            StatusCode::NOT_FOUND,
        ));
        // )),
        // Some(routes) => {
        //     let map: HashMap<String, serde_json::Value> = routes
        //         .iter()
        //         .map(|r| (slugify(&r.name), serde_json::to_value(r).unwrap()))
        //         .collect();
        //     Ok(warp::reply::with_status(
        //         warp::reply::json(&map),
        //         StatusCode::OK,
        //     ))
        // }
    }

    let routes = routes_read_lock.get(&map_id).unwrap();
    Ok(warp::reply::with_status(
        warp::reply::json(&routes),
        StatusCode::OK,
    ))
}

/// Returns all map routing routes:
///     * one route to get a map's routes;
///     * one route to create map routes.
///
pub fn get_routes(store: Store) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let store_filter = warp::any().map(move || store.clone());

    let create = warp::post()
        .and(warp::path("v1"))
        .and(warp::path("maps"))
        .and(warp::path::param())
        .and(warp::path("routes"))
        .and(warp::path::end())
        .and(post_json())
        .and(store_filter.clone())
        .and_then(create_map_route);

    let list = warp::get()
        .and(warp::path("v1"))
        .and(warp::path("maps"))
        .and(warp::path::param())
        .and(warp::path("routes"))
        .and(warp::path::end())
        .and(store_filter.clone())
        .and_then(get_map_routes);

    let edit = warp::put()
        .and(warp::path("v1"))
        .and(warp::path("maps"))
        .and(warp::path::param())
        .and(warp::path("routes"))
        .and(warp::path::param())
        .and(warp::path::end())
        .and(warp::query::<EditQuery>())
        .and(post_json())
        .and(store_filter)
        .and_then(edit_map_route);

    create.or(list).or(edit)
}
