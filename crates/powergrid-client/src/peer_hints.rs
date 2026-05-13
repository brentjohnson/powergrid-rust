use powergrid_core::{
    actions::HintPayload,
    types::{PlayerId, Resource},
};
use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// PeerHints — stores the latest hint from each peer
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct PeerHints {
    pub hints: HashMap<PlayerId, HintPayload>,
}

impl PeerHints {
    pub fn set(&mut self, player_id: PlayerId, hint: HintPayload) {
        match hint {
            HintPayload::Clear => {
                self.hints.remove(&player_id);
            }
            other => {
                self.hints.insert(player_id, other);
            }
        }
    }

    pub fn clear(&mut self) {
        self.hints.clear();
    }
}

// ---------------------------------------------------------------------------
// LocalHintTracker — detects local selection changes and applies debounce
// ---------------------------------------------------------------------------

/// Tracks whether the local selection has changed since the last hint was sent.
/// Debounces to avoid flooding: emits at most once per 150 ms after a change.
pub struct LocalHintTracker {
    last_cart: HashMap<Resource, u8>,
    last_build_cities: Vec<String>,
    last_build_edges: HashSet<(String, String)>,
    /// Time the last change was detected.
    changed_at: Option<f64>,
}

impl LocalHintTracker {
    pub fn new() -> Self {
        Self {
            last_cart: HashMap::new(),
            last_build_cities: Vec::new(),
            last_build_edges: HashSet::new(),
            changed_at: None,
        }
    }

    /// Call every frame. Returns `Some(hint)` when there is a debounced hint ready to send.
    pub fn update(
        &mut self,
        cart: &HashMap<Resource, u8>,
        build_cities: &[String],
        build_edges: &HashSet<(String, String)>,
        now: f64,
    ) -> Option<HintPayload> {
        let cart_changed = cart != &self.last_cart;
        let build_changed = build_cities != self.last_build_cities.as_slice()
            || build_edges != &self.last_build_edges;

        if (cart_changed || build_changed) && self.changed_at.is_none() {
            self.changed_at = Some(now);
        }

        let changed_at = self.changed_at?;

        if now < changed_at + 0.15 {
            return None;
        }

        // Debounce elapsed — emit and update snapshots.
        self.changed_at = None;
        self.last_cart = cart.clone();
        self.last_build_cities = build_cities.to_vec();
        self.last_build_edges = build_edges.clone();

        let hint = if !cart.is_empty() {
            HintPayload::Cart {
                items: cart.iter().map(|(&r, &a)| (r, a)).collect(),
            }
        } else if !build_cities.is_empty() {
            HintPayload::BuildSelection {
                cities: build_cities.to_vec(),
                edges: build_edges.iter().cloned().collect(),
            }
        } else {
            HintPayload::Clear
        };
        Some(hint)
    }

    /// Force-reset all snapshots (e.g. on room change) without emitting.
    pub fn reset(&mut self) {
        self.last_cart.clear();
        self.last_build_cities.clear();
        self.last_build_edges.clear();
        self.changed_at = None;
    }
}
