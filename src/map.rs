use serde::{Deserialize, Serialize};
use warp::{Filter, Rejection, Reply, http::StatusCode};

use crate::Store;

// Maps is just an ordered list of known map names for listing purposes.
// The routes_list HashMap is the authoritative source; this list mirrors its keys.
pub type Maps = Vec<String>;

#[derive(Debug, Deserialize, Serialize)]
struct CreateMapRequest {
    map_name: String,
}

async fn get_list(store: Store) -> Result<impl Reply, Rejection> {
    let r = store.maps_list.read().await;
    Ok(warp::reply::json(&*r))
}

async fn create_map(body: CreateMapRequest, store: Store) -> Result<impl Reply, Rejection> {
    let map_name = body.map_name.trim().to_string();

    if store.routes_list.read().await.contains_key(&map_name) {
        return Ok(warp::reply::with_status(
            warp::reply::json(&"Map already exists."),
            StatusCode::ALREADY_REPORTED,
        ));
    }

    store
        .routes_list
        .write()
        .await
        .insert(map_name.clone(), Vec::new());
    store.maps_list.write().await.push(map_name);

    Ok(warp::reply::with_status(
        warp::reply::json(&"Map created."),
        StatusCode::CREATED,
    ))
}

fn post_json() -> impl Filter<Extract = (CreateMapRequest,), Error = Rejection> + Clone {
    warp::body::content_length_limit(1024 * 16).and(warp::body::json())
}

pub fn get_routes(store: Store) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let store_filter = warp::any().map(move || store.clone());

    let list = warp::get()
        .and(warp::path("v1"))
        .and(warp::path("maps"))
        .and(warp::path::end())
        .and(store_filter.clone())
        .and_then(get_list);

    let create = warp::post()
        .and(warp::path("v1"))
        .and(warp::path("maps"))
        .and(warp::path::end())
        .and(post_json())
        .and(store_filter)
        .and_then(create_map);

    list.or(create)
}
