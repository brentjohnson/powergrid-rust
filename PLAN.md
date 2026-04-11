# Powergrid — Implementation Plan

## Context

Build a digital implementation of the Powergrid board game (2–6 players) as a client-server application. Friends connect to a shared server; all game logic lives server-side. The client is a native cross-platform GUI. The goal is a working, playable v1 with the base game core loop — auction, buy resources, build cities, bureaucracy — with edge cases deferred. The server should be cloud-deployable from day one.

---

## Architecture

### Workspace layout

```
powergrid/
├── Cargo.toml              # workspace
├── Dockerfile              # multi-stage build of server
├── docker-compose.yml
├── maps/
│   └── germany.toml        # map graph data (cities + connection costs)
└── crates/
    ├── powergrid-core/     # shared: game state, types, rules, serialization
    ├── powergrid-server/   # axum WebSocket server
    └── powergrid-client/   # iced GUI client
```

### Crate responsibilities

**`powergrid-core`** (no I/O, pure logic)
- All game types: `GameState`, `Phase`, `Player`, `PowerPlant`, `ResourceMarket`, `Map`, etc.
- All `Action` and `ServerMessage` types (serde-serializable)
- State transition logic: `apply_action(state, action) -> Result<GameState, ActionError>`
- Map loading from TOML (`maps/*.toml`)
- Unit-testable in isolation — no network, no GUI

**`powergrid-server`**
- `axum` HTTP + WebSocket server
- One `GameState` held in an `Arc<Mutex<GameState>>`
- On each valid client `Action`: mutate state, broadcast full `GameState` to all connected clients
- HTTP `GET /health` for cloud health checks
- Loads map from a path (env var or CLI arg)

**`powergrid-client`**
- `iced` application
- WebSocket connection to server (reconnects on drop)
- Receives `GameState` updates → re-renders
- Sends `Action` messages on player input
- Screens: Connect/Join → Lobby → Game board

---

## Key dependencies

| Crate | Purpose |
|---|---|
| `tokio` | Async runtime (server + client) |
| `axum` | HTTP + WebSocket server |
| `tokio-tungstenite` | WebSocket client (in iced app) |
| `serde` + `serde_json` | Message serialization |
| `toml` + `serde` | Map data loading |
| `iced` | Native GUI |
| `uuid` | Player IDs |
| `tracing` + `tracing-subscriber` | Logging |

---

## Protocol

All messages are JSON over WebSocket.

**Client → Server (`Action` enum):**
```
JoinGame { name: String, color: Color }
StartGame
BidOnPlant { plant_id: u8, bid: u32 }
PassAuction
BuyResources { resource: Resource, amount: u8 }
BuildCity { city_id: String }
PowerCities { plant_ids: Vec<u8> }
```

**Server → Client (`ServerMessage` enum):**
```
StateUpdate(GameState)       // broadcast after every valid action
ActionError(String)          // sent only to the acting client
```

Full `GameState` is broadcast after each action (simple, no delta tracking — fine for board game state sizes).

---

## Game phases (MVP core loop)

```
Setup → PlayerOrder → Auction → BuyResources → BuildCities → Bureaucracy → [loop or EndGame]
```

Each phase is a variant of the `Phase` enum. `apply_action` returns `ActionError` for actions not valid in the current phase.

**Deferred for post-MVP:**
- Step 2 / Step 3 transitions and the "remove highest plant" rules
- Auction pass edge cases (must buy if you haven't yet)
- Tie-breaking for player order beyond basic city count
- Expansion maps

---

## Map data format (`maps/germany.toml`)

```toml
name = "Germany"
regions = ["northwest", "northeast", ...]   # for area-based rules

[[cities]]
id = "hamburg"
name = "Hamburg"
region = "northwest"

[[connections]]
from = "hamburg"
to = "bremen"
cost = 4
```

---

## Server setup (cloud-ready)

- Single binary, configured via env vars: `PORT`, `MAP_FILE`
- Multi-stage `Dockerfile`: `rust:alpine` builder → `alpine` runtime image
- `docker-compose.yml` for local dev: mounts `maps/` dir, exposes port

---

## Implementation order

1. **`powergrid-core`**: types, `GameState`, `Phase`, `apply_action` skeleton, map loading
2. **`powergrid-server`**: WebSocket connection handling, state broadcast, action routing
3. **Germany map data**: `maps/germany.toml`
4. **`powergrid-client`**: connect screen → lobby screen → game state display (read-only first)
5. **Game phases one at a time**: PlayerOrder → Auction → BuyResources → BuildCities → Bureaucracy
6. **Dockerfile + docker-compose**

---

## Verification

- `cargo test -p powergrid-core` — unit test each phase transition independently, no server needed
- Run server locally: `cargo run -p powergrid-server -- --map maps/germany.toml`
- Run two client instances, join the same game, step through a full round
- `docker build` succeeds and server starts in container
- `GET /health` returns 200
