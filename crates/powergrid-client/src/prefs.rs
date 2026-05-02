use serde::{Deserialize, Serialize};
use tracing::warn;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct ClientPrefs {
    pub client_id: Uuid,
    pub player_name: String,
}

impl Default for ClientPrefs {
    fn default() -> Self {
        Self {
            client_id: Uuid::new_v4(),
            player_name: String::new(),
        }
    }
}

/// Load preferences from the OS config directory.  Returns defaults (with a freshly generated
/// `client_id`) if the file is absent or unreadable, and persists them so the id is stable.
pub fn load() -> ClientPrefs {
    match confy::load::<ClientPrefs>("powergrid-client", None) {
        Ok(prefs) => prefs,
        Err(e) => {
            warn!("Failed to load preferences ({e}); using defaults");
            let prefs = ClientPrefs::default();
            save(&prefs);
            prefs
        }
    }
}

/// Persist preferences to the OS config directory.  Failures are logged but not fatal.
pub fn save(prefs: &ClientPrefs) {
    if let Err(e) = confy::store("powergrid-client", None, prefs) {
        warn!("Failed to save preferences: {e}");
    }
}
