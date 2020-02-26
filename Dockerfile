FROM rust:alpine3.11
RUN apk add libsodium-dev
WORKDIR /usr/src/brchd
COPY . .
RUN cargo build --release --verbose
RUN strip target/release/brchd

FROM alpine:3.11
RUN apk add libsodium
COPY --from=0 /usr/src/brchd/target/release/brchd /usr/local/bin/brchd
ENTRYPOINT ["brchd"]
