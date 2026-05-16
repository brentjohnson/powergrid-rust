use egui::{RichText, Ui};
use powergrid_core::{
    actions::Action,
    types::{Phase, PlantKind, PlayerId},
    GameStateView,
};

use crate::{state::AppState, theme, ws::WsChannels};

use super::super::helpers::{neon_button, resource_counter_row, send};

pub(in crate::ui) fn power_cities_fuel_panel(
    ui: &mut Ui,
    state: &mut AppState,
    channels: Option<&WsChannels>,
    gs: &GameStateView,
    my_id: PlayerId,
) {
    let Phase::PowerCitiesFuel {
        player,
        hybrid_cost,
        plant_numbers,
        ..
    } = &gs.phase
    else {
        return;
    };

    let room_owned = state.current_room.clone();
    let room = room_owned.as_deref();

    if *player == my_id {
        let hybrid_cost = *hybrid_cost;
        let (gas_held, oil_held) = gs
            .player(my_id)
            .map(|p| (p.resources.gas, p.resources.oil))
            .unwrap_or((0, 0));

        let (pure_gas, pure_oil) = gs
            .player(my_id)
            .map(|p| {
                plant_numbers.iter().fold((0u8, 0u8), |(pg, po), &num| {
                    if let Some(pl) = p.plants.iter().find(|pl| pl.number == num) {
                        match pl.kind {
                            PlantKind::Gas => (pg + pl.cost, po),
                            PlantKind::Oil => (pg, po + pl.cost),
                            _ => (pg, po),
                        }
                    } else {
                        (pg, po)
                    }
                })
            })
            .unwrap_or((0, 0));

        let gas_avail = gas_held.saturating_sub(pure_gas);
        let oil_avail = oil_held.saturating_sub(pure_oil);

        let min_gas = hybrid_cost.saturating_sub(oil_avail);
        let max_gas = hybrid_cost.min(gas_avail);
        if state.power_fuel_gas < min_gas {
            state.power_fuel_gas = min_gas;
        }
        if state.power_fuel_gas > max_gas {
            state.power_fuel_gas = max_gas;
        }
        let oil_used = hybrid_cost - state.power_fuel_gas;

        ui.label(
            RichText::new(format!(
                "Hybrid plants need {} fuel — choose gas/oil split:",
                hybrid_cost
            ))
            .color(theme::NEON_AMBER)
            .monospace(),
        );
        ui.add_space(4.0);

        match resource_counter_row(
            ui,
            "     GAS",
            state.power_fuel_gas,
            min_gas,
            max_gas,
            &format!("avail: {}", gas_avail),
        ) {
            d if d < 0 => state.power_fuel_gas -= 1,
            d if d > 0 => state.power_fuel_gas += 1,
            _ => {}
        }

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("     OIL: {:>2}", oil_used))
                    .color(theme::TEXT_BRIGHT)
                    .monospace(),
            );
            ui.label(
                RichText::new(format!("avail: {}", oil_avail))
                    .color(theme::TEXT_DIM)
                    .monospace(),
            );
        });

        ui.add_space(4.0);
        if ui
            .add(neon_button("[ CONFIRM ]", theme::NEON_CYAN))
            .clicked()
        {
            send(
                Action::PowerCitiesFuel {
                    gas: state.power_fuel_gas,
                    oil: oil_used,
                },
                room,
                channels,
            );
        }
    } else {
        let name = gs.player(*player).map(|p| p.name.as_str()).unwrap_or("???");
        ui.label(
            RichText::new(format!("● Waiting for {} to choose fuel split…", name))
                .color(theme::TEXT_DIM)
                .monospace(),
        );
    }
}
