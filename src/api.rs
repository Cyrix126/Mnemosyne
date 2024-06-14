use std::str::FromStr;

use axum::{
    body::to_bytes,
    extract::{Path, Request, State},
    http::{HeaderMap, HeaderValue},
    response::IntoResponse,
};
use enclose::enc;
use reqwest::{
    header::{ETAG, VARY},
    StatusCode,
};
use tokio::spawn;
use tracing::{debug, info, trace, warn};
use uuid::Uuid;

use crate::{
    index_cache::{headers_match_vary, IndexCache},
    AppState,
};

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
// handle delete_all endpoint
pub async fn delete_all(State(state): State<AppState>) -> impl IntoResponse {
    debug!("new request to delete all cache entries");
    state.cache.invalidate_all();
    *state.index_cache.lock().await = IndexCache::new();
    debug!("all cache cleared");
    StatusCode::OK
}

// handle request
pub async fn handler(State(state): State<AppState>, request: Request) -> impl IntoResponse {
    debug!("new request for backend");
    trace!("{:?}", request);
    // check if etag is present in headers
    if state.cache.check_etag(request.headers()) {
        // respond 304 if etag is present in cache
        debug!("etag is valid, returning 304 status");
        return StatusCode::NOT_MODIFIED.into_response();
    }

    // if response is in cache with valid header if any, return response from cache
    let index = state.index_cache;
    if let Some(uuid) = index.lock().await.request_to_uuid(&request) {
        if let Some(rep) = state.cache.get(&uuid).await {
            info!("cache entry is served");
            return rep.into_response();
        } else {
            // present in index_cache but not in cache, it means it was automatically invalidated.
            // must update index cache.
            debug!("index was not updated, entry in cache was deleted automaticcaly");
            index.lock().await.delete_uuid_from_index(&uuid);
        }
    }

    // if not in cache, make the request to backend service
    let req_method = request.method().to_owned();
    let req_headers = request.headers().to_owned();
    let req_uri = request.uri().to_owned();
    debug!("response was not cached, requesting backend service");
    let url_backend = state.config.to_backend_uri(&req_uri);
    debug!("Request URI retrieved: {req_uri}");
    debug!("Request URL transmitted:{url_backend}");
    match state
        .client
        .request(request.method().to_owned(), url_backend)
        .headers(request.headers().to_owned())
        .body(to_bytes(request.into_body(), usize::MAX).await.unwrap())
        .send()
        .await
    {
        Ok(mut rep) => {
            // first send Response and then cache so client wait as little as possible.
            // need to add Etag headers to response
            let uuid = Uuid::new_v4();
            let cache = state.cache.clone();
            rep.headers_mut()
                .insert(ETAG, HeaderValue::from_str(&uuid.to_string()).unwrap());
            let headers = rep.headers().to_owned();
            let req_headers_match_vary = match headers_match_vary(&req_headers, headers.get(VARY)) {
                Ok(h) => h,
                Err(err) => {
                    warn!("backend service contains malformated header value for Vary");
                    debug!("{err}");
                    trace!("{:?}", rep);
                    HeaderMap::new()
                }
            };

            let axum_rep = (
                rep.status(),
                rep.headers().to_owned(),
                rep.bytes().await.unwrap(),
            );

            spawn(enc!((uuid, axum_rep, index) async move {
                debug!("adding the new response to the cache and indexing");
                // add entry to index cache
                index.lock().await.add_entry(uuid, req_method, req_uri, req_headers_match_vary);
                // add response to cache
                cache.insert(uuid, axum_rep).await;

            }));
            debug!("serving new response with added header Etag");
            trace!("{:?}", axum_rep);
            axum_rep.into_response()
        }
        Err(err) => {
            // the request to the backend failed
            warn!("the request to the backend service failed");
            debug!("{}", err);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
