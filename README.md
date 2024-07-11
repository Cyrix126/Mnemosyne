# Mnemosyne
Mnemosyne is a http request caching in memory proxy API.
## Status of development
All features are present, but the software is very new and not tested for production.
### TODO
- [x] allows to limit cache by size
- [x] organize code in modules
- [x] tracing
- [x] tests
- [x] documentation
- [ ] benchmarks/optimizations
## Description
Mnemosyne is placed between your load balancer (ex: nginx) and your server applications that needs their requests to be cached. It will optimize the resources by caching responses and adding caching headers that will ask clients to re-use the cache locally. The cache will be expired based on activity and from manual invalidation.
## Objectives
This software is meant to add caching capability to your backend service without adding any code and be agnostic about them. 
It must give a very good performance for common usages of websites, but will sacrifice small performance for modularity and easier maintenance if needed.
## Features
- configuration file
- multiple backend service possible, based on HOST header to decide where to redirect.
- well thought expiration of cache (thanks [moka](https://github.com/moka-rs/moka))
- add etag header
- return non modified status when client has a valid etag 
- takes into account Vary header from server (will save different cache object for every variation of the specified header)
- let backend service decide his own caching controls.
- admin API
  - update rules of redirection without restart or loosing current cache.
  - cache invalidation
  - update fallback
  - get raw cache content
  - get stats of cache 
## Usage
Configure your reverse proxy to redirect requests you want to cache on Mnemosyne.  
**Warning**: make sure your reverse proxy does not apply unwanted modification on HOST header of your requests.  
Configure at start the HOST value that will trigger a redirection to a backend service or use the administrator API of Mnemosyne to configure it at runtime.  

