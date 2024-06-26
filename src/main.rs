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
    let state = AppState {
        cache: Cache::new(&config),
        config,
        index_cache: Arc::new(Mutex::new(IndexCache::new())),
        client: Client::new(),
    };
    info!("Done.");
    // create route for cache API
    let route = Router::new()
        .route("/delete/:uuid", delete(api::delete_entry))
        .route("/delete_all", delete(api::delete_all))
        .fallback(api::handler)
        .with_state(state);
    info!("starting to listen on {listen}");
    let listener = tokio::net::TcpListener::bind(listen).await?;
    axum::serve(listener, route.into_make_service()).await?;
    Ok(())
}
