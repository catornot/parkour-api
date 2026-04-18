use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use warp::{hyper::StatusCode, Filter, Rejection, Reply};

use crate::Store;

pub type ScoreEntries = HashMap<String, Vec<ScoreEntry>>;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ScoreEntry {
    pub uid: String,
    pub name: String,
    pub time: f64,
    pub timestamp: i64,
    // pub recording_ref: String,
}

#[derive(Debug, Deserialize)]
struct ScoreRequest {
    uid: String,
    name: String,
    time: f32,
}

/// Retrives scores list associated to a route id.
///
async fn get_list(route_id: String, store: Store) -> Result<impl Reply, Rejection> {
    let scores_read_lock = store.scores_list.read();
    if !scores_read_lock.contains_key(&route_id) {
        return Ok(warp::reply::with_status(
            warp::reply::json(&"{\"message\": \"Route not found.\"}"),
            StatusCode::NOT_FOUND,
        ));
    }

    let db = store.db.lock();
    let mut stmt = db
        .prepare(
            "SELECT s.uid, u.name, s.time, s.timestamp \
             FROM scores s JOIN users u ON s.uid = u.uid \
             WHERE s.map_name = ?1 AND s.route_slug = ?2 \
             ORDER BY s.time ASC",
        )
        .unwrap();

    let entries: Vec<ScoreEntry> = stmt
        .query_map([&map_name, &route_slug], |row| {
            Ok(ScoreEntry {
                uid: row.get(0)?,
                name: row.get(1)?,
                time: row.get(2)?,
                timestamp: row.get(3)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    Ok(warp::reply::with_status(
        warp::reply::json(&scores),
        StatusCode::OK,
    ))
}

/// This middleware creates `Score` payloads from POST request bodies.
///
fn post_json() -> impl Filter<Extract = (ScoreEntry,), Error = Rejection> + Clone {
    warp::body::content_length_limit(1024 * 16).and(warp::body::json())
}

/// Creates a score entry on a given route, based on its identifier.
///
async fn create_score_entry(
    route_id: String,
    entry: ScoreEntry,
    store: Store,
) -> Result<impl Reply, Rejection> {
    // Check if provided route exists
    let scores_map: ScoreEntries = store.scores_list.read().clone();
    let optional_scores = scores_map.get(&route_id);
    if optional_scores.is_none() {
        return Ok(warp::reply::with_status(
            warp::reply::json(&"Route not found."),
            StatusCode::NOT_FOUND,
        ));
    }

    let mut scores = optional_scores.unwrap().clone().to_vec();
    let index = scores
        .iter()
        .position(|e| e.name == entry.name)
        .unwrap_or(usize::MAX);
    if index != usize::MAX {
        let existing_entry = &scores[index];
        // If existing entry is better than new entry, we keep the new entry
        if entry.time >= existing_entry.time {
            return Ok(warp::reply::with_status(
                warp::reply::json(&"{\"message\": \"Leaderboard contains a better score entry for this player.\"}"),
                StatusCode::ALREADY_REPORTED,
            ));
        }
        // Else, we remove the existing entry
        else {
            scores.remove(index);
        }
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    db.execute(
        "INSERT OR REPLACE INTO scores (map_name, route_slug, uid, time, timestamp) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![map_name, route_slug, body.uid, body.time, now],
    )
    .unwrap();

    Ok(warp::reply::with_status(
        warp::reply::json(&"Score created."),
        StatusCode::CREATED,
    ))
}

/// Returns all score-associated routes:
///     * one route to list a route's scores;
///     * one route to create scores on a given route.
///
pub fn get_routes(store: Store) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let store_filter = warp::any().map(move || store.clone());

    let scores_list_route = warp::get()
        .and(warp::path("v1"))
        .and(warp::path("routes"))
        .and(warp::path::param())
        .and(warp::path("scores"))
        .and(warp::path::end())
        .and(store_filter.clone())
        .and_then(get_list);

    let score_creation_route = warp::post()
        .and(warp::path("v1"))
        .and(warp::path("routes"))
        .and(warp::path::param())
        .and(warp::path("scores"))
        .and(warp::path::end())
        .and(post_json())
        .and(store_filter)
        .and_then(create_score_entry);

    scores_list_route.or(score_creation_route)
}
