FROM rust:alpine3.11
RUN apk add libsodium-dev
WORKDIR /usr/src/brch
COPY . .
RUN cargo build --release --verbose
RUN strip target/release/brch

FROM alpine:3.11
RUN apk add libsodium
COPY --from=0 /usr/src/brch/target/release/brch /usr/local/bin/brch
ENTRYPOINT ["brch"]
