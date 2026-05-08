# Bot AI

Bots are in-process AI opponents that play alongside human players. They use a weighted utility scoring system with per-difficulty profiles.

## Difficulty levels

Three difficulty levels are available: **Easy**, **Normal**, and **Hard**.

| Level | Behaviour |
|---|---|
| Easy | Buys plants more aggressively, overbids more, and occasionally picks a suboptimal move via Boltzmann sampling. Intended for beginners. |
| Normal | Deterministic best-move strategy. Reproduces the original hand-coded logic exactly. |
| Hard | Deterministic best-move with opponent and endgame awareness. Considers how far behind opponents it is, how close the game is to ending, and whether better plants are coming in the future deck. |

### Selecting difficulty

**Local game (GUI):** The local play setup screen shows a row for each bot. Use the dropdown on each row to pick Easy, Normal, or Hard. You can also add and remove bots individually there.

**Online lobby:** When adding a bot as room host, a difficulty dropdown appears next to the bot name and color fields.

**Standalone bot binary:**
```bash
cargo run -p powergrid-bot -- --name BotA --color red --difficulty hard
```

Default is `normal` if `--difficulty` is omitted.

---

## Tuning bot behaviour

All decision weights live in **`assets/bots/default.toml`**, embedded into the binary at compile time. Edit the file and recompile to apply changes. No code changes are needed to adjust how bots play.

Each section of the file corresponds to a game phase. The `[normal]` values reproduce the original hand-coded constants exactly; they are the safe baseline.

### Profile structure

```toml
[normal]
display_name = "Normal"
temperature  = 0.0   # 0 = always pick best; higher = sample from distribution
jitter       = 0.3   # probability of adding random bid noise
max_jitter   = 3     # maximum elektro added by bid noise

[normal.auction]
cities_weight       = 15.0   # value per city a plant can power
green_bonus         = 25.0   # bonus for Wind/Fusion (no fuel cost ever)
efficiency_weight   = 10.0   # cities × weight / fuel cost
city_reserve        = 30.0   # elektro kept in reserve for city builds
safety_buffer       = 5.0    # extra safety margin on top of city reserve
upgrade_margin      = 10.0   # minimum score improvement to justify replacing a plant
min_open_score      = 20.0   # minimum plant score to bother bidding
capacity_premium    = 2.0    # extra elektro per city of capacity gained (bid ceiling)
# Non-zero only in the hard profile:
opponent_gap_weight = 0.0    # urgency bonus per city the leader is ahead of us
endgame_weight      = 0.0    # urgency bonus proportional to endgame proximity
pipeline_weight     = 0.0    # bonus when this plant beats the upcoming deck average

[normal.buy]
fuel_reserve_multiplier = 4.0   # elektro reserved per unit of plant cost for fuel

[normal.build]
block_weight = 0.0   # bonus for building in cities opponents already occupy

[normal.bureaucracy]
oil_preference = 1.0   # 1.0 = always use oil first for hybrid plants
```

### `temperature` and how the bot picks an action

With `temperature = 0.0` (Normal and Hard), the bot always picks the highest-scoring candidate — pure deterministic play.

With `temperature > 0` (Easy defaults to `2.0`), candidates are sampled proportionally to `exp(score / temperature)`. This means:
- The best candidate is still most likely to be chosen.
- Worse candidates are occasionally chosen instead, making the bot less predictable and weaker.
- Higher temperature = more randomness.

### `jitter` and bid noise

During the auction, the bot computes a bid ceiling based on what a plant is worth to it. `jitter` (default 0.3 = 30% chance) adds 1–`max_jitter` elektro on top, making the ceiling harder for opponents to read exactly.

- Set `jitter = 0.0` for a fully predictable ceiling (useful for testing).
- Increase `max_jitter` to make bids noisier.

### Hard-profile features

The hard profile turns on three opponent/endgame features that are zero in Normal and Easy:

| Weight | Effect |
|---|---|
| `opponent_gap_weight` | Each city the leading opponent is ahead of you adds this many points to every plant's score. Bots that are behind bid more aggressively on plants. |
| `endgame_weight` | Scaled by `max_player_cities / end_game_trigger`. Near the end, all plants become more attractive. |
| `pipeline_weight` | Adds a bonus when a candidate plant scores above the average of the upcoming future deck. Prevents the bot from waiting for better plants that may never come. |
| `build.block_weight` | Each opponent already occupying a slot in a city adds this many points to that city's build priority. Hard bots deny contested cities sooner. |

---

## How decisions are made

Each game phase generates a set of **candidate actions**, scores each one, and then picks via argmax (or Boltzmann sampling for Easy).

### Auction

**Candidates:** every affordable plant in the active market, plus a Pass option.

**Scoring a plant:**
```
score = cities × cities_weight
      + green_bonus            (if Wind or Fusion)
      + (cities × efficiency_weight) / fuel_cost
      + opponent_gap_weight × max(0, leader_cities − my_cities)   [hard only]
      + endgame_weight × (max_player_cities / end_game_cities)     [hard only]
      + pipeline_weight × max(0, plant_score − avg_future_score)   [hard only]
```

**Pass** is scored at `min_open_score`. A plant must outscore Pass to be selected.

**Bid ceiling** (how high the bot will raise during an active bid):
```
ceiling = listed_price + (capacity_gained × capacity_premium)
         − fuel_reserve − city_reserve − safety_buffer
```
Round 1 caps strictly at the listed price.

**Should-skip check:** if the bot already has more generation capacity than cities owned, it skips the auction regardless of score (building cities is more valuable than stockpiling plants).

### Buy Resources

The bot ensures each plant can fire at least once, buying fuel in priority order (most cities powered first). Hybrid (coal-or-oil) plants prefer whichever fuel has more market supply, tie-breaking to oil to conserve coal for pure-coal plants.

### Build Cities

The bot only buys up to its **capacity headroom** — the number of additional cities its plants can actually power:
```
headroom = total_plant_capacity − cities_already_owned
```
Buying beyond headroom never increases income (income is based on cities powered, not owned) and wastes the city-build budget.

Within the headroom, candidates are all reachable cities in active regions where a slot is open. They are sorted by adjusted cost:
```
adjusted_cost = route_cost + slot_cost − contest_bonus
```
`contest_bonus = block_weight × already_occupied_slots` (hard only). The bot builds greedily cheapest-first, recomputing route costs after each city added to account for network expansion.

### Bureaucracy / Power Cities

The bot calls `Player::optimal_firing_subset()`, which brute-forces the best subset of plants to fire (maximises cities powered; ties broken by fuel conserved). This is already optimal and not affected by difficulty.

For hybrid plants, the bot splits fuel between coal and oil based on `oil_preference` — `1.0` means use oil first to conserve coal.

---

## Adding a new difficulty tier or personality

1. Add a new table to `assets/bots/default.toml` (e.g. `[aggressive]`).
2. Add a field to `ProfileRegistry` in `crates/powergrid-bot-strategy/src/profile.rs`.
3. Add the variant to `BotDifficulty` in `crates/powergrid-core/src/types.rs`.
4. Wire it into the UI and protocol like the existing three tiers.

---

## Standalone bot binary

The `powergrid-bot` binary connects to the legacy single-game server over WebSocket and plays a full game.

```bash
cargo run -p powergrid-server             # start the server
cargo run -p powergrid-bot -- \
  --name BotA --color red --difficulty hard    # connect a hard bot
cargo run -p powergrid-bot -- \
  --name BotB --color blue                     # connect a normal bot (default)
```

The bot reconnects automatically on disconnect and exits cleanly on game over.

---

## Code map

| File | Purpose |
|---|---|
| `assets/bots/default.toml` | All tunable weights for all difficulty profiles |
| `crates/powergrid-bot-strategy/src/profile.rs` | `BotProfile`, weight structs, TOML loading |
| `crates/powergrid-bot-strategy/src/features.rs` | Pure scoring functions (plant score, bid ceiling, etc.) |
| `crates/powergrid-bot-strategy/src/bot.rs` | `Bot` struct: identity + profile + seeded RNG |
| `crates/powergrid-bot-strategy/src/strategy.rs` | Phase-specific decision logic; `decide` compatibility shim |
| `crates/powergrid-session/src/lib.rs` | `Session::add_bot`, `Session::next_bot_action`, `run_bot_pump` |
| `crates/powergrid-lobby/src/driver.rs` | Lobby-level bot pump (same logic, room scope) |
| `crates/powergrid-bot/src/runtime.rs` | Standalone WS bot runtime |
| `crates/powergrid-core/src/types.rs` | `BotDifficulty` wire type |
| `crates/powergrid-core/src/actions/protocol.rs` | `LobbyAction::AddBot` carries `difficulty` |
