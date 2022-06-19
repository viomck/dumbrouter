#!/bin/sh

docker rm -f "http-echo"
docker run -d --name "http-echo" -p "8083:80" ealen/echo-server
