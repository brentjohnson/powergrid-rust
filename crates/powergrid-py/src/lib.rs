use powergrid_bot_strategy::{default_registry, Bot};
use powergrid_core::{
    actions::Action,
    map::default_map,
    rules::apply_action,
    state::GameState,
    types::{
        connection_cost, BotDifficulty, Phase, PlantKind, PlayerColor, PlayerId, PlayerResources,
        PowerPlant, Resource,
    },
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use serde::Serialize;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Legal-move info
// ---------------------------------------------------------------------------

#[derive(Default, Serialize)]
struct LegalMoveInfo {
    pass_auction: bool,
    done_buying: bool,
    done_building: bool,
    select_plant_slots: Vec<usize>,
    /// Minimum legal bid amount (= active_bid.amount + 1).
    bid_min: Option<u32>,
    /// Maximum legal bid amount (= actor's money).
    bid_max: Option<u32>,
    discard_plant_slots: Vec<usize>,
    buildable_city_ids: Vec<String>,
    /// Resource indices: coal=0, oil=1, garbage=2, uranium=3
    buyable_resources: Vec<u8>,
    /// Bitmasks 0..7 over the actor's first 3 plants (sorted by number).
    power_subsets: Vec<u8>,
    /// Valid coal amounts to drop in DiscardResource (oil = drop_total - coal).
    discard_resource_coal: Vec<u8>,
    /// Valid coal amounts to use in PowerCitiesFuel (oil = hybrid_cost - coal).
    fuel_coal: Vec<u8>,
}

fn is_subset_feasible(plants: &[PowerPlant], resources: &PlayerResources, mask: u8) -> bool {
    let mut coal = resources.coal;
    let mut oil = resources.oil;
    let mut garbage = resources.garbage;
    let mut uranium = resources.uranium;

    // Pass 1: satisfy pure-fuel plants.
    for (i, plant) in plants.iter().enumerate().take(3) {
        if mask & (1 << i) == 0 {
            continue;
        }
        match plant.kind {
            PlantKind::Coal => {
                if coal < plant.cost {
                    return false;
                }
                coal -= plant.cost;
            }
            PlantKind::Oil => {
                if oil < plant.cost {
                    return false;
                }
                oil -= plant.cost;
            }
            PlantKind::Garbage => {
                if garbage < plant.cost {
                    return false;
                }
                garbage -= plant.cost;
            }
            PlantKind::Uranium => {
                if uranium < plant.cost {
                    return false;
                }
                uranium -= plant.cost;
            }
            PlantKind::Wind | PlantKind::Fusion | PlantKind::CoalOrOil => {}
        }
    }

    // Pass 2: satisfy CoalOrOil hybrid plants with remaining fuel.
    for (i, plant) in plants.iter().enumerate().take(3) {
        if mask & (1 << i) == 0 || plant.kind != PlantKind::CoalOrOil {
            continue;
        }
        let available = coal + oil;
        if available < plant.cost {
            return false;
        }
        let use_oil = plant.cost.min(oil);
        oil -= use_oil;
        coal -= plant.cost - use_oil;
    }

    true
}

fn current_actor_id(state: &GameState) -> Option<PlayerId> {
    match &state.phase {
        Phase::Lobby | Phase::PlayerOrder | Phase::GameOver { .. } => None,
        Phase::Auction {
            current_bidder_idx,
            active_bid,
            ..
        } => {
            if let Some(bid) = active_bid {
                bid.remaining_bidders.first().copied()
            } else {
                state.player_order.get(*current_bidder_idx).copied()
            }
        }
        Phase::DiscardPlant { player, .. } => Some(*player),
        Phase::DiscardResource { player, .. } => Some(*player),
        Phase::BuyResources { remaining } => remaining.first().copied(),
        Phase::BuildCities { remaining } => remaining.first().copied(),
        Phase::Bureaucracy { remaining } => remaining.first().copied(),
        Phase::PowerCitiesFuel { player, .. } => Some(*player),
    }
}

fn compute_legal_move_info(state: &GameState, actor_id: PlayerId) -> LegalMoveInfo {
    let mut info = LegalMoveInfo::default();

    let Some(player) = state.players.iter().find(|p| p.id == actor_id) else {
        return info;
    };

    match &state.phase {
        Phase::Lobby | Phase::PlayerOrder | Phase::GameOver { .. } => {}

        Phase::Auction {
            current_bidder_idx,
            active_bid,
            bought,
            passed,
        } => {
            if let Some(bid) = active_bid {
                if bid.remaining_bidders.first() == Some(&actor_id) {
                    info.pass_auction = true;
                    let min_bid = bid.amount + 1;
                    if player.money >= min_bid {
                        info.bid_min = Some(min_bid);
                        info.bid_max = Some(player.money);
                    }
                }
            } else if state.player_order.get(*current_bidder_idx) == Some(&actor_id)
                && !bought.contains(&actor_id)
                && !passed.contains(&actor_id)
            {
                if state.round > 1 || bought.contains(&actor_id) {
                    info.pass_auction = true;
                }
                // Only actual-market plants can be selected; future market is read-only.
                for (slot, plant) in state.market.actual.iter().enumerate() {
                    if player.money >= plant.number as u32 {
                        info.select_plant_slots.push(slot);
                    }
                }
            }
        }

        Phase::DiscardPlant {
            player: discard_player,
            ..
        } => {
            if *discard_player == actor_id {
                for slot in 0..player.plants.len() {
                    info.discard_plant_slots.push(slot);
                }
            }
        }

        Phase::DiscardResource {
            player: res_player,
            drop_total,
            ..
        } => {
            if *res_player == actor_id {
                for coal in 0..=*drop_total {
                    let oil = drop_total - coal;
                    if coal <= player.resources.coal && oil <= player.resources.oil {
                        info.discard_resource_coal.push(coal);
                    }
                }
            }
        }

        Phase::BuyResources { remaining } => {
            if remaining.first() == Some(&actor_id) {
                info.done_buying = true;
                for (ri, &resource) in [Resource::Coal, Resource::Oil, Resource::Garbage, Resource::Uranium]
                    .iter()
                    .enumerate()
                {
                    if player.can_add_resource(resource, 1) {
                        if let Some(cost) = state.resources.price(resource, 1) {
                            if cost <= player.money {
                                info.buyable_resources.push(ri as u8);
                            }
                        }
                    }
                }
            }
        }

        Phase::BuildCities { remaining } => {
            if remaining.first() == Some(&actor_id) {
                info.done_building = true;
                for (city_id, city) in &state.map.cities {
                    if !state.is_city_active(city_id) {
                        continue;
                    }
                    if city.owners.len() >= state.step as usize {
                        continue;
                    }
                    if player.cities.contains(city_id) {
                        continue;
                    }
                    if let Some(routing) =
                        state.map.connection_cost_to(&player.cities, city_id)
                    {
                        if routing + connection_cost(city.owners.len()) <= player.money {
                            info.buildable_city_ids.push(city_id.clone());
                        }
                    }
                }
            }
        }

        Phase::Bureaucracy { remaining } => {
            if remaining.contains(&actor_id) {
                let n = player.plants.len().min(3) as u8;
                for mask in 0u8..(1u8 << n) {
                    if is_subset_feasible(&player.plants, &player.resources, mask) {
                        info.power_subsets.push(mask);
                    }
                }
            }
        }

        Phase::PowerCitiesFuel {
            player: fuel_player,
            hybrid_cost,
            ..
        } => {
            if *fuel_player == actor_id {
                for coal in 0..=*hybrid_cost {
                    let oil = hybrid_cost - coal;
                    if coal <= player.resources.coal && oil <= player.resources.oil {
                        info.fuel_coal.push(coal);
                    }
                }
            }
        }
    }

    info
}

// ---------------------------------------------------------------------------
// Game Python class
// ---------------------------------------------------------------------------

#[pyclass]
struct Game {
    state: GameState,
}

#[pymethods]
impl Game {
    #[new]
    fn new(num_players: usize, seed: Option<u64>) -> PyResult<Self> {
        if !(2..=6).contains(&num_players) {
            return Err(PyValueError::new_err("num_players must be 2–6"));
        }
        let map = default_map();
        let state = match seed {
            Some(s) => GameState::new_with_seed(map, num_players, s),
            None => GameState::new(map, num_players),
        };
        Ok(Game { state })
    }

    /// Join all players and start the game.
    /// `colors` must be snake_case strings: "red", "blue", "green", "yellow", "purple", "white".
    fn start(&mut self, player_names: Vec<String>, colors: Vec<String>) -> PyResult<()> {
        if player_names.len() != colors.len() {
            return Err(PyValueError::new_err(
                "player_names and colors must have the same length",
            ));
        }
        let mut host_id: Option<Uuid> = None;
        let base_seed = self.state.rng_seed.unwrap_or(0);
        for (i, (name, color_str)) in player_names.iter().zip(colors.iter()).enumerate() {
            let color: PlayerColor =
                serde_json::from_value(serde_json::Value::String(color_str.clone()))
                    .map_err(|e| {
                        PyValueError::new_err(format!("invalid color '{}': {}", color_str, e))
                    })?;
            // Deterministic UUID derived from seed+index so reset() with the same seed
            // produces identical agent IDs (required for reproducibility).
            let id = if base_seed != 0 {
                let lo = base_seed
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(i as u64);
                let hi = base_seed
                    .wrapping_mul(1442695040888963407)
                    .wrapping_add(i as u64 + 1);
                Uuid::from_u128((hi as u128) << 64 | lo as u128)
            } else {
                Uuid::new_v4()
            };
            apply_action(
                &mut self.state,
                id,
                Action::JoinGame {
                    name: name.clone(),
                    color,
                },
            )
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
            if host_id.is_none() {
                host_id = Some(id);
            }
        }
        if let Some(hid) = host_id {
            apply_action(&mut self.state, hid, Action::StartGame)
                .map_err(|e| PyValueError::new_err(e.to_string()))?;
        }
        Ok(())
    }

    /// Serialized `GameStateView` as a JSON string.
    fn state_json(&self) -> String {
        serde_json::to_string(&self.state.view()).expect("serialize GameStateView")
    }

    /// Player IDs in join order (same as `player_order` after `start()`).
    fn player_ids(&self) -> Vec<String> {
        self.state.players.iter().map(|p| p.id.to_string()).collect()
    }

    /// UUID string of the player whose turn it is, or None if no single actor (Lobby, GameOver).
    fn current_actor(&self) -> Option<String> {
        current_actor_id(&self.state).map(|id| id.to_string())
    }

    fn is_terminal(&self) -> bool {
        matches!(self.state.phase, Phase::GameOver { .. })
    }

    fn winner(&self) -> Option<String> {
        if let Phase::GameOver { winner } = &self.state.phase {
            Some(winner.to_string())
        } else {
            None
        }
    }

    /// Apply an action. Raises `ValueError` on invalid actions (including wrong-phase, not-your-turn, etc.).
    fn apply(&mut self, actor: &str, action_json: &str) -> PyResult<()> {
        let actor_id =
            Uuid::parse_str(actor).map_err(|e| PyValueError::new_err(e.to_string()))?;
        let action: Action = serde_json::from_str(action_json)
            .map_err(|e| PyValueError::new_err(format!("invalid action JSON: {}", e)))?;
        apply_action(&mut self.state, actor_id, action)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Ask the Rust strategy bot to decide an action for `actor`.
    /// Returns the action as a JSON string, or None if the bot has no move.
    fn bot_decide(&self, actor: &str, difficulty: &str) -> PyResult<Option<String>> {
        let actor_id =
            Uuid::parse_str(actor).map_err(|e| PyValueError::new_err(e.to_string()))?;
        let player = self
            .state
            .players
            .iter()
            .find(|p| p.id == actor_id)
            .ok_or_else(|| PyValueError::new_err("actor not found in game"))?;
        let diff = match difficulty {
            "easy" => BotDifficulty::Easy,
            "hard" => BotDifficulty::Hard,
            _ => BotDifficulty::Normal,
        };
        let registry = default_registry();
        let profile = registry.profile_for(diff).clone();
        let seed = actor_id.as_u128() as u64;
        let mut bot = Bot::new(actor_id, player.name.clone(), player.color, profile, seed);
        Ok(bot
            .decide(&self.state)
            .map(|a| serde_json::to_string(&a).expect("serialize action")))
    }

    /// Sorted list of all city IDs in the map (stable across calls — use to build the city index).
    fn city_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.state.map.cities.keys().cloned().collect();
        ids.sort();
        ids
    }

    /// JSON describing which moves are legal for `actor` right now.
    /// Python uses this to build the action mask without re-implementing game rules.
    fn legal_move_info(&self, actor: &str) -> PyResult<String> {
        let actor_id =
            Uuid::parse_str(actor).map_err(|e| PyValueError::new_err(e.to_string()))?;
        let info = compute_legal_move_info(&self.state, actor_id);
        Ok(serde_json::to_string(&info).expect("serialize LegalMoveInfo"))
    }
}

// ---------------------------------------------------------------------------
// Module
// ---------------------------------------------------------------------------

#[pymodule]
fn powergrid_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Game>()?;
    Ok(())
}
