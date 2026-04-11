use futures::{SinkExt, Stream, StreamExt};
use iced::futures::channel::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use powergrid_core::actions::{Action, ServerMessage};

#[derive(Debug, Clone)]
pub enum WsEvent {
    Connected(mpsc::Sender<Action>),
    MessageReceived(ServerMessage),
    Disconnected,
}

pub fn connect(url: String) -> iced::Subscription<WsEvent> {
    iced::Subscription::run_with_id(
        std::any::TypeId::of::<WsEvent>(),
        event_stream(url),
    )
}

fn event_stream(url: String) -> impl Stream<Item = WsEvent> + Send + 'static {
    let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel::<WsEvent>();
    tokio::spawn(ws_worker(url, event_tx));
    tokio_stream::wrappers::UnboundedReceiverStream::new(event_rx)
}

async fn ws_worker(
    url: String,
    event_tx: tokio::sync::mpsc::UnboundedSender<WsEvent>,
) {
    loop {
        let ws_stream = match connect_async(&url).await {
            Ok((s, _)) => s,
            Err(_) => {
                let _ = event_tx.send(WsEvent::Disconnected);
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                continue;
            }
        };

        let (mut write, mut read) = ws_stream.split();
        let (action_tx, mut action_rx) = mpsc::channel::<Action>(32);

        let _ = event_tx.send(WsEvent::Connected(action_tx));

        loop {
            tokio::select! {
                Some(action) = action_rx.next() => {
                    let json = serde_json::to_string(&action).unwrap();
                    if write.send(Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&text) {
                                if event_tx.send(WsEvent::MessageReceived(server_msg)).is_err() {
                                    return;
                                }
                            }
                        }
                        _ => break,
                    }
                }
            }
        }

        let _ = event_tx.send(WsEvent::Disconnected);
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }
}
