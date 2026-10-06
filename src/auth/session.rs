use chrono::Utc;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub type RateLimitStore = Arc<Mutex<HashMap<String, Vec<i64>>>>;

pub fn check_rate_limit(store: &RateLimitStore, key: &str, max: i32, window_secs: i64) -> bool {
    let now = Utc::now().timestamp();
    let mut map = store.lock().unwrap();
    let entry = map.entry(key.to_string()).or_insert_with(Vec::new);
    entry.retain(|&t| now - t < window_secs);
    if (entry.len() as i32) >= max {
        return false;
    }
    entry.push(now);
    true
}
