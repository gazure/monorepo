# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

### Workspace-wide

```bash
cargo +nightly fmt --all # format all code
cargo +nightly fmt --all -- --check # check formatting (CI)
cargo clippy --profile ci --all --all-targets --all-features --examples --tests   # lint (CI)
cargo check --profile ci --workspace --all-targets --all-features # type-check (CI)
cargo test -p <crate> # test a specific crate
```

Clippy is configured with `pedantic = deny`. New code must pass clippy and fmt before committing.

### ArenaBuddy desktop app

```bash
dx serve --platform desktop           # run in dev mode
dx bundle --release --package arenabuddy  # release build
```

Requires the Dioxus CLI (`cargo binstall dioxus-cli --version 0.7.3`).

### ArenaBuddy server

```bash
cargo run -p arenabuddy_server        # run gRPC server locally
# or via Docker Compose:
docker compose -f arenabuddy/server/docker-compose.yml up
```

### ArenaBuddy CLI

```bash
cargo run -p arenabuddy_cli -- --help
cargo run -p arenabuddy_cli -- parse --player-log /path/to/Player.log
cargo run -p arenabuddy_cli -- scrape
```

### Bevy games / other projects

```bash
cargo run --release -p baseball
cargo run --release -p twotris
cargo run --release -p life
dx serve --platform web --package gsazure
dx serve --platform web --package christmas
```

## Architecture

### Workspace layout

```
arenabuddy/
  core/      – domain models, protobuf definitions, card DB (arenabuddy_core)
  data/      – SQLx/PostgreSQL storage layer (arenabuddy_data)
  cli/       – log parser + card scraper CLI (arenabuddy_cli)
  arenabuddy/– Dioxus desktop UI
  server/    – Tonic gRPC backend (arenabuddy_server)
baseball/    – Bevy baseball game + rules engine
twotris/     – Bevy Tetris clone
life/        – Bevy Game of Life
gsazure/     – Dioxus personal portfolio (web-only)
christmas/   – Dioxus fullstack drawing app
sports/      – BaseballRef scraper and analysis tools
lib/
  start/     – Dioxus app initialization helper
  tracingx/  – tracing-subscriber wrapper
  multimap/  – generic multi-value map
```

### ArenaBuddy data flow

Desktop UI (Dioxus) → gRPC (Tonic) → `arenabuddy_server` → SQLx → PostgreSQL

`arenabuddy_core` owns all shared types and the protobuf service definitions (`arenabuddy/core/proto/`). Generated gRPC code is produced at build time via `build.rs` using `tonic-prost-build`. The server also exposes an HTTP endpoint (port 8080) alongside gRPC (port 50051).

Authentication uses JWT; Google Sheets sync uses OAuth2 (`yup-oauth2`).

### Build profiles

- `dev` – `opt-level=1` for your code, `opt-level=3` for all deps (fast iteration locally)
- `ci` – inherits `dev` without the dep optimization overrides (faster CI builds)
- `release` – full `opt-level=3` + LTO

Always use `--profile ci` in CI scripts.

### Workspace lints

- `unsafe_code = "forbid"` across the workspace
- `clippy::pedantic = "deny"` — exceptions: `missing_errors_doc`, `module_name_repetitions`, `must_use_candidate`

### Rust edition

Edition 2024, `rust-version = "1.93"`. `rustfmt` uses nightly only features and is configured for 120-character line width with grouped imports (`rustfmt.toml`).
