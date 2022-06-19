#!/bin/sh

docker rm -f dumbrouter
docker run --name dumbrouter -v /var/run/docker.sock:/var/run/docker.sock "$@" viomckinney/dumbrouter
