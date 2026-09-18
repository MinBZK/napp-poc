# Wat er van deze repository overblijft: een 301 naar de poc achter het
# portaal. Zie redirect/src/main.rs voor het waarom.
#
# De oude build (Rust-backend + Vue-frontend + WASM-engine, met een clone van
# regelrecht erbij) is weg: die applicatie leeft nu in de monorepo. Wat hier
# nog draait, hoeft alleen een Location-header te zetten.

FROM rust:1.90-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY redirect/ redirect/
RUN cargo build --release --bin napp-redirect

FROM alpine:3.23
RUN apk add --no-cache ca-certificates \
    && addgroup -S napp && adduser -S -G napp -u 10001 napp

COPY --from=builder /build/target/release/napp-redirect /usr/local/bin/

# Dezelfde poort als de oude backend: het ZAD-component staat op 8400 en dat
# blijft zo, zodat alleen het image wisselt en niet de bedrading eromheen.
ENV PORT=8400
USER napp
EXPOSE 8400
CMD ["napp-redirect"]
