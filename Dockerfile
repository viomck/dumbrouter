FROM rust

WORKDIR /usr/src/dumbrouter
COPY . .

RUN cargo install --path .

CMD ["dumbrouter"]
