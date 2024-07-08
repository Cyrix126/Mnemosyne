use ahash::HashMap;
use ahash::HashMapExt;
use axum::body::Body;
use axum::http::uri::PathAndQuery;
use axum::http::HeaderValue;
use axum::http::{HeaderMap, Request};
use derive_more::{Deref, DerefMut};
use reqwest::Method;
use uuid::Uuid;
#[derive(Deref, DerefMut, Clone)]
/// IndexCache will store entry for each combination of uri/method with a vec of uuid per HeaderMap. HeaderMap here are request headers that match the headers name in the Vary header value response.
pub struct IndexCache(pub HashMap<(axum::http::Method, PathAndQuery), Vec<(Uuid, HeaderMap)>>);

impl IndexCache {
    pub fn new() -> Self {
        IndexCache(HashMap::new())
    }
    pub fn add_entry(
        &mut self,
        uuid: Uuid,
        req_method: Method,
        req_uri: PathAndQuery,
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
    pub fn request_to_uuid(&self, request: &Request<Body>) -> Option<Uuid> {
        let method = request.method().to_owned();
        let uri = request
            .uri()
            .path_and_query()
            .cloned()
            .unwrap_or(PathAndQuery::from_static(""));
        let headermap = request.headers();
        if let Some(uuids) = self.get(&(method, uri.clone())) {
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
    pub fn delete_uuid_from_index(&mut self, uuid: &Uuid) {
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
/// from a request, keep only headers that are present in Vary response header
pub fn headers_match_vary(
    request_headers: &HeaderMap,
    vary_header: Option<&HeaderValue>,
) -> anyhow::Result<HeaderMap> {
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
