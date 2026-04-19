use std::{fs::File, io::Read, sync::Arc};

use handlebars::{Handlebars, handlebars_helper};
use serde::{Deserialize, Serialize};
use serde_json::json;
use warp::{Filter, Rejection, Reply};

use crate::{Store, log, scores::ScoreEntry, slug::slugify};

const TEMPLATE_FILE: &str = "scoreboard/template.html";

#[derive(Debug, Deserialize, Serialize, Clone)]
struct RouteResult {
    slug: String,
    name: String,
    map_name: String,
    scores: Vec<ScoreEntry>,
}

fn render(hbs: Arc<Handlebars<'_>>, store: Store) -> impl warp::Reply {
    let routes_snapshot = store.routes_list.read().clone();
    let db = store.db.lock();

    let mut results: Vec<RouteResult> = Vec::new();

    // Iterate maps in insertion order (maps_list preserves it)
    for map_name in store.maps_list.read().iter() {
        let map_routes = match routes_snapshot.get(map_name) {
            Some(r) => r,
            None => continue,
        };

        for route in map_routes {
            let route_slug = slugify(&route.name);

            let mut stmt = db
                .prepare(
                    "SELECT s.uid, u.name, s.time, s.timestamp, s.recording \
                     FROM scores s JOIN users u ON s.uid = u.uid \
                     WHERE s.map_name = ?1 AND s.route_slug = ?2 \
                     ORDER BY s.time ASC",
                )
                .unwrap();

            let scores: Vec<ScoreEntry> = stmt
                .query_map([map_name.as_str(), route_slug.as_str()], |row| {
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

            results.push(RouteResult {
                slug: route_slug,
                name: route.name.clone(),
                map_name: map_name.clone(),
                scores,
            });
        }
    }

    let value = json!({ "results": results });
    let render = hbs
        .render("template.html", &value)
        .unwrap_or_else(|err| err.to_string());

    warp::reply::html(render)
}

pub fn get_routes(store: Store) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let mut file = match File::open(TEMPLATE_FILE) {
        Ok(file) => file,
        Err(_) => {
            log::info(&format!(
                "\"{}\" template file was not found.",
                TEMPLATE_FILE
            ));
            std::process::exit(3);
        }
    };
    let mut data = String::new();
    match file.read_to_string(&mut data) {
        Ok(_) => (),
        Err(err) => {
            log::error(&format!(
                "Failed reading \"{}\" file [{}].",
                TEMPLATE_FILE, err
            ));
            std::process::exit(2);
        }
    };

    let mut hb = Handlebars::new();
    hb.register_template_string("template.html", data).unwrap();

    handlebars_helper!(score_index: |index: i64| index + 1);
    hb.register_helper("score_index", Box::new(score_index));

    handlebars_helper!(reddec: |time: f64| format!("{time:.3}"));
    hb.register_helper("reddec", Box::new(reddec));

    let hb = Arc::new(hb);
    let handlebars = move || render(hb.clone(), store.clone());

    let static_assets = warp::path("assets").and(warp::fs::dir("scoreboard/assets"));
    let scoreboard = warp::get().and(warp::path::end()).map(handlebars);

    static_assets.or(scoreboard)
}
