# syntax=docker/dockerfile:1

ARG RUST_VERSION=1.82.0
ARG APP_NAME=timet-tui

FROM rust:${RUST_VERSION} AS build
ARG APP_NAME
WORKDIR /app

RUN --mount=type=bind,source=src,target=src \
    --mount=type=bind,source=build.rs,target=build.rs \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=/usr/local/cargo/git/db \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    cargo build --locked --release && \
    cp ./target/release/$APP_NAME /bin/$APP_NAME

FROM gcr.io/distroless/cc-debian12 AS final

LABEL io.whalebrew.config.environment '["TIMET_CONFIG_HOME", "TIMET_API_KEY"]'
LABEL io.whalebrew.config.volumes '["~/.config:/bin/config:rw"]'

WORKDIR /bin
COPY --from=build /bin/timet-tui /bin/

ENTRYPOINT  ["/bin/timet-tui"]
