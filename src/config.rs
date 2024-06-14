use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    time::Duration,
};

use axum::http::Uri;
use reqwest::Url;
use serde::{Deserialize, Serialize};
/// configuration struct.
/// Example:
/// listen_port: 9834,
/// endpoints: [("/api1", "127.0.0.1:3998")]
/// request /api1/abc
/// will do 127.0.0.1:3998/abc
#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    /// address and port to which Mnemosyne will listen for incoming requests.
    pub listen_address: SocketAddr,
    /// String is the path mnemosyne will accept request and redirect them to Url
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
    pub fn to_backend_uri(&self, uri_request: &Uri) -> Url {
        if let Some((endpoint, url)) = self
            .endpoints
            .iter()
            .find(|b| uri_request.to_string().contains(&format!("^{}", b.0)))
        {
            let new_uri = uri_request.to_string().replace(endpoint, "");
            Url::parse(&format!("{}{}", url, new_uri).replace("//", "/"))
                .expect("could not parse to Url")
        } else {
            // no uri recognized, using fallback backend
            Url::parse(&format!("{}{}", self.fall_back_endpoint, uri_request).replace("//", "/"))
                .expect("could not parse to Url")
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct CacheConfig {
    /// cache expiration after last request
    pub expiration: Duration,
    /// in megabytes, the maximum size of memory the cache can take.
    pub size_limit: u64,
}
