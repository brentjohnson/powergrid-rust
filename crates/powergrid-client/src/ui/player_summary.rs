use egui::{RichText, Ui};
use powergrid_core::{
    income_for,
    types::PlayerId,
    GameStateView,
};

use crate::{state::player_color_to_egui, theme};

pub(super) fn player_summary(ui: &mut Ui, gs: &GameStateView, my_id: PlayerId) {
    let Some(p) = gs.player(my_id) else {
        return;
    };

    let player_color = player_color_to_egui(p.color);

    ui.horizontal(|ui| {
        ui.label(RichText::new("■").color(player_color).monospace());
        ui.label(
            RichText::new(&p.name)
                .color(player_color)
                .monospace()
                .strong(),
        );
        ui.label(RichText::new("(you)").color(theme::TEXT_DIM).small());
    });

    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("${}", p.money))
                .size(theme::HEADING_M)
                .color(theme::NEON_GREEN)
                .monospace(),
        );
        ui.add_space(8.0);
        ui.label(
            RichText::new(format!("{}/{} cities", p.city_count(), gs.end_game_cities))
                .color(theme::TEXT_MID)
                .monospace(),
        );
    });

    let (_, max_cities_powered, _) = p.optimal_firing_subset();
    let cities_owned = p.city_count() as u8;
    let effective = max_cities_powered.min(cities_owned);
    ui.label(
        RichText::new(format!("Income: {} elektro", income_for(effective)))
            .color(theme::TEXT_DIM)
            .small()
            .monospace(),
    );
}
