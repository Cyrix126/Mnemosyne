use std::str::FromStr;

use axum::body::Bytes;
use derive_more::{Deref, DerefMut};
use moka::future::Cache as MokaCache;
use reqwest::header::{HeaderMap, ETAG};
use reqwest::StatusCode;
use typesize::TypeSize;
use uuid::Uuid;

use crate::config::Config;
#[derive(Deref, DerefMut, Clone, Debug)]
pub struct Cache(pub MokaCache<Uuid, (StatusCode, HeaderMap, Bytes), ahash::RandomState>);

impl Cache {
    pub fn new(config: &Config) -> Cache {
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
    pub fn check_etag(&self, headers: &HeaderMap) -> bool {
        if let Some(etag) = headers.get(ETAG) {
            if let Ok(str) = etag.to_str() {
                if let Ok(uuid) = Uuid::from_str(str) {
                    return self.contains_key(&uuid);
                }
            }
        }
        false
    }
}
