use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use axum::http::{uri::PathAndQuery, HeaderValue};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tracing::debug;
/// configuration struct.
/// Example:
/// listen_port: 9834,
/// endpoints: [("/api1", "127.0.0.1:3998")]
/// request /api1/abc
/// will do 127.0.0.1:3998/abc
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    /// address and port to which Mnemosyne will listen for incoming requests.
    pub listen_address: SocketAddr,
    /// String is the HOST mnemosyne will accept request and redirect them to Url
    pub endpoints: Vec<(String, Url)>,
    /// if none of the request contained recognized uri or if you want to redirect every request to one backend.
    pub fall_back_endpoint: Url,
    /// cache backend configuration
    pub cache: CacheConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen_address: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9830)),
            endpoints: Default::default(),
            cache: Default::default(),
            fall_back_endpoint: Url::parse("http://127.0.0.1:1000").unwrap(),
        }
    }
}

impl Config {
    pub fn to_backend_uri(&self, uri_req: &PathAndQuery, host: Option<&HeaderValue>) -> Url {
        //todo use regex to get the start of the line
        if let Some(host) = host {
            if let Ok(host) = host.to_str() {
                if let Some((endpoint, url)) = self.endpoints.iter().find(|b| host == b.0) {
                    debug!("endpoint detected: {endpoint}");
                    debug!("url: {url}");
                    return Url::parse(&format!("{}{}", url, uri_req).replace("//", "/"))
                        .expect("could not parse to Url");
                }
            }
        }
        // no uri recognized, using fallback backend
        Url::parse(&format!("{}{}", self.fall_back_endpoint, uri_req).replace("//", "/"))
            .expect("could not parse to Url")
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CacheConfig {
    /// cache expiration after last request
    pub expiration: u64,
    /// in megabytes, the maximum size of memory the cache can take.
    pub size_limit: u64,
}

/// About a month to clear unused entries (if there still room)
/// 250Mb of memory space for the cache
impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            expiration: 2592000,
            size_limit: 250,
        }
    }
}
