FROM rust:1.88-bookworm AS chef

# RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
RUN cargo install cargo-chef

WORKDIR /app

FROM chef AS planner
COPY . .

RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
# Install `dx`
RUN cargo install dioxus-cli --version 0.7.0-alpha.3 --root /.cargo
ENV PATH="/.cargo/bin:$PATH"

COPY --from=planner /app/recipe.json recipe.json
RUN rustup target add wasm32-unknown-unknown
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .

# Create the final bundle folder. Bundle executes in debug mode by default.
RUN dx bundle --platform web --release

# We do not need the Rust toolchain to run the binary!
FROM debian:bookworm-slim AS runtime

# Install tini for proper signal handling
RUN apt-get update && apt-get install -y tini libssl-dev && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/dx/gsazure/release/web/ /usr/local/app

# set our port and make sure to listen for all connections
ENV PORT=8080
ENV IP=0.0.0.0

# expose the port 8080
EXPOSE 8080

WORKDIR /usr/local/app
ENTRYPOINT ["/usr/bin/tini", "--"]
CMD [ "/usr/local/app/gsazure" ]
