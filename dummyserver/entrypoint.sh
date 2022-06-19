#!/bin/sh

touch index.html
echo "<h1>this is dummy server $NUMBER" > index.html
caddy run --config Caddyfile --adapter caddyfile
