use egui::{RichText, Ui};
use powergrid_core::{actions::Action, types::Phase, types::PlayerId, GameStateView};

use crate::{card_painter, state::AppState, theme, ws::WsChannels};

use super::super::helpers::send;

pub(in crate::ui) fn discard_plant_panel(
    ui: &mut Ui,
    state: &mut AppState,
    channels: Option<&WsChannels>,
    gs: &GameStateView,
    my_id: PlayerId,
) {
    let Phase::DiscardPlant {
        player, new_plant, ..
    } = &gs.phase
    else {
        return;
    };

    let room_owned = state.current_room.clone();
    let room = room_owned.as_deref();

    if *player == my_id {
        ui.label(
            RichText::new("You won a 4th plant! Choose one of your existing plants to discard:")
                .color(theme::NEON_AMBER)
                .monospace(),
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new(format!("Incoming: plant #{}", new_plant.number))
                .color(theme::NEON_GREEN)
                .monospace(),
        );
        card_painter::draw_plant_card(ui, new_plant);
        ui.add_space(4.0);
        ui.label(
            RichText::new("Click a plant to discard it:")
                .color(theme::TEXT_BRIGHT)
                .monospace(),
        );
        if let Some(player_data) = gs.player(my_id) {
            for plant in &player_data.plants {
                let resp = card_painter::draw_plant_card(ui, plant);
                if resp.clicked() {
                    send(
                        Action::DiscardPlant {
                            plant_number: plant.number,
                        },
                        room,
                        channels,
                    );
                }
            }
        }
    } else {
        let name = gs.player(*player).map(|p| p.name.as_str()).unwrap_or("???");
        ui.label(
            RichText::new(format!("● Waiting for {} to discard a plant…", name))
                .color(theme::TEXT_DIM)
                .monospace(),
        );
    }
}
