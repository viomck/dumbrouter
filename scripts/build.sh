#!/bin/sh
# Builds dumbrouter for production

docker buildx build \
    --tag viomckinney/dumbrouter:latest \
    -o type=image \
    --platform=linux/arm64,linux/amd64 \
    --push \
    .