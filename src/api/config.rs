use axum::{
    extract::{Path, State},
    response::IntoResponse,
};
use reqwest::StatusCode;
use tracing::debug;
use url::Url;

use crate::AppState;

// handle delete endpoint
pub async fn delete_endpoint(
    Path(path): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    debug!("new request to delete an endpoint in configuration");
    if let Some(index) = state
        .config
        .lock()
        .await
        .endpoints
        .iter()
        .position(|x| *x.0 == path)
    {
        // delete endpoint
        state.config.lock().await.endpoints.remove(index);
        // write config

        // return success
        return StatusCode::OK;
    }
    // return not found
    StatusCode::NOT_FOUND
}
// handle add endpoint
pub async fn add_endpoint(
    Path(path): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    debug!("new request to delete an endpoint in configuration");
    if let Some(index) = state
        .config
        .lock()
        .await
        .endpoints
        .iter()
        .position(|x| *x.0 == path)
    {
        // delete endpoint
        state.config.lock().await.endpoints.remove(index);
        // write config

        // return success
        return StatusCode::OK;
    }
    // return not found
    StatusCode::NOT_FOUND
}
pub async fn set_fallback_value(State(state): State<AppState>, body: String) -> impl IntoResponse {
    debug!("new request to set the fallback in configuration");
    if let Ok(url) = Url::parse(&body) {
        state.config.lock().await.fall_back_endpoint = url;
    }
    // return not found
    StatusCode::NOT_FOUND
}
pub async fn get_fallback_value(State(state): State<AppState>) -> impl IntoResponse {
    debug!("new request to get the fallback in configuration");
    let body = &state.config.lock().await.fall_back_endpoint;
    // return not found
    (StatusCode::NOT_FOUND, body.to_string())
}
// handle delete all  endpoints
pub async fn delete_endpoints(State(state): State<AppState>) -> impl IntoResponse {
    debug!("new request to delete all endpoints in configuration");
    state.config.lock().await.endpoints = Vec::new();
    StatusCode::OK
}
