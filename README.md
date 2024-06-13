# Mnemosyne
Mnemosyne is a http request caching in memory proxy API.
## Status of development
Work in progress, not functional.
## Description
Mnemosyne is placed between your load balancer (ex: nginx) and your server applications that needs their requests to be cached. It will optimize the resources by caching responses and adding caching headers that will ask clients to re-use the cache locally. The cache will be expired based on activity and from manual invalidation.
## Objectives
This software is meant to add caching capability to your backend service without adding any code and be agnostic about them. 
It must give a very good performance for common usages of websites, but will sacrifice small performance for modularity and easier maintenance.
For example, the code will always be safe even if unsafe could bring more perfomance.   
## Features
- configuration file
- multiple endpoints possible
- cache invalidation api
- well thought expiration of cache
- use etag and and non-modified headers. Etag takes into account Vary header from server.
- let server decide his own caching controls.
