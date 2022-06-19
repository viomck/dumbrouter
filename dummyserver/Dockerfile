FROM caddy
WORKDIR /build

COPY Caddyfile Caddyfile
COPY entrypoint.sh entrypoint.sh

CMD ["/bin/sh", "entrypoint.sh"]