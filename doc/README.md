# DOCUMENTATION Mnemosyne
## About
Mnemosyne is a http caching proxy made to save resources on server side by caching response from backend service and on client side using etag and not modified headers.
It does not support non http connections for now.
It offers an API to manage the cache and invalidate entries, so backend service can trigger the cache to remove obsolete cache entries without waiting for a timer.
## Configuration file
The configuration file is expected to be on the path /etc/mnemosyne/config.toml It needs to have read/write permission of the user running Mnemosyne.
The configuration format is toml.
```,ignore
## which address:port Mnemosyne will listen to
listen_address = "127.0.0.1:9830"
## for a HOST header, redirect to address.
## If it's not precised enough for your scenario, you could make your reverse proxy put a custom HOST header for different path.
endpoints = [["example.net","http://127.0.0.1:9934"]]
## if the HOST of the request does not exists in the "endpoints" var, redirect to this address.
fall_back_endpoint = "http://127.0.0.1:1000/"

## cache configuration
[cache]
## Size in Megabytes before most unused entries will be deleted.
size_limit = 250
## time in seconds before unused entres will be deleted.
expiration = 2592000
```
## Integrating in your reverse-proxy
Your reverse proxy must send the request to Mnemosyne that will redirect them to their respective backend service depending on the HOST header.
### Example nginx
```,ignore
location / {
    ## redirect to Mnemosyne
    proxy_pass http://127.0.0.1:9830;
    ## keep the same HOST header
    proxy_set_header Host $host;
```
## Admin API
The admin API should be protected by an authentication. Mnemosyne does not have any, you must choose one yourself and protect the endpoint /api with it.
You can access the OpenAPI document file on /openapi.json and view it with a OpenAPI document viewer like Swagger.
