use std::time::Duration;

use reqwest::Url;
use serde::{Deserialize, Serialize};
/// configuration struct.
/// Example:
/// listen_port: 9834,
/// endpoints: [("/api1", "127.0.0.1:3998")]
#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    /// port to which Mnemosyne will listen for incoming requests.
    pub listen_port: u16,
    /// String is the path mnemosyne will accept request and redirect them to Url
    pub endpoints: Vec<(String, Url)>,
    /// cache backend configuration
    pub cache: CacheConfig,
}

#[derive(Serialize, Deserialize, Default)]
pub struct CacheConfig {
    /// cache expiration after last request
    pub expiration: Duration,
    /// maximum cache entry
    pub max_entry: u64,
}
