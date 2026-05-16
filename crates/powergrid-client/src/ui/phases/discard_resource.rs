use egui::{RichText, Ui};
use powergrid_core::{actions::Action, types::Phase, types::PlayerId, GameStateView};

use crate::{state::AppState, theme, ws::WsChannels};

use super::super::helpers::{neon_button, resource_counter_row, send};

pub(in crate::ui) fn discard_resource_panel(
    ui: &mut Ui,
    state: &mut AppState,
    channels: Option<&WsChannels>,
    gs: &GameStateView,
    my_id: PlayerId,
) {
    let Phase::DiscardResource {
        player, drop_total, ..
    } = &gs.phase
    else {
        return;
    };

    let room_owned = state.current_room.clone();
    let room = room_owned.as_deref();

    if *player == my_id {
        let drop_total = *drop_total;
        let (gas_held, oil_held) = gs
            .player(my_id)
            .map(|p| (p.resources.gas, p.resources.oil))
            .unwrap_or((0, 0));

        ui.label(
            RichText::new(format!(
                "Hybrid plants can't hold all your fuel — discard {} total:",
                drop_total
            ))
            .color(theme::NEON_AMBER)
            .monospace(),
        );
        ui.add_space(4.0);

        let gas_max = gas_held.min(drop_total - state.discard_oil);
        match resource_counter_row(
            ui,
            "     GAS",
            state.discard_gas,
            0,
            gas_max,
            &format!("held: {}", gas_held),
        ) {
            d if d < 0 => state.discard_gas -= 1,
            d if d > 0 => state.discard_gas += 1,
            _ => {}
        }

        let oil_max = oil_held.min(drop_total - state.discard_gas);
        match resource_counter_row(
            ui,
            "     OIL",
            state.discard_oil,
            0,
            oil_max,
            &format!("held: {}", oil_held),
        ) {
            d if d < 0 => state.discard_oil -= 1,
            d if d > 0 => state.discard_oil += 1,
            _ => {}
        }

        let selected = state.discard_gas + state.discard_oil;
        ui.add_space(4.0);
        ui.label(
            RichText::new(format!("{} / {} selected", selected, drop_total))
                .color(if selected == drop_total {
                    theme::NEON_GREEN
                } else {
                    theme::TEXT_DIM
                })
                .monospace(),
        );

        if ui
            .add_enabled(
                selected == drop_total,
                neon_button("[ CONFIRM ]", theme::NEON_CYAN),
            )
            .clicked()
        {
            send(
                Action::DiscardResource {
                    gas: state.discard_gas,
                    oil: state.discard_oil,
                },
                room,
                channels,
            );
        }
    } else {
        let name = gs.player(*player).map(|p| p.name.as_str()).unwrap_or("???");
        ui.label(
            RichText::new(format!("● Waiting for {} to discard fuel…", name))
                .color(theme::TEXT_DIM)
                .monospace(),
        );
    }
}
