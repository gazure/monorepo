# Unified Dockerfile for all workspace apps
FROM rust:1.88-bookworm AS chef

RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
# Use build arg to determine which app to prepare
ARG APP_NAME
RUN cargo chef prepare --recipe-path recipe.json --bin ${APP_NAME}

FROM chef AS builder
RUN cargo install dioxus-cli --version 0.7.0-alpha.3 --root /.cargo

ARG APP_NAME
ENV PATH="/.cargo/bin:$PATH"

COPY --from=planner /app/recipe.json recipe.json
RUN rustup target add wasm32-unknown-unknown
# Cook dependencies for the specific app
RUN cargo chef cook --release --recipe-path recipe.json --bin ${APP_NAME}

# Copy the entire workspace
COPY . .

# Build the specific app
RUN dx bundle --platform web --release --package ${APP_NAME}

FROM debian:bookworm-slim AS runtime
ARG APP_NAME
ENV APP_NAME=${APP_NAME}

# Install tini for better signal handling
RUN apt-get update && apt-get install -y tini && rm -rf /var/lib/apt/lists/*

# Copy the built app - note the path includes the app name
COPY --from=builder /app/target/dx/${APP_NAME}/release/web/ /usr/local/app

ENV PORT=8080
ENV IP=0.0.0.0

EXPOSE 8080

WORKDIR /usr/local/app
ENTRYPOINT ["/usr/bin/tini", "--"]
# Use the app name in the CMD
CMD [ "sh", "-c", "/usr/local/app/${APP_NAME}" ]
