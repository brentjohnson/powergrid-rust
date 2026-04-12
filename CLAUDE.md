# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Build everything
cargo build

# Run tests (game logic lives here)
cargo test -p powergrid-core

# Check types/lints
cargo cargo clippy --all-targets --all-features -- -D warnings

# Format code
cargo fmt

# Check code
cargo check

# Run a single test
cargo test -p powergrid-core test_join_and_start

# Run the server (from repo root)
MAP_FILE=maps/germany.toml cargo run -p powergrid-server

# Run the client
cargo run -p powergrid-client

# Docker
docker compose up --build
```

## Architecture

Three-crate Cargo workspace:

```
crates/
  powergrid-core/    # pure game logic, no I/O
  powergrid-server/  # axum WebSocket server
  powergrid-client/  # iced native GUI client
maps/
  germany.toml       # data-driven map graph
```

### powergrid-core

All game state and rules. The key entry point is `rules::apply_action(state, player_id, action) -> Result<(), ActionError>`. It's pure — no I/O — and fully unit-testable.

- `types.rs` — `Player`, `PowerPlant`, `ResourceMarket`, `PlantMarket`, `Phase`, `PlayerColor`, etc.
- `state.rs` — `GameState` struct (all game data including the map)
- `actions.rs` — `Action` enum (client → server), `ServerMessage` enum (server → client), `ActionError`
- `map.rs` — `Map` (runtime graph) + `MapData` (TOML-deserializable). Dijkstra routing in `Map::connection_cost_to`.
- `rules.rs` — `apply_action` dispatcher + one `handle_*` function per phase. Also `build_plant_deck()`.

**Phase flow:** `Lobby → Auction → BuyResources → BuildCities → Bureaucracy → [next round or GameOver]`

### powergrid-server

- `main.rs` — axum router: `GET /health`, `GET /ws`. Shared state is `Arc<Mutex<ServerState>>`.
- `ws.rs` — per-connection WebSocket handler. On each valid action: mutate state, broadcast full `GameState` JSON to all clients. On error: send `ActionError` only to the acting client.
- Configured via env vars: `PORT` (default 3000), `MAP_FILE` (default `maps/germany.toml`), `RUST_LOG`.

### powergrid-client

- `main.rs` — iced app entry point
- `app.rs` — `App` struct, `Message` enum, `update` / `view` / `subscription`
- `screens.rs` — `ConnectScreen`, `lobby_view`, `game_view`, `action_panel`
- `connection.rs` — WebSocket subscription via `iced::Subscription::run_with_id` + a tokio task driving the socket. Emits `WsEvent::{Connected, MessageReceived, Disconnected}`.

### Protocol

JSON over WebSocket. `Action` (tagged by `"type"` field) client→server, `ServerMessage` server→client. Full `GameState` broadcast after every valid action.

### Map format

`maps/*.toml` — list of `[[cities]]` (id, name, region) and `[[connections]]` (from, to, cost). Adding a new map requires only a new TOML file; no code changes.
