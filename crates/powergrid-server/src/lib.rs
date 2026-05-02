pub mod ws;

use axum::{
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Router,
};
use powergrid_session::{Map, Session, MAX_PLAYERS};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;

pub type SharedSession = Arc<Mutex<Session>>;

/// Bind on `addr` (use "127.0.0.1:0" for an ephemeral port).
/// Returns the actual bound `SocketAddr` and a future the caller must spawn.
pub async fn serve_embedded(
    map: Map,
    addr: &str,
) -> std::io::Result<(
    SocketAddr,
    impl std::future::Future<Output = std::io::Result<()>> + Send + 'static,
)> {
    let session: SharedSession = Arc::new(Mutex::new(Session::new(map, MAX_PLAYERS)));
    let app = Router::new()
        .route("/health", get(health))
        .route("/ws", get(ws_handler))
        .with_state(session);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let local_addr = listener.local_addr()?;
    Ok((local_addr, async move { axum::serve(listener, app).await }))
}

async fn health() -> &'static str {
    "ok"
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(session): State<SharedSession>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| ws::handle_socket(socket, session))
}
