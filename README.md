# dumbrouter
dumbrouter is an HTTP reverse proxy that uses docker container names to
determine routes.

it is intentionally dumb (per the name) and is used for projects i run.

when connecting to `service.example.org`, dumbrouter will try to route:
(with service == `_root` if no subdomain)
1. to a container named `http-service`
2. to a container named `http-prod-service`

if there are three services named `http-service1`, `http-service2`, 
`http-service3`, etc. it will route to a random one.

this is a primitive starts with check so be careful.

## support
there is no support for this project

## to (eventually) do
1. bidirectional support
2. HTTP/2
