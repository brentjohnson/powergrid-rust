use crate::rooms::Room;
use powergrid_bot_strategy::strategy;
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tracing::{info, warn};

const MAX_BOT_ITERATIONS: usize = 50;

/// Drive all in-process bots in `room_arc` until none has a move or the cap is hit.
/// The lock is released during each delay so humans can still receive state updates.
pub async fn run_bot_pump(room_arc: Arc<Mutex<Room>>, delay: Duration) {
    for iter in 0..MAX_BOT_ITERATIONS {
        let next = {
            let room = room_arc.lock().await;
            room.session
                .bots
                .iter()
                .find_map(|b| strategy::decide(&room.session.game, b.id).map(|a| (b.id, a)))
        };

        let Some((bot_id, action)) = next else {
            return;
        };

        tokio::time::sleep(delay).await;

        let mut room = room_arc.lock().await;
        match room.session.apply(bot_id, action) {
            Ok(()) => {
                info!(
                    "Bot {} acted in room '{}' (iter {})",
                    bot_id, room.name, iter
                );
            }
            Err(e) => {
                warn!(
                    "Bot {} in room '{}' produced invalid action: {}",
                    bot_id, room.name, e
                );
            }
        }
    }

    let room = room_arc.lock().await;
    warn!(
        "Bot pump hit MAX_BOT_ITERATIONS ({}) in room '{}'",
        MAX_BOT_ITERATIONS, room.name
    );
}
