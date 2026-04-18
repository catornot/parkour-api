use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;

const ROUTE_CHANGES_FILE: &str = "data/route_changes.log";

pub fn route_change(action: &str, map_name: &str, route_name: &str, slug: &str, reason: Option<&str>) {
    let date = Local::now();
    let reason_part = match reason {
        Some(r) if !r.trim().is_empty() => format!(" reason={:?}", r),
        _ => String::new(),
    };
    let line = format!(
        "{} [{}] map={:?} route={:?} slug={:?}{}\n",
        date.format("[%Y-%m-%d %H:%M:%S]"),
        action,
        map_name,
        route_name,
        slug,
        reason_part,
    );
    match OpenOptions::new()
        .create(true)
        .append(true)
        .open(ROUTE_CHANGES_FILE)
    {
        Ok(mut f) => { let _ = f.write_all(line.as_bytes()); }
        Err(e) => error(&format!("Failed writing to route changes log: {}", e)),
    }
}

pub fn info(msg: &str) {
    print_message(msg, "info");
}

pub fn warn(msg: &str) {
    print_message(msg, "warn");
}

pub fn error(msg: &str) {
    print_message(msg, "error");
}

fn print_message(msg: &str, level: &str) {
    let date = Local::now();
    println!("{}[{}] {}", date.format("[%Y-%m-%d %H:%M:%S]"), level, msg);
}
