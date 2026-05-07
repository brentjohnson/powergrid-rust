mod auth;
mod card_painter;
mod effects;
mod local;
mod map_panel;
mod state;
mod theme;
mod ui;
mod ws;

use local::LocalHandle;
use state::{AppState, CliArgs};
use std::time::Duration;
use ui::UiAction;
use ws::WsChannels;

fn main() -> eframe::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = CliArgs::parse();
    let app_state = AppState::new(cli);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Power Grid: Reimagined")
            .with_inner_size([1600.0, 900.0])
            .with_fullscreen(app_state.fullscreen),
        ..Default::default()
    };

    eframe::run_native(
        "Power Grid: Reimagined",
        options,
        Box::new(|_cc| Ok(Box::new(PowerGridApp::new(app_state)))),
    )
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

struct PowerGridApp {
    state: AppState,
    ws: Option<WsChannels>,
    local: Option<LocalHandle>,
}

impl PowerGridApp {
    fn new(state: AppState) -> Self {
        Self {
            state,
            ws: None,
            local: None,
        }
    }
}

impl eframe::App for PowerGridApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Per-frame polling: drain background thread results before drawing.
        state::process_auth_events(&mut self.state);
        ws::process_ws_events(&mut self.state, self.ws.as_ref());

        // Auto-connect for online play: trigger once when pending and not yet connected.
        if self.state.pending_connect && self.ws.is_none() {
            let url = self.state.ws_url();
            self.ws = Some(ws::spawn_ws(url));
        }

        // Auto-refresh room list (every 10s when in RoomBrowser).
        if self.state.screen == state::Screen::RoomBrowser {
            let now = ctx.input(|i| i.time);
            if now - self.state.room_list_last_refresh >= 10.0 {
                if let Some(ch) = &self.ws {
                    ch.send_lobby(powergrid_core::actions::LobbyAction::ListRooms);
                }
                self.state.room_list_last_refresh = now;
            }
        }

        // Keep the app responsive while a session or auth request is in flight.
        if self.ws.is_some() || self.state.auth_in_flight {
            ctx.request_repaint_after(Duration::from_millis(50));
        }

        // Draw UI and collect deferred actions.
        let action = ui::ui_system(ctx, &mut self.state, self.ws.as_ref());

        // Apply side-effects after the egui pass.
        match action {
            UiAction::None => {}
            UiAction::StartLocal(cfg) => {
                let (channels, handle) = local::start_local_session(cfg);
                self.ws = Some(channels);
                self.local = Some(handle);
            }
            UiAction::ExitToMenu => {
                self.local = None;
                self.ws = None;
            }
            UiAction::ToggleFullscreen => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.state.fullscreen));
                if let Err(e) = auth::save_preferences(&auth::UserPreferences {
                    fullscreen: self.state.fullscreen,
                }) {
                    tracing::warn!("Failed to save preferences: {e}");
                }
            }
            UiAction::Exit => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }

        // Drop session channels when disconnected and not pending a reconnect.
        // (The WS worker reconnects automatically; only clear on an explicit exit.)
        let _ = frame; // frame unused but required by trait
    }
}
