use anyhow::Result;
use api::cache::{cache_stats, delete_entries, delete_entry, get_cache_entry};
use api::config::{
    add_endpoint, delete_endpoint, delete_endpoints, get_fallback_value, set_fallback_value,
};
use axum::routing::get;
use axum::{
    routing::{delete, post, put},
    Router,
};
use cache::Cache;
use config::Config;
use index_cache::IndexCache;
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

/// Handlers
mod api;
/// impl for Moka Cache wrapper
mod cache;
/// configuration from file
mod config;
/// IndexCache
mod index_cache;
#[derive(Clone)]
struct AppState {
    config: Arc<Mutex<Config>>,
    // option HeaderMap is the header request that needs to be present.
    // the response will contains a Vary Header in this case.
    // one method and uri can contain multiple different response based on headers, so we use a Vec per entry since the id of the entry is based on uri and method.
    cache: Cache,
    index_cache: Arc<Mutex<IndexCache>>,
    client: Client,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    info!("loading configuration file");
    let config = confy::load_path::<Config>("/etc/mnemosyne/config.toml")?;
    let listen = config.listen_address;
    info!("creating the cache and index...");
    let state = new_state(config);
    info!("Done.");
    // create route for cache API
    let route = router().with_state(state);
    info!("starting to listen on {listen}");
    let listener = tokio::net::TcpListener::bind(listen).await?;
    axum::serve(listener, route.into_make_service()).await?;
    Ok(())
}

fn router() -> Router<AppState> {
    Router::new()
        .nest("/api/1/cache", cache_router())
        .nest("/api/1/config", config_router())
        .fallback(api::handler)
}

fn cache_router() -> Router<AppState> {
    Router::new()
        .route("/:uuid", delete(delete_entry))
        .route("/:uuid", get(get_cache_entry))
        .route("/", delete(delete_entries))
        .route("/", get(cache_stats))
}
fn config_router() -> Router<AppState> {
    Router::new()
        .route("/endpoint/:endpoint", delete(delete_endpoint))
        .route("/endpoint/:endpoint", put(add_endpoint))
        .route("/endpoint", delete(delete_endpoints))
        .route("/fallback", get(get_fallback_value))
        .route("/fallback", post(set_fallback_value))
}
fn new_state(config: Config) -> AppState {
    AppState {
        cache: Cache::new(&config),
        config: Arc::new(Mutex::new(config)),
        index_cache: Arc::new(Mutex::new(IndexCache::new())),
        client: Client::new(),
    }
}
// tests

#[cfg(test)]
// backend
mod test {
    use anyhow::Result;
    use axum::{http::HeaderValue, routing::get, Router};
    use axum_test::TestServer;
    use reqwest::header::HOST;
    use tokio::{net::TcpListener, spawn};
    use url::Url;

    use crate::{config::Config, new_state, router};

    async fn backend_handler() -> &'static str {
        "Hello, World!"
    }
    fn router_backend() -> Router {
        Router::new().route("/", get(backend_handler))
    }
    // needs to start a backend service, will be assigned an open port by the os
    async fn app_backend(listener: TcpListener) -> Result<()> {
        axum::serve(listener, router_backend().into_make_service()).await?;
        Ok(())
    }
    #[tokio::test]
    async fn first_request() -> Result<()> {
        tracing_subscriber::fmt::init();
        // start backend service
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr().unwrap().port();
        spawn(async move { app_backend(listener).await });
        // configuration of Mnemosyne
        let config = Config {
            endpoints: vec![(
                "example.com".to_string(),
                Url::parse(&format!("http://127.0.0.1:{port}"))?,
            )],
            ..Default::default()
        };
        // state of Mnemosyne
        let state = new_state(config);
        // router
        let router = router().with_state(state);
        // start Mnemosyne
        let app = TestServer::new(router).unwrap();
        // send get request for the first time
        let rep = app
            .get("/")
            .add_header(HOST, HeaderValue::from_static("example.com"))
            .await;
        rep.assert_status_ok();
        Ok(())
    }
}
