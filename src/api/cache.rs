use std::str::FromStr;

use crate::index_cache::IndexCache;
use crate::AppState;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::{extract::State, response::IntoResponse, Json};
use serde::Serialize;
use tracing::{debug, warn};
use uuid::Uuid;

// handle get cache endpoint
pub async fn cache_stats(State(state): State<AppState>) -> impl IntoResponse {
    debug!("new request to get cache stats");
    let stats = CacheStats {
        name: state.cache.name().unwrap_or_default().to_string(),
        entries: state.cache.entry_count(),
        size: state.cache.weighted_size(),
    };
    (StatusCode::OK, Json(stats))
}
#[derive(Serialize)]
struct CacheStats {
    name: String,
    entries: u64,
    size: u64,
}

// handle delete endpoint
// will also delete from index by iterating over the entries to find the method/path
pub async fn delete_entry(
    Path(path): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    debug!("new request to delete a cache entry");
    if let Ok(uuid) = Uuid::from_str(&path) {
        state.cache.invalidate(&uuid).await;
        state.index_cache.lock().await.delete_uuid_from_index(&uuid);
        debug!("cache entry removed");
        return StatusCode::OK;
    }
    warn!("deletion request for invalid uuid");
    StatusCode::NOT_FOUND
}
// handle raw entry endpoint
// will return the raw data of a cache entry
// it is present for debugging purposes.
pub async fn get_cache_entry(
    Path(path): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    debug!("new request to return a raw cache entry");
    if let Ok(uuid) = Uuid::from_str(&path) {
        if let Some(entry) = state.cache.get(&uuid).await {
            return entry.into_response();
        }
    }
    warn!("deletion request for invalid uuid");
    StatusCode::NOT_FOUND.into_response()
}
// handle delete_all endpoint
pub async fn delete_entries(State(state): State<AppState>) -> impl IntoResponse {
    debug!("new request to delete all cache entries");
    state.cache.invalidate_all();
    *state.index_cache.lock().await = IndexCache::new();
    debug!("all cache cleared");
    StatusCode::OK
}
