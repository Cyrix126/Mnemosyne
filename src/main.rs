use ahash::{HashMap, HashMapExt};
use anyhow::Result;
use axum::{
    body::{to_bytes, Bytes},
    extract::{Path, Request, State},
    http::{HeaderMap, HeaderValue, Uri},
    response::IntoResponse,
    routing::delete,
    Router,
};
use config::Config;
use derive_more::{Deref, DerefMut};
use enclose::enc;
use moka::future::Cache as MokaCache;
use reqwest::{
    header::{ETAG, VARY},
    Client, Method, StatusCode,
};
use std::{str::FromStr, sync::Arc};
use tokio::{spawn, sync::Mutex};
use typesize::TypeSize;
use url::Url;
use uuid::Uuid;

/// configuration from file
mod config;
#[derive(Clone)]
struct AppState {
    config: Config,
    // option HeaderMap is the header request that needs to be present.
    // the response will contains a Vary Header in this case.
    // one method and uri can contain multiple different response based on headers, so we use a Vec per entry since the id of the entry is based on uri and method.
    cache: Cache,
    index_cache: Arc<Mutex<IndexCache>>,
    client: Client,
}
#[derive(Deref, DerefMut, Clone)]
/// IndexCache will store entry for each combination of uri/method with a vec of uuid per HeaderMap. HeaderMap here are request headers that match the headers name in the Vary header value response.
struct IndexCache(HashMap<(Method, Uri), Vec<(Uuid, HeaderMap)>>);
#[derive(Deref, DerefMut, Clone)]
struct Cache(MokaCache<Uuid, (StatusCode, HeaderMap, Bytes), ahash::RandomState>);

impl Cache {
    fn new(config: &Config) -> Cache {
        Self(
            MokaCache::builder()
                .name("mnemosyne")
                .time_to_idle(config.cache.expiration)
                .weigher(
                    |_key: &Uuid, (s, h, b): &(StatusCode, HeaderMap, Bytes)| -> u32 {
                        let s = s.to_string().get_size() as u32;
                        let h = h.iter().fold(0, |acc, x| {
                            acc + (x.0.to_string().get_size()
                                + x.1.to_str().unwrap().to_string().get_size())
                                as u32
                        });
                        let b = b.len() as u32;
                        // note that the size overhead of the index cache is not taken into account.
                        // could take about 100B per entry.
                        s + h + b
                    },
                )
                // This cache will hold up to 32MiB of values.
                .max_capacity(config.cache.size_limit * 1024 * 1024)
                .build_with_hasher(ahash::RandomState::new()),
        )
    }
    fn check_etag(&self, headers: &HeaderMap) -> bool {
        if let Some(etag) = headers.get("Etag") {
            if let Ok(str) = etag.to_str() {
                if let Ok(uuid) = Uuid::from_str(str) {
                    return self.contains_key(&uuid);
                }
            }
        }
        false
    }
}

/// from a request, keep only headers that are present in Vary response header
fn headers_match_vary(
    request_headers: &HeaderMap,
    vary_header: Option<&HeaderValue>,
) -> Result<HeaderMap> {
    if let Some(vary) = vary_header {
        let mut h_vary = vary.to_str()?.split(',');
        let mut headers = HeaderMap::new();
        request_headers
            .iter()
            .filter(|h_req| h_vary.any(|name| name == h_req.0.as_str()))
            .for_each(|header| {
                headers.insert(header.0, header.1.clone());
            });
        Ok(headers)
    } else {
        Ok(HeaderMap::new())
    }
}

impl IndexCache {
    fn new() -> Self {
        IndexCache(HashMap::new())
    }
    fn add_entry(
        &mut self,
        uuid: Uuid,
        req_method: Method,
        req_uri: Uri,
        req_headers_match_vary: HeaderMap,
    ) {
        let key = (req_method, req_uri);
        let value = (uuid, req_headers_match_vary);
        // check if entry exist for method/uri

        if let Some(v) = self.get_mut(&key) {
            // if entry exist, push into vec
            v.push(value);
        } else {
            // if no entries, create one.
            self.insert(key, vec![value]);
        }
    }
    /// will search for an entry in cache based on a request. Will check that request headers includes the ones associated in this entry if any.
    /// Will return the uuid of the entry.
    fn request_to_uuid(&self, request: &Request) -> Option<Uuid> {
        let method = request.method().to_owned();
        let uri = request.uri().to_owned();
        let headermap = request.headers();
        if let Some(uuids) = self.get(&(method, uri)) {
            return uuids
                .iter()
                .find(|(_, headermap_object)| {
                    headermap_object
                        .iter()
                        .all(|x| headermap.get(x.0).is_some_and(|value| value == x.1))
                })
                .map(|v| v.0);
        }
        None
    }
    fn delete_uuid_from_index(&mut self, uuid: &Uuid) {
        // remove uuid entry from vec
        self.iter_mut().for_each(|v| v.1.retain(|c| &c.0 != uuid));
        // check if the entry for method/uri is now empty and delete it if that's the case.
        let key = self.iter().find_map(|(key, value)| {
            if value.is_empty() {
                Some(key.to_owned())
            } else {
                None
            }
        });
        if let Some(key) = key {
            self.remove(&key);
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // load config
    let config = confy::load_path::<Config>("/etc/mnemosyne")?;
    // create cache moka
    // create state app
    let listen = config.listen_address;
    let state = AppState {
        cache: Cache::new(&config),
        config,
        index_cache: Arc::new(Mutex::new(IndexCache::new())),
        client: Client::new(),
    };
    // create route for cache API
    let route = Router::new()
        .route("/delete/:uuid", delete(delete_entry))
        .route("/delete_all", delete(delete_all))
        .fallback(handler)
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(listen).await?;
    axum::serve(listener, route.into_make_service()).await?;
    // create listener for all endpoints

    Ok(())
}

// handle delete endpoint
// will also delete from index by iterating over the entries to find the method/path
async fn delete_entry(
    Path(path): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if let Ok(uuid) = Uuid::from_str(&path) {
        state.cache.invalidate(&uuid).await;
        state.index_cache.lock().await.delete_uuid_from_index(&uuid);
        return StatusCode::OK;
    }
    StatusCode::NOT_FOUND
}
// handle delete_all endpoint
async fn delete_all(State(state): State<AppState>) -> impl IntoResponse {
    state.cache.invalidate_all();
    *state.index_cache.lock().await = IndexCache::new();
    StatusCode::OK
}

// handle request
#[axum::debug_handler]
async fn handler(State(state): State<AppState>, request: Request) -> impl IntoResponse {
    // check if etag is present in headers
    if state.cache.check_etag(request.headers()) {
        // respond 304 if etag is present in cache
        return StatusCode::NOT_MODIFIED.into_response();
    }

    // if response is in cache with valid header if any, return response from cache

    if let Some(uuid) = state.index_cache.lock().await.request_to_uuid(&request) {
        let rep = state
            .cache
            .get(&uuid)
            .await
            .expect("a value should be there if index has one");
        // Body can not be saved in Cache so we save Bytes and convert to body when we need it.
        return rep.into_response();
    }

    // if not in cache, make the request to backend service
    let req_method = request.method().to_owned();
    let req_headers = request.headers().to_owned();
    let req_uri = request.uri().to_owned();
    match state
        .client
        .request(
            request.method().to_owned(),
            to_backend_uri(&state.config, request.uri()),
        )
        .headers(request.headers().to_owned())
        .body(to_bytes(request.into_body(), usize::MAX).await.unwrap())
        .send()
        .await
    {
        Ok(mut rep) => {
            // first send Response and then cache so client wait as little as possible.
            // need to add Etag headers to response
            let uuid = Uuid::new_v4();

            let index = state.index_cache.clone();
            let cache = state.cache.clone();
            rep.headers_mut()
                .insert(ETAG, HeaderValue::from_str(&uuid.to_string()).unwrap());
            let headers = rep.headers().to_owned();
            let req_headers_match_vary = match headers_match_vary(&req_headers, headers.get(VARY)) {
                Ok(h) => h,
                Err(_err) => {
                    // seems backend service response contains malformated header value for Vary
                    HeaderMap::new()
                }
            };

            let axum_rep = (
                rep.status(),
                rep.headers().to_owned(),
                rep.bytes().await.unwrap(),
            );

            spawn(enc!((uuid, axum_rep) async move {
                // add entry to index cache
                index.lock().await.add_entry(uuid, req_method, req_uri, req_headers_match_vary);
                // add response to cache
                cache.insert(uuid, axum_rep).await;

            }));
            axum_rep.into_response()
        }
        Err(_err) => {
            // the request to the backend failed

            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

fn to_backend_uri(config: &Config, uri_request: &Uri) -> Url {
    if let Some((endpoint, url)) = config
        .endpoints
        .iter()
        .find(|b| uri_request.to_string().contains(&format!("^{}", b.0)))
    {
        let new_uri = uri_request.to_string().replace(endpoint, "");
        Url::parse(&format!("{}{}", url, new_uri).replace("//", "/"))
            .expect("could not parse to Url")
    } else {
        // no uri recognized, using fallback backend
        config.fall_back_endpoint.to_owned()
    }
}

// if not,
// check cache availability
// if not in cache,
// send request for each route in config to backend service

// add caching headers
// send response
// save in cache
