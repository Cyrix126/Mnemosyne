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
    use std::time::Duration;

    use anyhow::Result;
    use axum::{http::HeaderValue, routing::get, Router};
    use axum_test::TestServer;
    use reqwest::{
        header::{ETAG, HOST},
        StatusCode,
    };
    use tokio::{net::TcpListener, spawn, time::sleep};
    use url::Url;
    use uuid::Uuid;

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
    async fn app() -> Result<TestServer> {
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
        Ok(TestServer::new(router).unwrap())
    }
    #[tokio::test]
    async fn first_request() -> Result<()> {
        // tracing_subscriber::fmt::init();
        let app = app().await.unwrap();
        // send get request for the first time
        let rep = app
            .get("/")
            .add_header(HOST, HeaderValue::from_static("example.com"))
            .await;
        rep.assert_status_ok();
        Ok(())
    }
    #[tokio::test]
    async fn correct_etag() -> Result<()> {
        // tracing_subscriber::fmt::init();
        let app = app().await.unwrap();
        // send get request for the first time
        let rep = app
            .get("/")
            .add_header(HOST, HeaderValue::from_static("example.com"))
            .await;
        rep.assert_status_ok();
        let etag = rep.headers().get(ETAG).unwrap();
        // wait for the cache to save the entry.
        sleep(Duration::from_millis(100)).await;
        // resend same request with the etag
        let rep = app
            .get("/")
            .add_header(HOST, HeaderValue::from_static("example.com"))
            .add_header(ETAG, etag.clone())
            .await;
        // response should only contains header not modified without the body
        rep.assert_status(StatusCode::NOT_MODIFIED);

        Ok(())
    }
    #[tokio::test]
    async fn incorrect_etag() -> Result<()> {
        // tracing_subscriber::fmt::init();
        let app = app().await.unwrap();
        // send get request for the first time
        let rep = app
            .get("/")
            .add_header(HOST, HeaderValue::from_static("example.com"))
            .await;
        rep.assert_status_ok();
        // wait for the cache to save the entry.
        sleep(Duration::from_millis(100)).await;
        // resend same request with the etag
        let etag = Uuid::new_v4().to_string();
        let rep = app
            .get("/")
            .add_header(HOST, HeaderValue::from_static("example.com"))
            .add_header(ETAG, HeaderValue::from_str(&etag).unwrap())
            .await;
        // response should only contains header not modified without the body
        rep.assert_status(StatusCode::OK);
        Ok(())
    }
    #[tokio::test]
    async fn cache_served() -> Result<()> {
        // tracing_subscriber::fmt::init();
        let app = app().await.unwrap();
        // send get request for the first time
        let rep = app
            .get("/")
            .add_header(HOST, HeaderValue::from_static("example.com"))
            .await;
        rep.assert_status_ok();
        // wait for the cache to save the entry.
        sleep(Duration::from_millis(100)).await;
        // check that cache has the entry.
        let etag = rep.headers().get(ETAG).unwrap();
        let uri = format!("/api/1/cache/{}", etag.to_str().unwrap());
        app.get(&uri).await.assert_status_ok();
        // resend request. response should be served from cache.
        app.get("/")
            .add_header(HOST, HeaderValue::from_static("example.com"))
            .await
            .assert_status_ok();
        // response should only contains header not modified without the body
        Ok(())
    }
    #[tokio::test]
    async fn cache_must_be_empty() -> Result<()> {
        // tracing_subscriber::fmt::init();
        let app = app().await.unwrap();
        // send get request for the first time
        let rep = app
            .get("/")
            .add_header(HOST, HeaderValue::from_static("example.com"))
            .await;
        rep.assert_status_ok();
        // wait for the cache to save the entry.
        sleep(Duration::from_millis(100)).await;
        // delete the entry
        let etag = rep.headers().get(ETAG).unwrap();
        let uri = format!("/api/1/cache/{}", etag.to_str().unwrap());
        app.delete(&uri).await.assert_status_ok();
        app.get(&uri).await.assert_status_not_found();
        // resend request. response should be served from cache.
        app.get("/")
            .add_header(HOST, HeaderValue::from_static("example.com"))
            .await
            .assert_status_ok();
        // response should only contains header not modified without the body
        Ok(())
    }
}
