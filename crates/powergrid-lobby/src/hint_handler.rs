use crate::{rooms::RoomManager, ws::ConnState};
use powergrid_core::actions::{HintPayload, ServerMessage};
use std::sync::Arc;
use tracing::warn;

pub async fn handle_room_hint(
    room_name: String,
    hint: HintPayload,
    conn: &ConnState,
    manager: &Arc<RoomManager>,
) {
    let room_arc = match manager.get(&room_name).await {
        None => return,
        Some(r) => r,
    };

    {
        let room = room_arc.lock().await;
        if !room.humans.iter().any(|(id, _)| *id == conn.user_id) {
            warn!(
                "Hint from {} rejected: not in room '{}'",
                conn.user_id, room_name
            );
            return;
        }
    }

    let msg = ServerMessage::PeerHint {
        player_id: conn.user_id,
        hint,
    };
    room_arc.lock().await.session.broadcast(&msg);
}
