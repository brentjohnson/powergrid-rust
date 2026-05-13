use crate::types::Resource;
use serde::{Deserialize, Serialize};

/// Ephemeral hint broadcast from one client to peers. Never touches game state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HintPayload {
    /// BuyResources: the player's current shopping cart.
    Cart { items: Vec<(Resource, u8)> },
    /// BuildCities: the player's pending city selections and route edges.
    BuildSelection {
        cities: Vec<String>,
        /// Sender-computed route polyline (mirrors build_preview.edges).
        edges: Vec<(String, String)>,
    },
    /// Player cleared their selection (phase ended / turn changed).
    Clear,
}
