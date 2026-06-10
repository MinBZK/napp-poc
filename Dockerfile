# syntax=docker/dockerfile:1
# Productie-image: Rust-backend + Vue-frontend + WASM-engine in één rootless container.
#
# De backend heeft path-dependencies op ../regelrecht (engine + auth), dus de
# build clonet regelrecht naast de napp-checkout. Pin desgewenst een ref via
# `--build-arg REGELRECHT_REF=<tag-of-sha>`.

ARG REGELRECHT_REF=main

# ---------------------------------------------------------------------------
# Stage 1: Rust — backend-binary en WASM-engine voor de scenario-runner
# ---------------------------------------------------------------------------
FROM rust:1.96-alpine AS rust-builder

RUN apk add --no-cache git musl-dev

WORKDIR /build
ARG REGELRECHT_REF
RUN git clone --depth 1 --branch "${REGELRECHT_REF}" \
        https://github.com/MinBZK/regelrecht.git regelrecht \
    # De gepinde toolchain van regelrecht is hier niet relevant; bouw met de
    # toolchain van het image in plaats van een extra rustup-download.
    && rm -f regelrecht/rust-toolchain.toml

COPY . napp/

# Backend (musl, draait op kale alpine)
RUN cargo build --release --bin napp-backend --manifest-path napp/Cargo.toml

# WASM-engine; wasm-bindgen-cli moet exact de crate-versie uit regelrecht's
# lockfile zijn, dus lees die daaruit
RUN rustup target add wasm32-unknown-unknown \
    && WB_VERSION=$(awk '/^name = "wasm-bindgen"$/ {getline; gsub(/"/, "", $3); print $3}' \
        regelrecht/packages/Cargo.lock) \
    && cargo install "wasm-bindgen-cli@${WB_VERSION}" --locked
RUN cargo build --manifest-path regelrecht/packages/engine/Cargo.toml \
        --features wasm --target wasm32-unknown-unknown --release \
    && wasm-bindgen --target web --out-dir /build/wasm-pkg \
        regelrecht/packages/target/wasm32-unknown-unknown/release/regelrecht_engine.wasm

# ---------------------------------------------------------------------------
# Stage 2: frontend — vite bundelt law/ en scenarios/ at build-time mee
# ---------------------------------------------------------------------------
FROM node:24-alpine AS frontend-builder

WORKDIR /build
COPY frontend/package.json frontend/package-lock.json frontend/
RUN cd frontend && npm ci

COPY law/ law/
COPY scenarios/ scenarios/
COPY frontend/ frontend/
COPY --from=rust-builder /build/wasm-pkg frontend/public/wasm/pkg
RUN cd frontend && npm run build

# ---------------------------------------------------------------------------
# Stage 3: runtime — rootless, alleen binary + statische assets + wetteksten
# ---------------------------------------------------------------------------
FROM alpine:3.23

RUN apk add --no-cache ca-certificates \
    && addgroup -S napp && adduser -S -G napp -u 10001 napp \
    && mkdir /data && chown napp:napp /data

WORKDIR /app
COPY --from=rust-builder /build/napp/target/release/napp-backend ./napp-backend
COPY --from=frontend-builder /build/frontend/dist ./frontend/dist
COPY law/ ./law/

ENV NAPP_LAW_DIR=/app/law \
    NAPP_STATIC_DIR=/app/frontend/dist \
    NAPP_PORT=8400 \
    DATABASE_URL="sqlite:/data/napp.db?mode=rwc"

USER napp
EXPOSE 8400
CMD ["/app/napp-backend"]
