# Mnemosyne
Mnemosyne is a http request caching in memory proxy API.
## Status of development
Work in progress, not functional.
### TODO
- [x] allows to limit cache by size
- [ ] remove allocation when possible
- [ ] organize code in modules
- [ ] tracing
- [ ] tests
- [ ] benchmarks
- [ ] documentation
## Description
Mnemosyne is placed between your load balancer (ex: nginx) and your server applications that needs their requests to be cached. It will optimize the resources by caching responses and adding caching headers that will ask clients to re-use the cache locally. The cache will be expired based on activity and from manual invalidation.
## Objectives
This software is meant to add caching capability to your backend service without adding any code and be agnostic about them. 
It must give a very good performance for common usages of websites, but will sacrifice small performance for modularity and easier maintenance if needed.
## Features
- configuration file
- multiple endpoints possible
- cache invalidation api
- well thought expiration of cache (thanks moka)
- add etag header
- return non modified status when client has a valid etag 
- takes into account Vary header from server (will save different cache object for every variation of the specified header)
- let server decide his own caching controls.
