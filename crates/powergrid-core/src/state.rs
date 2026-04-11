use serde::{Deserialize, Serialize};
use crate::map::Map;
use crate::types::{Phase, Player, PlayerId, PlantMarket, ResourceMarket};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub phase: Phase,
    pub players: Vec<Player>,
    /// Turn order (indices into `players`), recalculated each round.
    pub player_order: Vec<PlayerId>,
    pub market: PlantMarket,
    pub resources: ResourceMarket,
    pub map: Map,
    pub round: u32,
    /// Number of cities needed to trigger end-game (depends on player count).
    pub end_game_cities: u8,
    /// Log of recent events for display.
    pub event_log: Vec<String>,
}

impl GameState {
    pub fn new(map: Map, player_count: usize) -> Self {
        let end_game_cities = match player_count {
            2 => 21,
            3 => 17,
            4 => 17,
            5 => 15,
            _ => 14, // 6 players
        };

        Self {
            phase: Phase::Lobby,
            players: Vec::new(),
            player_order: Vec::new(),
            market: crate::rules::build_plant_deck(),
            resources: ResourceMarket::initial(),
            map,
            round: 0,
            end_game_cities,
            event_log: Vec::new(),
        }
    }

    pub fn player(&self, id: PlayerId) -> Option<&Player> {
        self.players.iter().find(|p| p.id == id)
    }

    pub fn player_mut(&mut self, id: PlayerId) -> Option<&mut Player> {
        self.players.iter_mut().find(|p| p.id == id)
    }

    pub fn host_id(&self) -> Option<PlayerId> {
        self.players.first().map(|p| p.id)
    }

    pub fn log(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        self.event_log.push(msg);
        if self.event_log.len() > 50 {
            self.event_log.remove(0);
        }
    }
}
