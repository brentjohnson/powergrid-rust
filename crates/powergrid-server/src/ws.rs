use crate::SharedState;
use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use powergrid_core::{
    actions::{Action, ServerMessage},
    rules::apply_action,
};
use tokio::sync::mpsc;
use tracing::{info, warn};
use uuid::Uuid;

pub async fn handle_socket(socket: WebSocket, state: SharedState) {
    // Temporary id for pre-join routing; replaced by client_id on JoinGame.
    let temp_id = Uuid::new_v4();
    let mut current_id = temp_id;
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    {
        let mut s = state.lock().await;
        s.clients.push((temp_id, tx.clone()));
        info!("Client connected: {temp_id}");
    }
    let welcome = serde_json::to_string(&ServerMessage::Welcome { your_id: temp_id }).unwrap();
    let _ = tx.send(welcome);

    let (mut sink, mut stream) = socket.split();

    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sink.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(msg)) = stream.next().await {
        let text = match msg {
            Message::Text(t) => t.to_string(),
            Message::Close(_) => break,
            _ => continue,
        };

        let action: Action = match serde_json::from_str(&text) {
            Ok(a) => a,
            Err(e) => {
                warn!("Malformed action from {current_id}: {e}");
                send_error(&state, current_id, format!("Invalid message: {e}")).await;
                continue;
            }
        };

        // On JoinGame, adopt the client-supplied id as the authoritative PlayerId.
        if let Action::JoinGame { client_id, .. } = &action {
            let client_id = *client_id;
            if client_id != current_id {
                let mut s = state.lock().await;
                // Re-key the connection entry: remove the temp entry and replace with client_id.
                // If an entry for client_id already exists (player is rejoining), replace its sender.
                s.clients
                    .retain(|(id, _)| *id != current_id && *id != client_id);
                s.clients.push((client_id, tx.clone()));
                current_id = client_id;
                info!("Client {temp_id} adopted player id {current_id}");
            }
        }

        info!("Action from {current_id}: {action:?}");

        let mut s = state.lock().await;
        match apply_action(&mut s.game, current_id, action) {
            Ok(()) => {
                info!(
                    "Action from {current_id} succeeded; broadcasting state to {} client(s)",
                    s.clients.len()
                );
                let msg =
                    serde_json::to_string(&ServerMessage::StateUpdate(Box::new(s.game.clone())))
                        .unwrap();
                s.clients
                    .retain(|(_, tx): &(Uuid, _)| tx.send(msg.clone()).is_ok());
            }
            Err(e) => {
                warn!("Action from {current_id} rejected: {e}");
                let err_msg = serde_json::to_string(&ServerMessage::ActionError {
                    message: e.to_string(),
                })
                .unwrap();
                if let Some((_, tx)) = s
                    .clients
                    .iter()
                    .find(|(id, _): &&(Uuid, _)| *id == current_id)
                {
                    let _ = tx.send(err_msg);
                }
            }
        }
    }

    // Disconnect: remove the current (possibly client-owned) id from the connection list.
    {
        let mut s = state.lock().await;
        s.clients.retain(|(id, _)| *id != current_id);
        info!("Client disconnected: {current_id}");
    }

    send_task.abort();
}

async fn send_error(state: &SharedState, player_id: Uuid, msg: String) {
    let err_msg = serde_json::to_string(&ServerMessage::ActionError { message: msg }).unwrap();
    let s = state.lock().await;
    if let Some((_, tx)) = s
        .clients
        .iter()
        .find(|(id, _): &&(Uuid, _)| *id == player_id)
    {
        let _ = tx.send(err_msg);
    }
}
