mod event_log;
mod helpers;
mod left_panel;
mod lobby;
mod local_setup;
mod login;
mod main_menu;
mod phases;
mod player_summary;
mod register;
mod room_browser;
mod top_panel;

use egui::RichText;
use powergrid_core::types::{Phase, PlayerColor, PlayerId};

use crate::{
    local::LocalConfig,
    state::{AppState, BottomTab, Screen},
    theme,
    ws::WsChannels,
};

/// Side-effects requested by the UI for the app to apply after the egui pass.
pub enum UiAction {
    None,
    StartLocal(LocalConfig),
    ExitToMenu,
    Exit,
    ToggleFullscreen,
}

// ---------------------------------------------------------------------------
// Main UI function (called from eframe App::update each frame)
// ---------------------------------------------------------------------------

pub fn ui_system(
    ctx: &egui::Context,
    state: &mut AppState,
    channels: Option<&WsChannels>,
) -> UiAction {
    theme::apply(ctx);

    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        state.menu_open = !state.menu_open;
    }

    if matches!(state.screen, Screen::Game)
        && !ctx.wants_keyboard_input()
        && ctx.input(|i| i.key_pressed(egui::Key::Space))
    {
        state.bottom_panel_open = !state.bottom_panel_open;
    }

    let mut action = UiAction::None;

    match state.screen {
        Screen::MainMenu => {
            main_menu::main_menu_screen(ctx, state, &mut action);
        }
        Screen::LocalSetup => {
            local_setup::local_setup_screen(ctx, state, &mut action);
        }
        Screen::Login => {
            login::login_screen(ctx, state);
        }
        Screen::Register => {
            register::register_screen(ctx, state);
        }
        Screen::RoomBrowser => {
            room_browser::room_browser_screen(ctx, state, channels);
        }
        Screen::Game => {
            game_screen(ctx, state, channels);
        }
    }

    if state.menu_open {
        egui::Window::new("MENU")
            .collapsible(false)
            .resizable(false)
            .movable(false)
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                if ui
                    .add(helpers::neon_button(
                        "[ BACK TO MAIN MENU ]",
                        theme::NEON_AMBER,
                    ))
                    .clicked()
                {
                    state.connected = false;
                    state.pending_connect = false;
                    state.my_id = None;
                    state.current_room = None;
                    state.game_state = None;
                    state.map = None;
                    state.error_message = None;
                    state.screen = Screen::MainMenu;
                    state.menu_open = false;
                    action = UiAction::ExitToMenu;
                }
                ui.add_space(4.0);
                let fs_label = if state.fullscreen {
                    "[ WINDOWED MODE ]"
                } else {
                    "[ FULLSCREEN ]"
                };
                if ui
                    .add(helpers::neon_button(fs_label, theme::NEON_CYAN))
                    .clicked()
                {
                    state.fullscreen = !state.fullscreen;
                    state.menu_open = false;
                    action = UiAction::ToggleFullscreen;
                }
                ui.add_space(4.0);
                if ui
                    .add(helpers::neon_button("[ EXIT ]", theme::NEON_RED))
                    .clicked()
                {
                    action = UiAction::Exit;
                }
                ui.add_space(4.0);
            });
    }

    action
}

// ---------------------------------------------------------------------------
// Game screen
// ---------------------------------------------------------------------------

fn game_screen(ctx: &egui::Context, state: &mut AppState, channels: Option<&WsChannels>) {
    let Some(gs) = state.game_state.clone() else {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| {
                ui.label(
                    RichText::new("● AWAITING UPLINK…")
                        .color(theme::NEON_AMBER)
                        .heading(),
                );
            });
        });
        return;
    };

    let my_id = state.my_id.unwrap_or_default();

    if matches!(gs.phase, Phase::Lobby) {
        lobby::lobby_screen(ctx, state, channels, &gs, my_id);
        return;
    }

    // GameOver overlay — rendered last so it floats above everything
    if let Phase::GameOver { winner } = gs.phase {
        phases::game_over_overlay(ctx, &gs, winner);
    }

    egui::TopBottomPanel::top("top_panel")
        .min_height(180.0)
        .frame(theme::panel_frame(6))
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    top_panel::top_panel_contents(ui, gs.clone(), state, channels, my_id);
                });
        });

    // Left panel is added before CentralPanel so it extends the full remaining height.
    egui::SidePanel::left("player_panel")
        .resizable(false)
        .exact_width(220.0)
        .frame(theme::panel_frame(0))
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(6.0);
                left_panel::left_panel_contents(ui, &gs, my_id);
            });
        });

    egui::CentralPanel::default()
        .frame(
            egui::Frame::NONE
                .fill(theme::BG_MAP)
                .inner_margin(egui::Margin::same(0)),
        )
        .show(ctx, |ui| {
            crate::map_panel::draw(ui, state, &gs, my_id);
        });

    // ── Bottom-right info panel (Space or toggle button) ──────────────────────
    if state.bottom_panel_open {
        bottom_info_panel(ctx, state, &gs);
    } else {
        // Small tab visible when panel is closed
        egui::Area::new(egui::Id::new("info_toggle_area"))
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-8.0, -8.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                if ui
                    .add(helpers::neon_button("[ ▲ INFO ]", theme::NEON_CYAN))
                    .clicked()
                {
                    state.bottom_panel_open = true;
                }
            });
    }
}

// ---------------------------------------------------------------------------
// Bottom-right tabbed info panel
// ---------------------------------------------------------------------------

const PANEL_HEIGHT: f32 = 280.0;

fn bottom_info_panel(
    ctx: &egui::Context,
    state: &mut AppState,
    gs: &powergrid_core::GameStateView,
) {
    #[allow(deprecated)]
    let panel_w = (ctx.screen_rect().width() * 0.5).max(320.0);

    egui::Window::new("info_panel")
        .title_bar(false)
        .resizable(false)
        .movable(false)
        .collapsible(false)
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::Vec2::ZERO)
        .fixed_size(egui::vec2(panel_w, PANEL_HEIGHT))
        .frame(theme::panel_frame(4))
        .show(ctx, |ui| {
            // Tab bar + collapse button
            ui.horizontal(|ui| {
                for tab in [
                    BottomTab::EventLog,
                    BottomTab::CityGraph,
                    BottomTab::Replenish,
                    BottomTab::Payout,
                ] {
                    let active = state.bottom_panel_tab == tab;
                    let color = if active {
                        theme::NEON_CYAN
                    } else {
                        theme::TEXT_DIM
                    };
                    let resp = ui.add(
                        egui::Button::new(
                            RichText::new(tab.label()).color(color).monospace().small(),
                        )
                        .fill(if active {
                            theme::BG_WIDGET
                        } else {
                            egui::Color32::TRANSPARENT
                        })
                        .stroke(egui::Stroke::new(
                            if active { 1.0 } else { 0.0 },
                            theme::NEON_CYAN,
                        )),
                    );
                    if resp.clicked() {
                        state.bottom_panel_tab = tab;
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(helpers::neon_button("[ ▼ ]", theme::NEON_CYAN))
                        .clicked()
                    {
                        state.bottom_panel_open = false;
                    }
                });
            });

            ui.separator();

            // Tab content
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| match state.bottom_panel_tab {
                    BottomTab::EventLog => {
                        event_log::event_log_contents(ui, gs);
                    }
                    BottomTab::CityGraph => {
                        if !state.city_history.is_empty() {
                            let players_info: Vec<(PlayerId, PlayerColor)> =
                                gs.players.iter().map(|p| (p.id, p.color)).collect();
                            theme::neon_frame().show(ui, |ui| {
                                top_panel::city_history_graph(
                                    ui,
                                    &state.city_history,
                                    &players_info,
                                    gs.end_game_cities,
                                    gs,
                                );
                            });
                        } else {
                            ui.label(
                                RichText::new("No city history yet.")
                                    .color(theme::TEXT_DIM)
                                    .monospace()
                                    .small(),
                            );
                        }
                    }
                    BottomTab::Replenish => {
                        theme::neon_frame().show(ui, |ui| {
                            top_panel::step_replenish_columns(ui, gs.step, gs.players.len());
                        });
                    }
                    BottomTab::Payout => {
                        theme::neon_frame().show(ui, |ui| {
                            top_panel::city_payout_table(ui, gs);
                        });
                    }
                });
        });
}
