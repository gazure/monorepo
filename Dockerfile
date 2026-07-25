# Unified Dockerfile for all workspace Dioxus web apps.
#
#   docker build --build-arg APP_NAME=christmas -t christmas .
#
# APP_NAME is the cargo package name, which must also be the `[[bin]]` name.
ARG RUST_VERSION=1.96
ARG DX_VERSION=0.8.0-alpha.0

FROM rust:${RUST_VERSION}-bookworm AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
ARG APP_NAME
COPY . .
RUN cargo chef prepare --recipe-path recipe.json --bin ${APP_NAME}

FROM chef AS builder
ARG APP_NAME
ARG DX_VERSION
# Must track the `dioxus` version in Cargo.toml — the CLI and the library are
# released in lockstep and mixing majors across an alpha boundary breaks the build.
RUN cargo install dioxus-cli --version ${DX_VERSION} --locked --root /.cargo
ENV PATH="/.cargo/bin:$PATH"
RUN rustup target add wasm32-unknown-unknown

COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json --bin ${APP_NAME}

COPY . .
RUN dx bundle --platform web --release --package ${APP_NAME}

# Collapse the APP_NAME-dependent path here so the runtime stage can copy from a
# fixed location. dx emits `web/{server,public}` — the binary is named `server`,
# not after the package.
RUN cp -r target/dx/${APP_NAME}/release/web /out \
    && if [ -d "${APP_NAME}/seed" ]; then cp -r "${APP_NAME}/seed" /out/seed; fi

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates tini \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /out /usr/local/app

ENV PORT=8080
ENV IP=0.0.0.0
EXPOSE 8080

WORKDIR /usr/local/app
ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["/usr/local/app/server"]
