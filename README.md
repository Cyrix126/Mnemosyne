# Mnemosyne
Mnemosyne is a http request caching in memory proxy API.
## Status of development
Work in progress, not functional.
## Description
Mnemosyne is placed between your load balancer (ex: nginx) and your server applications that needs their requests to be cached. It will optimize the resources by caching responses and adding caching headers that will ask clients to re-use the cache locally. The cache will be expired based on activity and from manual invalidation.
## Features
- configuration file
- multiple endpoints possible
- cache invalidation api
- well thought expiration of cache
- use etag, vary and non-modified headers
- let server decide hiw own caching controls.



