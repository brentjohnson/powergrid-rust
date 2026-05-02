use bevy::prelude::Commands;
use bevy_egui::egui;
use egui::RichText;
use powergrid_core::types::PlayerColor;

use crate::{
    auth::{do_logout, AuthEvent},
    state::{player_color_to_egui, AppState},
    theme, ws,
};

use super::helpers::color_label;

pub(super) fn connect_screen(ctx: &egui::Context, state: &mut AppState, commands: &mut Commands) {
    egui::CentralPanel::default()
        .frame(
            egui::Frame::NONE
                .fill(theme::BG_DEEP)
                .inner_margin(egui::Margin::same(0)),
        )
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);

                // Title
                ui.label(
                    RichText::new("POWER GRID")
                        .size(42.0)
                        .color(theme::NEON_CYAN)
                        .monospace(),
                );
                ui.label(
                    RichText::new("REIMAGINED")
                        .size(20.0)
                        .color(theme::NEON_CYAN_DIM)
                        .monospace(),
                );

                ui.add_space(40.0);

                theme::neon_frame().show(ui, |ui| {
                    ui.set_width(420.0);
                    ui.spacing_mut().item_spacing.y = 10.0;

                    // Logged-in identity
                    if let Some(ref username) = state.auth_username.clone() {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("● {}", username))
                                    .color(theme::NEON_CYAN)
                                    .monospace(),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new("[ LOG OUT ]")
                                                    .color(theme::NEON_RED)
                                                    .small()
                                                    .monospace(),
                                            )
                                            .fill(egui::Color32::TRANSPARENT)
                                            .stroke(egui::Stroke::new(1.0, theme::NEON_RED)),
                                        )
                                        .clicked()
                                    {
                                        handle_logout(state);
                                    }
                                },
                            );
                        });
                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);
                    }

                    // Server name
                    ui.label(RichText::new("SERVER NAME").color(theme::TEXT_DIM).small());
                    ui.text_edit_singleline(&mut state.server_name);

                    // Color selector
                    ui.label(
                        RichText::new("FACTION COLOR")
                            .color(theme::TEXT_DIM)
                            .small(),
                    );
                    let btn_row_id = ui.id().with("color_btn_row_width");
                    let row_width: f32 = ui.ctx().data(|d| d.get_temp(btn_row_id).unwrap_or(0.0));
                    let leading = ((ui.available_width() - row_width) / 2.0).max(0.0);
                    ui.horizontal(|ui| {
                        ui.add_space(leading);
                        let x0 = ui.cursor().left();
                        for color in [
                            PlayerColor::Red,
                            PlayerColor::Blue,
                            PlayerColor::Green,
                            PlayerColor::Yellow,
                            PlayerColor::Purple,
                            PlayerColor::White,
                        ] {
                            let egui_color = player_color_to_egui(color);
                            let selected = state.selected_color == color;
                            let label = color_label(color);

                            let btn = egui::Button::new(RichText::new(label).color(if selected {
                                egui::Color32::BLACK
                            } else {
                                egui_color
                            }))
                            .fill(if selected {
                                egui_color
                            } else {
                                theme::BG_WIDGET
                            })
                            .stroke(egui::Stroke::new(
                                if selected { 2.0 } else { 1.0 },
                                egui_color,
                            ));

                            if ui.add(btn).clicked() {
                                state.selected_color = color;
                            }
                        }
                        let measured = ui.min_rect().right() - x0;
                        ui.ctx().data_mut(|d| d.insert_temp(btn_row_id, measured));
                    });

                    ui.add_space(8.0);

                    let can_connect = state.auth_token.is_some();
                    let connect_btn = egui::Button::new(
                        RichText::new("[ CONNECT ]")
                            .color(if can_connect {
                                theme::BG_DEEP
                            } else {
                                theme::TEXT_DIM
                            })
                            .monospace(),
                    )
                    .fill(if can_connect {
                        theme::NEON_CYAN
                    } else {
                        theme::BG_WIDGET
                    })
                    .stroke(egui::Stroke::new(
                        1.5,
                        if can_connect {
                            theme::NEON_CYAN
                        } else {
                            theme::NEON_CYAN_DARK
                        },
                    ));

                    if ui.add_enabled(can_connect, connect_btn).clicked() {
                        let color = state.selected_color;
                        state.pending_join = Some(color);
                        let url = state.ws_url();
                        let channels = ws::spawn_ws(url);
                        commands.insert_resource(channels);
                    }
                });

                if !state.connected && state.pending_join.is_some() {
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new("● CONNECTING…")
                            .color(theme::NEON_AMBER)
                            .monospace(),
                    );
                }
            });
        });
}

fn handle_logout(state: &mut AppState) {
    // Fire-and-forget logout request in background
    if let (Some(token), server, port) = (
        state.auth_token.clone(),
        state.server_name.clone(),
        state.port,
    ) {
        let slot = state.auth_pending.0.clone();
        std::thread::spawn(move || {
            do_logout(&server, port, &token);
            *slot.lock().unwrap() = Some(AuthEvent::LoggedOut);
        });
        state.auth_in_flight = true;
    } else {
        state.logout();
    }
}
