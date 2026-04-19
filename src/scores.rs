use serde::{Deserialize, Serialize};
use warp::{Filter, Rejection, Reply, hyper::StatusCode};

use crate::{Store, slug::slugify};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ScoreEntry {
    pub uid: String,
    pub name: String,
    pub time: f64,
    pub recording: String,
}

#[derive(Debug, Deserialize)]
struct ScoreRequest {
    uid: String,
    name: String,
    time: f64,
    recording: String,
}

#[derive(Debug, Serialize)]
struct GhostRecord {
    name: String,
    recording: String,
}

fn route_exists(store: &Store, map_name: &str, route_slug: &str) -> bool {
    let routes = store.routes_list.read();
    match routes.get(map_name) {
        None => false,
        Some(map_routes) => map_routes.iter().any(|r| slugify(&r.name) == route_slug),
    }
}

async fn get_list(
    map_name: String,
    route_slug: String,
    store: Store,
) -> Result<impl Reply, Rejection> {
    if !route_exists(&store, &map_name, &route_slug) {
        return Ok(warp::reply::with_status(
            warp::reply::json(&"Route not found."),
            StatusCode::NOT_FOUND,
        ));
    }

    let db = store.db.lock();
    let mut stmt = db
        .prepare(
            "SELECT s.uid, u.name, s.time, s.timestamp, s.recording \
                    FROM scores s \
                    JOIN users u ON s.uid = u.uid \
                    WHERE s.map_name = ?1 AND s.route_slug = ?2 \
                    ORDER BY s.time ASC;",
        )
        .unwrap();

    let entries: Vec<ScoreEntry> = stmt
        .query_map([map_name, route_slug], |row| {
            Ok(ScoreEntry {
                uid: row.get(0)?,
                name: row.get(1)?,
                time: row.get(2)?,
                recording: "".to_string(),
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    Ok(warp::reply::with_status(
        warp::reply::json(&entries),
        StatusCode::OK,
    ))
}

async fn create_score(
    map_name: String,
    route_slug: String,
    body: ScoreRequest,
    store: Store,
) -> Result<impl Reply, Rejection> {
    if !route_exists(&store, &map_name, &route_slug) {
        return Ok(warp::reply::with_status(
            warp::reply::json(&"Route not found."),
            StatusCode::NOT_FOUND,
        ));
    }

    let db = store.db.lock();

    // Upsert user (update name if it changed)
    db.execute(
        "INSERT INTO users (uid, name) VALUES (?1, ?2) \
         ON CONFLICT(uid) DO UPDATE SET name = excluded.name",
        rusqlite::params![body.uid, body.name],
    )
    .unwrap();

    // Check existing best time
    let existing: Option<f64> = db
        .query_row(
            "SELECT time FROM scores WHERE map_name = ?1 AND route_slug = ?2 AND uid = ?3",
            rusqlite::params![map_name, route_slug, body.uid],
            |row| row.get(0),
        )
        .ok();

    if let Some(best) = existing
        && body.time >= best
    {
        return Ok(warp::reply::with_status(
            warp::reply::json(&"Leaderboard contains a better score for this player."),
            StatusCode::ALREADY_REPORTED,
        ));
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    db.execute(
        "INSERT OR REPLACE INTO scores (map_name, route_slug, uid, time, timestamp, recording) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![map_name, route_slug, body.uid, body.time, now, body.recording],
    )
    .unwrap();

    Ok(warp::reply::with_status(
        warp::reply::json(&"Score created."),
        StatusCode::CREATED,
    ))
}

async fn get_recording(
    map_name: String,
    route_slug: String,
    uid: String,
    store: Store,
) -> Result<impl Reply, Rejection> {
    if !route_exists(&store, &map_name, &route_slug) {
        return Ok(warp::reply::with_status(
            warp::reply::json(&"Route not found."),
            StatusCode::NOT_FOUND,
        ));
    }

    let db = store.db.lock();

    let ghost = db
        .query_row(
            r#"
    SELECT u.name, s.recording
    FROM scores s
    JOIN users u ON u.uid = s.uid
    WHERE s.map_name = ?1
    AND s.route_slug = ?2
    AND s.recording <> ''
    AND s.time < (
        SELECT time
        FROM scores
        WHERE map_name = ?1
        AND route_slug = ?2
        AND uid = ?3
        LIMIT 1
    )
    ORDER BY s.time DESC
    LIMIT 1;
    "#,
            rusqlite::params![map_name, route_slug, uid],
            |row| {
                Ok(GhostRecord {
                    name: row.get(0)?,
                    recording: row.get(1)?,
                })
            },
        )
        .ok()
        .or_else(|| {
            // try to get the lowest time (since maybe the player doesn't have a score on this route)
            db.query_row(
                r#"SELECT u.name, s.time, s.recording
                        FROM scores s
                        JOIN users u ON u.uid = s.uid
                        ORDER BY s.time DESC
                        LIMIT 1
                    "#,
                rusqlite::params![],
                |row| {
                    Ok(GhostRecord {
                        name: row.get(0)?,
                        recording: row.get(2)?,
                    })
                },
            )
            .ok()
        });

    match ghost {
        Some(ghost) => Ok(warp::reply::with_status(
            warp::reply::json(&ghost),
            StatusCode::OK,
        )),
        None => Ok(warp::reply::with_status(
            warp::reply::json(&"No recording found."),
            StatusCode::NOT_FOUND,
        )),
    }
}

fn post_json() -> impl Filter<Extract = (ScoreRequest,), Error = Rejection> + Clone {
    warp::body::content_length_limit(1024 * 16).and(warp::body::json())
}

pub fn get_routes(store: Store) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let store_filter = warp::any().map(move || store.clone());

    let list = warp::get()
        .and(warp::path("v1"))
        .and(warp::path("maps"))
        .and(warp::path::param())
        .and(warp::path("routes"))
        .and(warp::path::param())
        .and(warp::path("scores"))
        .and(warp::path::end())
        .and(store_filter.clone())
        .and_then(get_list);

    let create = warp::post()
        .and(warp::path("v1"))
        .and(warp::path("maps"))
        .and(warp::path::param())
        .and(warp::path("routes"))
        .and(warp::path::param())
        .and(warp::path("scores"))
        .and(warp::path::end())
        .and(post_json())
        .and(store_filter.clone())
        .and_then(create_score);

    let recording = warp::get()
        .and(warp::path("v1"))
        .and(warp::path("maps"))
        .and(warp::path::param())
        .and(warp::path("routes"))
        .and(warp::path::param())
        .and(warp::path("recording"))
        .and(warp::path::param())
        .and(warp::path::end())
        .and(store_filter)
        .and_then(get_recording);

    list.or(create).or(recording)
}
