use anyhow::Result;
use axum::{routing::delete, Router};
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
    config: Config,
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
        .route("/delete/:uuid", delete(api::delete_entry))
        .route("/delete_all", delete(api::delete_all))
        .fallback(api::handler)
}
fn new_state(config: Config) -> AppState {
    AppState {
        cache: Cache::new(&config),
        config,
        index_cache: Arc::new(Mutex::new(IndexCache::new())),
        client: Client::new(),
    }
}
// tests

#[cfg(test)]
// backend
mod test {
    use anyhow::Result;
    use axum::{routing::get, Router};
    use axum_test::TestServer;
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
                "/test".to_string(),
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
        let rep = app.get("/test").await;
        rep.assert_status_ok();
        Ok(())
    }
}
