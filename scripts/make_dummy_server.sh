#!/bin/sh

NUMBER="$@"

docker rm -f "http-dummyserver-$NUMBER"
docker run --name "http-dummyserver-$NUMBER" -e "NUMBER=$NUMBER" -d -p "809$NUMBER":80 dummyserver
