FROM rust:alpine3.11
ENV RUSTFLAGS="-C target-feature=-crt-static"
RUN apk add musl-dev libsodium-dev openssl-dev
WORKDIR /usr/src/brchd
COPY . .
RUN cargo build --release --verbose
RUN strip target/release/brchd

FROM alpine:3.11
RUN apk add libgcc libsodium openssl
COPY --from=0 /usr/src/brchd/target/release/brchd /usr/local/bin/brchd
ENTRYPOINT ["brchd"]
