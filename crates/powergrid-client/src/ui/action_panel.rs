use egui::{RichText, Ui};
use powergrid_core::{
    actions::Action,
    check_plant_feasibility, income_for,
    types::{Phase, PlantKind, PlayerId, Resource},
    GameStateView,
};

use crate::{card_painter, state::player_color_to_egui, state::AppState, theme, ws::WsChannels};

use super::helpers::{
    dim_color, is_active_player, neon_button, resource_counter_row, resource_name, section_header,
    send,
};

pub(super) fn action_panel(
    ui: &mut Ui,
    state: &mut AppState,
    channels: Option<&WsChannels>,
    gs: &GameStateView,
    my_id: PlayerId,
) {
    // Clone the room name so it can be used inside closures (avoids holding a borrow on state).
    let room_owned = state.current_room.clone();
    let room = room_owned.as_deref();
    match &gs.phase {
        Phase::Auction {
            current_bidder_idx,
            active_bid,
            bought,
            passed,
        } => {
            let my_nominate_turn = gs.player_order.get(*current_bidder_idx) == Some(&my_id);

            // Section header
            if let Some(bid) = active_bid {
                section_header(ui, &format!("─ AUCTION: Plant #{} ─", bid.plant_number));
            } else {
                section_header(ui, "─ AUCTION ─");
            }

            // Plant card (only when bidding is active)
            if let Some(bid) = active_bid {
                let target_plant = gs
                    .market
                    .actual
                    .iter()
                    .chain(gs.market.future.iter())
                    .find(|p| p.number == bid.plant_number);
                if let Some(plant) = target_plant {
                    card_painter::draw_plant_card(ui, plant);
                    ui.add_space(4.0);
                }
            }

            // Per-player status column in turn order
            for pid in &gs.player_order {
                if let Some(p) = gs.player(*pid) {
                    let is_me = p.id == my_id;
                    let active = is_active_player(gs, p.id);
                    let player_color = player_color_to_egui(p.color);
                    let swatch_color = if active {
                        player_color
                    } else {
                        dim_color(player_color)
                    };
                    let name_color = if active {
                        player_color
                    } else {
                        dim_color(player_color)
                    };

                    let (status_text, status_color) = if bought.contains(&p.id) {
                        ("PURCHASED".to_string(), theme::NEON_GREEN)
                    } else if passed.contains(&p.id) {
                        ("PASSED".to_string(), theme::TEXT_DIM)
                    } else if let Some(bid) = active_bid {
                        let last_bid = state.auction_last_bids.get(&p.id).copied();
                        if bid.highest_bidder == p.id {
                            (format!("BID ${}  ◀ leading", bid.amount), theme::NEON_AMBER)
                        } else if bid.remaining_bidders.first() == Some(&p.id) {
                            match last_bid {
                                Some(a) => (format!("▶ to bid  ${a}"), theme::NEON_CYAN),
                                None => ("▶ to bid".to_string(), theme::NEON_CYAN),
                            }
                        } else if bid.remaining_bidders.contains(&p.id) {
                            match last_bid {
                                Some(a) => (format!("in  ${a}"), theme::TEXT_MID),
                                None => ("in".to_string(), theme::TEXT_MID),
                            }
                        } else {
                            ("passed bid".to_string(), theme::TEXT_DIM)
                        }
                    } else if gs.player_order.get(*current_bidder_idx) == Some(&p.id) {
                        ("▶ to nominate".to_string(), theme::NEON_CYAN)
                    } else {
                        ("—".to_string(), theme::TEXT_DIM)
                    };

                    ui.horizontal(|ui| {
                        ui.label(RichText::new("■").color(swatch_color).monospace());
                        ui.label(RichText::new(&p.name).color(name_color).monospace());
                        if is_me {
                            ui.label(RichText::new("(you)").color(theme::TEXT_DIM).small());
                        }
                        ui.label(RichText::new(status_text).color(status_color).monospace());
                    });
                }
            }

            ui.add_space(4.0);

            // Bid / nominate controls
            if let Some(bid) = active_bid {
                let is_my_bid_turn = bid.remaining_bidders.first() == Some(&my_id);
                if is_my_bid_turn {
                    let my_money = gs.player(my_id).map(|p| p.money).unwrap_or(0);
                    let min_bid = bid.amount + 1;
                    let max_bid = my_money;

                    if state.bid_plant_number != Some(bid.plant_number) {
                        state.bid_plant_number = Some(bid.plant_number);
                        state.bid_amount = min_bid;
                    }
                    if state.bid_amount < min_bid {
                        state.bid_amount = min_bid;
                    }
                    if state.bid_amount > max_bid {
                        state.bid_amount = max_bid;
                    }

                    ui.label(
                        RichText::new("Your turn to bid:")
                            .color(theme::TEXT_BRIGHT)
                            .monospace(),
                    );
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                state.bid_amount > min_bid,
                                neon_button("[ - ]", theme::NEON_AMBER),
                            )
                            .clicked()
                        {
                            state.bid_amount -= 1;
                        }
                        ui.label(
                            RichText::new(format!("${}", state.bid_amount))
                                .color(theme::TEXT_BRIGHT)
                                .monospace(),
                        );
                        if ui
                            .add_enabled(
                                state.bid_amount < max_bid,
                                neon_button("[ + ]", theme::NEON_AMBER),
                            )
                            .clicked()
                        {
                            state.bid_amount += 1;
                        }
                        let can_bid = min_bid <= max_bid;
                        if ui
                            .add_enabled(can_bid, neon_button("[ BID ]", theme::NEON_CYAN))
                            .clicked()
                        {
                            send(
                                Action::PlaceBid {
                                    amount: state.bid_amount,
                                },
                                room,
                                channels,
                            );
                            state.bid_amount = 0;
                        }
                        if ui.add(neon_button("[ PASS ]", theme::NEON_AMBER)).clicked() {
                            send(Action::PassAuction, room, channels);
                        }
                    });
                }
            } else if my_nominate_turn {
                ui.label(
                    RichText::new("Your turn — select a plant from the market, or pass.")
                        .color(theme::TEXT_BRIGHT)
                        .monospace(),
                );
                if ui.add(neon_button("[ PASS ]", theme::NEON_AMBER)).clicked() {
                    send(Action::PassAuction, room, channels);
                }
            }
        }

        Phase::DiscardPlant {
            player, new_plant, ..
        } => {
            if *player == my_id {
                ui.label(
                    RichText::new(
                        "You won a 4th plant! Choose one of your existing plants to discard:",
                    )
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

        Phase::DiscardResource {
            player, drop_total, ..
        } => {
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

                // Cap each counter at min(held, drop_total - other_selection) so
                // gas+oil cannot exceed the required total.
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

        Phase::BuyResources { remaining } => {
            if remaining.first() == Some(&my_id) {
                let my_money = gs.player(my_id).map(|p| p.money).unwrap_or(0);
                let player = gs.player(my_id);

                for resource in [
                    Resource::Coal,
                    Resource::Oil,
                    Resource::Gas,
                    Resource::Uranium,
                ] {
                    let count = state.resource_cart.get(&resource).copied().unwrap_or(0);

                    let (owned, cap_lo, cap_hi) = player
                        .map(|p| {
                            let has_hybrid =
                                p.plants.iter().any(|pl| pl.kind == PlantKind::GasOrOil);
                            match resource {
                                Resource::Gas | Resource::Oil if has_hybrid => {
                                    let gas_only: u8 = p
                                        .plants
                                        .iter()
                                        .filter(|pl| pl.kind == PlantKind::Gas)
                                        .map(|pl| pl.cost * 2)
                                        .sum();
                                    let oil_only: u8 = p
                                        .plants
                                        .iter()
                                        .filter(|pl| pl.kind == PlantKind::Oil)
                                        .map(|pl| pl.cost * 2)
                                        .sum();
                                    let hybrid: u8 = p
                                        .plants
                                        .iter()
                                        .filter(|pl| pl.kind == PlantKind::GasOrOil)
                                        .map(|pl| pl.cost * 2)
                                        .sum();
                                    let (dedicated, owned) = if resource == Resource::Gas {
                                        (gas_only, p.resources.gas)
                                    } else {
                                        (oil_only, p.resources.oil)
                                    };
                                    (owned, dedicated, dedicated + hybrid)
                                }
                                _ => {
                                    let cap = p.resource_capacity(resource);
                                    (p.resources.get(resource), cap, cap)
                                }
                            }
                        })
                        .unwrap_or((0, 0, 0));

                    let cap_str = if cap_lo == cap_hi {
                        format!("{owned}/{cap_hi}")
                    } else {
                        format!("{owned}/{cap_lo}-{cap_hi}")
                    };

                    match resource_counter_row(
                        ui,
                        &format!("{:>8}", resource_name(resource)),
                        count,
                        0,
                        u8::MAX,
                        &cap_str,
                    ) {
                        d if d < 0 => state.remove_from_cart(resource),
                        d if d > 0 => state.add_to_cart(resource),
                        _ => {}
                    }
                }

                if let Some(cost) = state.resource_cart_cost {
                    let cost_color = if cost > my_money {
                        theme::NEON_RED
                    } else {
                        theme::NEON_GREEN
                    };
                    ui.label(
                        RichText::new(format!("TOTAL: ${cost}  BALANCE: ${my_money}"))
                            .color(cost_color)
                            .monospace(),
                    );
                }

                let has_fuel_plants =
                    player.is_some_and(|p| p.plants.iter().any(|pl| pl.kind.needs_resources()));
                if has_fuel_plants {
                    ui.horizontal(|ui| {
                        if ui.add(neon_button("[ 1 SET ]", theme::NEON_CYAN)).clicked() {
                            state.fill_cart_for_sets(1);
                        }
                        if ui
                            .add(neon_button("[ 2 SETS ]", theme::NEON_CYAN))
                            .clicked()
                        {
                            state.fill_cart_for_sets(2);
                        }
                    });
                }

                let unaffordable = state.resource_cart_cost.is_some_and(|c| c > my_money);
                ui.horizontal(|ui| {
                    if ui
                        .add(neon_button("[ CLEAR ]", theme::NEON_AMBER))
                        .clicked()
                    {
                        state.clear_cart();
                    }
                    if ui
                        .add_enabled(
                            !unaffordable,
                            neon_button("[ DONE BUYING ]", theme::NEON_CYAN),
                        )
                        .clicked()
                    {
                        let purchases = state.cart_purchases();
                        if purchases.is_empty() {
                            send(Action::DoneBuying, room, channels);
                        } else {
                            send(Action::BuyResourceBatch { purchases }, room, channels);
                        }
                    }
                });
            } else {
                ui.label(
                    RichText::new("● Waiting for other operators to buy…")
                        .color(theme::TEXT_DIM)
                        .monospace(),
                );
            }
        }

        Phase::BuildCities { remaining } => {
            if remaining.first() == Some(&my_id) {
                let my_money = gs.player(my_id).map(|p| p.money).unwrap_or(0);
                ui.label(
                    RichText::new("Click cities on map to select build targets.")
                        .color(theme::TEXT_BRIGHT)
                        .monospace(),
                );

                if !state.selected_build_cities.is_empty() {
                    let bp = &state.build_preview;
                    let cost_color = if bp.total_cost > my_money {
                        theme::NEON_RED
                    } else {
                        theme::NEON_GREEN
                    };
                    ui.label(
                        RichText::new(format!(
                            "Selected: {}  Route: ${}  Slots: ${}  Total: ${}",
                            state.selected_build_cities.len(),
                            bp.total_route_cost,
                            bp.total_slot_cost,
                            bp.total_cost,
                        ))
                        .color(cost_color)
                        .monospace(),
                    );
                }

                ui.horizontal(|ui| {
                    if ui
                        .add(neon_button("[ CLEAR ]", theme::NEON_AMBER))
                        .clicked()
                    {
                        state.clear_build_selection();
                    }
                    if ui
                        .add(neon_button("[ DONE BUILDING ]", theme::NEON_CYAN))
                        .clicked()
                    {
                        if state.selected_build_cities.is_empty() {
                            send(Action::DoneBuilding, room, channels);
                        } else {
                            let city_ids = state.build_preview.ordered.clone();
                            send(Action::BuildCities { city_ids }, room, channels);
                        }
                    }
                });
            } else {
                ui.label(
                    RichText::new("● Waiting for other operators to build…")
                        .color(theme::TEXT_DIM)
                        .monospace(),
                );
            }
        }

        Phase::Bureaucracy { remaining } => {
            if remaining.contains(&my_id) {
                if let Some(player) = gs.player(my_id) {
                    // Initialise selection to the optimal subset on the first frame in phase.
                    if !state.power_selected_initialised {
                        let (default_selection, _, _) = player.optimal_firing_subset();
                        state.power_selected_plants = default_selection.into_iter().collect();
                        state.power_selected_initialised = true;
                    }

                    ui.label(
                        RichText::new("Select plants to fire:")
                            .color(theme::TEXT_BRIGHT)
                            .monospace(),
                    );
                    ui.add_space(4.0);

                    for plant in &player.plants {
                        let kind_str = match plant.kind {
                            PlantKind::Coal => "coal",
                            PlantKind::Oil => "oil",
                            PlantKind::GasOrOil => "gas/oil",
                            PlantKind::Gas => "gas",
                            PlantKind::Uranium => "uranium",
                            PlantKind::Wind => "wind",
                        };
                        let city_word = if plant.cities == 1 { "city" } else { "cities" };
                        let label = format!(
                            "Plant {:>2}: {} {}  → {} {}",
                            plant.number, plant.cost, kind_str, plant.cities, city_word
                        );
                        let mut checked = state.power_selected_plants.contains(&plant.number);
                        if ui
                            .checkbox(
                                &mut checked,
                                RichText::new(label).monospace().color(theme::TEXT_BRIGHT),
                            )
                            .changed()
                        {
                            if checked {
                                state.power_selected_plants.insert(plant.number);
                            } else {
                                state.power_selected_plants.remove(&plant.number);
                            }
                        }
                    }

                    ui.add_space(4.0);

                    // Live preview: compute feasibility of current selection.
                    let selected: Vec<_> = player
                        .plants
                        .iter()
                        .filter(|p| state.power_selected_plants.contains(&p.number))
                        .collect();
                    let cities_owned = player.city_count() as u8;
                    let feasibility = check_plant_feasibility(&selected, &player.resources);

                    match feasibility {
                        Some((powered, remaining_res)) => {
                            let capped = powered.min(cities_owned);
                            let income = income_for(capped);
                            ui.label(
                                RichText::new(format!(
                                    "Powering {}/{} cities → {} elektro",
                                    capped, cities_owned, income
                                ))
                                .monospace()
                                .color(theme::NEON_GREEN),
                            );
                            let spent_coal = player.resources.coal - remaining_res.coal;
                            let spent_oil = player.resources.oil - remaining_res.oil;
                            let spent_gas = player.resources.gas - remaining_res.gas;
                            let spent_uranium = player.resources.uranium - remaining_res.uranium;
                            let mut parts: Vec<String> = Vec::new();
                            if spent_coal > 0 {
                                parts.push(format!("{} coal", spent_coal));
                            }
                            if spent_oil > 0 {
                                parts.push(format!("{} oil", spent_oil));
                            }
                            if spent_gas > 0 {
                                parts.push(format!("{} gas", spent_gas));
                            }
                            if spent_uranium > 0 {
                                parts.push(format!("{} uranium", spent_uranium));
                            }
                            let spend_str = if parts.is_empty() {
                                "Spends: nothing".to_string()
                            } else {
                                format!("Spends: {}", parts.join(", "))
                            };
                            ui.label(RichText::new(spend_str).monospace().color(theme::TEXT_DIM));
                            ui.add_space(4.0);
                            if ui
                                .add(neon_button("[ POWER CITIES ]", theme::NEON_GREEN))
                                .clicked()
                            {
                                let plant_numbers: Vec<u8> =
                                    state.power_selected_plants.iter().copied().collect();
                                send(Action::PowerCities { plant_numbers }, room, channels);
                            }
                        }
                        None => {
                            ui.label(
                                RichText::new("⚠ Selection is infeasible — not enough resources")
                                    .monospace()
                                    .color(theme::NEON_AMBER),
                            );
                            ui.add_space(4.0);
                            ui.add_enabled(
                                false,
                                neon_button("[ POWER CITIES ]", theme::NEON_GREEN),
                            );
                        }
                    }
                }
            } else {
                ui.label(
                    RichText::new("● Waiting for other operators…")
                        .color(theme::TEXT_DIM)
                        .monospace(),
                );
            }
        }

        Phase::PowerCitiesFuel {
            player,
            hybrid_cost,
            plant_numbers,
            ..
        } => {
            if *player == my_id {
                let hybrid_cost = *hybrid_cost;
                let (gas_held, oil_held) = gs
                    .player(my_id)
                    .map(|p| (p.resources.gas, p.resources.oil))
                    .unwrap_or((0, 0));

                // Compute how much pure-fuel plants in the chosen subset need.
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

                // Clamp gas selection to valid range.
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

        Phase::GameOver { winner } => {
            let name = gs
                .player(*winner)
                .map(|p| p.name.as_str())
                .unwrap_or("UNKNOWN");
            ui.label(
                RichText::new(format!("GRID CONTROLLED BY: {name}"))
                    .size(theme::HEADING_M)
                    .color(theme::NEON_GREEN)
                    .monospace(),
            );
        }

        _ => {}
    }
}
