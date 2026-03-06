use std::sync::atomic::Ordering;

use super::AppState;

pub fn increment_requests(state: &AppState) {
    state.requests_served.fetch_add(1, Ordering::Relaxed);
}
