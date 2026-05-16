use egui::{Align2, RichText};
use powergrid_core::{types::PlayerId, GameStateView};

use crate::theme;

pub(in crate::ui) fn game_over_overlay(ctx: &egui::Context, gs: &GameStateView, winner: PlayerId) {
    let name = gs
        .player(winner)
        .map(|p| p.name.as_str())
        .unwrap_or("UNKNOWN");

    egui::Window::new("GAME OVER")
        .collapsible(false)
        .resizable(false)
        .movable(false)
        .anchor(Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            theme::neon_frame().show(ui, |ui| {
                ui.add_space(16.0);
                ui.label(
                    RichText::new(format!("GRID CONTROLLED BY:\n{name}"))
                        .size(theme::HEADING_M)
                        .color(theme::NEON_GREEN)
                        .monospace(),
                );
                ui.add_space(16.0);
            });
        });
}
