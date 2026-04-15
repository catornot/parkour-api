use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use warp::{hyper::StatusCode, Filter, Rejection, Reply};

use crate::Store;

pub type RecordingEntries = HashMap<String, Vec<Recording>>;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Recording {
    reference: String,
    recording: Option<String>,
}

/// Retrives the recording for the rival.
///
async fn get_recording(route_id: String, store: Store) -> Result<impl Reply, Rejection> {
    let scores_read_lock = store.scores_list.read();
    let recodings_read_lock = store.recordings_list.read();
    if !scores_read_lock.contains_key(&route_id) {
        return Ok(warp::reply::with_status(
            warp::reply::json(&"{\"message\": \"Route not found.\"}"),
            StatusCode::NOT_FOUND,
        ));
    }

    let scores = scores_read_lock.get(&route_id).unwrap();
    Ok(warp::reply::with_status(
        warp::reply::json(&scores),
        StatusCode::OK,
    ))
}

/// This middleware creates `Score` payloads from POST request bodies.
///
fn post_json() -> impl Filter<Extract = (Recording,), Error = Rejection> + Clone {
    warp::body::content_length_limit(1024 * 32).and(warp::body::json())
}

/// Creates a recoding entry on a given route. based on it's id
///
async fn create_recording(
    route_id: String,
    entry: Recording,
    store: Store,
) -> Result<impl Reply, Rejection> {
    // Check if provided route exists
    let recordings = store.recordings_list.read();

    // TODO: make this better

    Ok(warp::reply::with_status(
        warp::reply::json(&"Score created."),
        StatusCode::CREATED,
    ))
}

/// Returns all score-associated routes:
///     * one route to get recording for a run;
///     * one route to create recording for a run.
///
pub fn get_routes(store: Store) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let store_filter = warp::any().map(move || store.clone());

    let scores_list_route = warp::get()
        .and(warp::path("v1"))
        .and(warp::path("routes"))
        .and(warp::path::param())
        .and(warp::path("recordings"))
        .and(warp::path::end())
        .and(store_filter.clone())
        .and_then(get_recording);

    let score_creation_route = warp::post()
        .and(warp::path("v1"))
        .and(warp::path("routes"))
        .and(warp::path::param())
        .and(warp::path("recordings"))
        .and(warp::path::end())
        .and(post_json())
        .and(store_filter)
        .and_then(create_recording);

    scores_list_route.or(score_creation_route)
}
