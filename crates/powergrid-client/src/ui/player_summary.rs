use egui::{RichText, Ui};
use powergrid_core::{
    income_for,
    types::{PlantKind, PlayerId, Resource},
    GameStateView,
};

use crate::{card_painter, state::player_color_to_egui, theme};

use super::helpers::{resource_name, section_header};

pub(super) fn player_summary(ui: &mut Ui, gs: &GameStateView, my_id: PlayerId) {
    let Some(p) = gs.player(my_id) else {
        return;
    };

    let player_color = player_color_to_egui(p.color);

    // Name / color header
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

    // Money + cities
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

    // Income preview based on optimal firing
    let (_, max_cities_powered, _) = p.optimal_firing_subset();
    let cities_owned = p.city_count() as u8;
    let effective = max_cities_powered.min(cities_owned);
    ui.label(
        RichText::new(format!("Income: {} elektro", income_for(effective)))
            .color(theme::TEXT_DIM)
            .small()
            .monospace(),
    );

    ui.add_space(6.0);

    // Resources
    section_header(ui, "FUEL");
    for resource in [
        Resource::Coal,
        Resource::Oil,
        Resource::Gas,
        Resource::Uranium,
    ] {
        let held = p.resources.get(resource);
        let cap = p.resource_capacity(resource);
        if cap == 0 {
            continue;
        }
        let fill_color = if held == 0 {
            theme::TEXT_DIM
        } else {
            theme::TEXT_BRIGHT
        };
        ui.label(
            RichText::new(format!("{:>7}: {}/{}", resource_name(resource), held, cap))
                .color(fill_color)
                .small()
                .monospace(),
        );
    }

    ui.add_space(6.0);

    // Plants
    if p.plants.is_empty() {
        ui.label(
            RichText::new("No plants")
                .color(theme::TEXT_DIM)
                .small()
                .monospace(),
        );
    } else {
        section_header(ui, "PLANTS");
        for plant in &p.plants {
            let kind_label = match plant.kind {
                PlantKind::Coal => "coal",
                PlantKind::Oil => "oil",
                PlantKind::GasOrOil => "gas/oil",
                PlantKind::Gas => "gas",
                PlantKind::Uranium => "uranium",
                PlantKind::Wind => "wind",
            };
            let city_word = if plant.cities == 1 { "city" } else { "cities" };
            // Show stored fuel relevant to this plant
            let stored = match plant.kind {
                PlantKind::Coal => format!("{}c", p.resources.coal),
                PlantKind::Oil => format!("{}o", p.resources.oil),
                PlantKind::Gas => format!("{}g", p.resources.gas),
                PlantKind::GasOrOil => {
                    format!("{}g+{}o", p.resources.gas, p.resources.oil)
                }
                PlantKind::Uranium => format!("{}u", p.resources.uranium),
                PlantKind::Wind => "—".to_string(),
            };
            let needs = plant.cost;
            let cap = plant.cost * 2;
            ui.horizontal(|ui| {
                card_painter::draw_plant_card(ui, plant);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(format!("#{} {} {}", plant.number, plant.cost, kind_label))
                            .color(theme::TEXT_MID)
                            .small()
                            .monospace(),
                    );
                    ui.label(
                        RichText::new(format!("→ {} {}", plant.cities, city_word))
                            .color(theme::TEXT_DIM)
                            .small()
                            .monospace(),
                    );
                    if plant.kind != PlantKind::Wind {
                        ui.label(
                            RichText::new(format!("fuel: {} / {} cap {}", stored, needs, cap))
                                .color(theme::TEXT_DIM)
                                .small()
                                .monospace(),
                        );
                    }
                });
            });
        }
    }
}
