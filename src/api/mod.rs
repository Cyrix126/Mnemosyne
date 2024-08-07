use crate::index_cache::headers_match_vary;
use crate::AppState;
use axum::body::to_bytes;
use axum::extract::{Request, State};
use axum::http::{uri::PathAndQuery, HeaderMap, HeaderValue};
use axum::response::IntoResponse;
use enclose::enc;
use reqwest::header::{ETAG, HOST, VARY};
use reqwest::StatusCode;
use tokio::spawn;
use tracing::{debug, info, trace, warn};
use uuid::Uuid;

pub mod cache;
pub mod config;

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
    let req_host = request.headers().get(HOST).cloned();
    let req_headers = request.headers().to_owned();
    let req_uri = request
        .uri()
        .path_and_query()
        .cloned()
        .unwrap_or(PathAndQuery::from_static(""));
    debug!("response was not cached, requesting backend service");
    let url_backend = state
        .config
        .lock()
        .await
        .to_backend_uri(&req_uri, &req_host);
    debug!("Request URI retrieved: {req_uri}");
    debug!("Request URL transmitted:{url_backend}");
    let req = state
        .client
        .request(request.method().to_owned(), url_backend)
        .headers(request.headers().to_owned())
        .body(to_bytes(request.into_body(), usize::MAX).await.unwrap())
        .send()
        .await;
    match req {
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
                if let Some(host) = req_host {
                // add entry to index cache
                debug!("adding the new response to the cache and indexing");
                index.lock().await.add_entry(uuid, req_method, req_uri, host, req_headers_match_vary);
                // add response to cache
                cache.insert(uuid, axum_rep).await;
                } else {
                    warn!("request does not have a HOST header, not adding any entry to cache");
                }

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
