use egui::{Align2, Color32, FontId, Rect, RichText, Sense, Stroke, StrokeKind, Ui};
use powergrid_core::{
    actions::{Action, HintPayload},
    price_table,
    types::{Phase, PlayerColor, PlayerId, Resource, ResourceMarket},
    GameStateView,
};
use std::collections::HashMap;

use crate::{
    card_painter,
    state::{player_color_to_egui, AppState, CitySnapshot},
    theme,
    ws::WsChannels,
};

use super::helpers::{dim_color, send, vertical_labeled_section};
use super::phase_tracker::phase_tracker;

pub(super) fn top_panel_contents(
    ui: &mut Ui,
    gs: GameStateView,
    state: &mut AppState,
    channels: Option<&WsChannels>,
    my_id: PlayerId,
) {
    let room = state.current_room.clone();
    let room = room.as_deref();
    let my_buy_turn = matches!(&gs.phase, Phase::BuyResources { remaining }
        if remaining.first() == Some(&my_id));
    ui.horizontal(|ui| {
        // Round / Step header
        ui.vertical(|ui| {
            theme::neon_frame_bright().show(ui, |ui| {
                ui.label(
                    RichText::new(format!("ROUND {}", gs.round))
                        .color(theme::NEON_CYAN)
                        .monospace(),
                );
                ui.label(RichText::new("STEP").color(theme::NEON_CYAN).monospace());
                step_replenish_columns(ui, gs.step, gs.players.len());
            });
        });

        ui.add_space(8.0);

        // Phase tracker
        phase_tracker(ui, &gs);

        ui.add_space(8.0);

        // Plant market
        vertical_labeled_section(ui, "PLANT MARKET", |ui| {
            ui.horizontal_top(|ui| {
                if gs.step >= 3 {
                    // Step 3: two columns of 3, all plants purchasable, no labels.
                    let mid = gs.market.actual.len().div_ceil(2);
                    let (left, right) = gs.market.actual.split_at(mid);
                    ui.vertical(|ui| {
                        plant_column(ui, left, channels, &gs.phase, my_id, &gs.player_order, room);
                    });
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        plant_column(
                            ui,
                            right,
                            channels,
                            &gs.phase,
                            my_id,
                            &gs.player_order,
                            room,
                        );
                    });
                } else {
                    // Steps 1 & 2: ACTUAL and FUTURE columns.
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("ACTUAL")
                                .color(theme::TEXT_DIM)
                                .small()
                                .monospace(),
                        );
                        plant_column(
                            ui,
                            &gs.market.actual,
                            channels,
                            &gs.phase,
                            my_id,
                            &gs.player_order,
                            room,
                        );
                    });
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("FUTURE")
                                .color(theme::TEXT_DIM)
                                .small()
                                .monospace(),
                        );
                        plant_column(
                            ui,
                            &gs.market.future,
                            channels,
                            &gs.phase,
                            my_id,
                            &gs.player_order,
                            room,
                        );
                    });
                }
            });
        });

        ui.add_space(8.0);

        // Resource market — clickable during the player's own BuyResources turn
        let cart_snapshot = state.resource_cart.clone();
        // Collect peer carts (BuyResources hints from other players)
        let peer_carts: Vec<(Color32, HashMap<Resource, u8>)> = state
            .peer_hints
            .hints
            .iter()
            .filter_map(|(pid, hint)| {
                if let HintPayload::Cart { items } = hint {
                    let color = gs
                        .player(*pid)
                        .map(|p| player_color_to_egui(p.color))
                        .unwrap_or(Color32::GRAY);
                    let cart: HashMap<Resource, u8> = items.iter().cloned().collect();
                    Some((color, cart))
                } else {
                    None
                }
            })
            .collect();
        let click = vertical_labeled_section(ui, "RESOURCE MARKET", |ui| {
            resource_market_grid(ui, &gs.resources, &cart_snapshot, &peer_carts, my_buy_turn)
        });
        if let Some((resource, amount)) = click {
            state.set_cart_amount(resource, amount);
        }
    });
}

fn plant_column(
    ui: &mut Ui,
    plants: &[powergrid_core::types::PowerPlant],
    channels: Option<&WsChannels>,
    phase: &Phase,
    my_id: PlayerId,
    player_order: &[PlayerId],
    room: Option<&str>,
) {
    let is_my_auction_turn = matches!(phase, Phase::Auction { current_bidder_idx, active_bid, .. }
        if active_bid.is_none() && player_order.get(*current_bidder_idx) == Some(&my_id));

    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        for plant in plants {
            let resp = card_painter::draw_plant_card(ui, plant);
            if is_my_auction_turn && resp.clicked() {
                send(
                    Action::SelectPlant {
                        plant_number: plant.number,
                    },
                    room,
                    channels,
                );
            }
            egui::Tooltip::for_enabled(&resp).show(|ui| {
                plant_tooltip(ui, plant);
            });
        }
    });
}

fn plant_tooltip(ui: &mut Ui, plant: &powergrid_core::types::PowerPlant) {
    ui.label(
        RichText::new(format!(
            "#{} {:?}\nCost: {}  Cities: {}",
            plant.number, plant.kind, plant.cost, plant.cities
        ))
        .monospace()
        .color(theme::TEXT_BRIGHT),
    );
}

fn replenish_rates(step: u8, n: usize) -> (u8, u8, u8, u8) {
    match step {
        1 => match n {
            2 => (3, 2, 1, 1),
            3 => (4, 2, 1, 1),
            4 => (5, 3, 2, 1),
            5 => (5, 4, 3, 2),
            _ => (7, 5, 3, 2),
        },
        2 => match n {
            2 => (4, 2, 1, 1),
            3 => (5, 3, 2, 1),
            4 => (6, 4, 3, 2),
            5 => (7, 5, 3, 3),
            _ => (9, 6, 5, 3),
        },
        _ => match n {
            2 => (3, 4, 3, 1),
            3 => (3, 4, 3, 1),
            4 => (4, 5, 4, 2),
            5 => (5, 6, 5, 3),
            _ => (7, 7, 6, 3),
        },
    }
}

fn step_replenish_columns(ui: &mut Ui, current_step: u8, n_players: usize) {
    let coal_color = theme::RES_COAL;
    let oil_color = theme::RES_OIL;
    let gas_color = theme::RES_GAS;
    let uran_color = theme::RES_URANIUM;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        for step in 1u8..=3 {
            let (coal, oil, gas, uran) = replenish_rates(step, n_players);
            let active = step == current_step;
            let hdr = if active {
                theme::NEON_CYAN
            } else {
                theme::TEXT_DIM
            };
            let c_col = if active {
                coal_color
            } else {
                dim_color(coal_color)
            };
            let o_col = if active {
                oil_color
            } else {
                dim_color(oil_color)
            };
            let g_col = if active {
                gas_color
            } else {
                dim_color(gas_color)
            };
            let u_col = if active {
                uran_color
            } else {
                dim_color(uran_color)
            };
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                ui.label(
                    RichText::new(format!("{step}"))
                        .color(hdr)
                        .monospace()
                        .small(),
                );
                ui.label(
                    RichText::new(format!("{coal}"))
                        .color(c_col)
                        .monospace()
                        .small(),
                );
                ui.label(
                    RichText::new(format!("{gas}"))
                        .color(g_col)
                        .monospace()
                        .small(),
                );
                ui.label(
                    RichText::new(format!("{oil}"))
                        .color(o_col)
                        .monospace()
                        .small(),
                );
                ui.label(
                    RichText::new(format!("{uran}"))
                        .color(u_col)
                        .monospace()
                        .small(),
                );
            });
        }
    });
}

/// Render the resource market grid.
///
/// When `clickable` is true (the player's BuyResources turn), clicking a square sets the
/// cart for that resource to "buy all from cheapest up to and including this square". Squares
/// that are currently in the cart are drawn as an outline instead of a filled box.
///
/// Returns `Some((resource, amount))` if the player clicked a square.
fn resource_market_grid(
    ui: &mut Ui,
    market: &ResourceMarket,
    cart: &HashMap<Resource, u8>,
    peer_carts: &[(Color32, HashMap<Resource, u8>)],
    clickable: bool,
) -> Option<(Resource, u8)> {
    const SQ: f32 = 14.0;
    const INNER_GAP: f32 = 2.0;
    const GROUP_GAP: f32 = 8.0;
    const LABEL_W: f32 = 36.0;
    const HEADER_H: f32 = 20.0;
    const ROW_H: f32 = SQ;
    const ROW_GAP: f32 = 4.0;

    let rows: &[(Resource, &str, Color32)] = &[
        (Resource::Coal, "COAL", theme::RES_COAL),
        (Resource::Gas, "GAS", theme::RES_GAS),
        (Resource::Oil, "OIL", theme::RES_OIL),
        (Resource::Uranium, "URAN", theme::RES_URANIUM),
    ];

    // For each resource, compute (price, group_size) pairs ordered cheapest → most expensive.
    let resource_groups: Vec<Vec<(u8, usize)>> = rows
        .iter()
        .map(|(r, _, _)| {
            let mut groups: Vec<(u8, usize)> = Vec::new();
            for &p in price_table(*r).iter().rev() {
                match groups.last_mut() {
                    Some(last) if last.0 == p => last.1 += 1,
                    _ => groups.push((p, 1)),
                }
            }
            groups
        })
        .collect();

    // All distinct prices across all resources, sorted ascending (cheapest first = leftmost).
    let mut all_prices: Vec<u8> = resource_groups
        .iter()
        .flat_map(|groups| groups.iter().map(|&(p, _)| p))
        .collect();
    all_prices.sort_unstable();
    all_prices.dedup();

    // Column width for price P = max group size at P across all resources.
    let col_widths: Vec<usize> = all_prices
        .iter()
        .map(|&p| {
            resource_groups
                .iter()
                .filter_map(|groups| groups.iter().find(|&&(gp, _)| gp == p).map(|&(_, gs)| gs))
                .max()
                .unwrap_or(0)
        })
        .collect();

    // X-offset (from ox + LABEL_W) of the first cell in each column.
    let mut col_x: Vec<f32> = Vec::with_capacity(all_prices.len());
    let mut x = 0.0f32;
    for (i, &w) in col_widths.iter().enumerate() {
        col_x.push(x);
        let col_w = w as f32 * (SQ + INNER_GAP) - INNER_GAP;
        x += col_w;
        if i + 1 < col_widths.len() {
            x += GROUP_GAP;
        }
    }
    let content_w = x;

    let total_w = LABEL_W + content_w;
    let n = rows.len() as f32;
    let total_h = HEADER_H + ROW_GAP + n * ROW_H + (n - 1.0) * ROW_GAP;

    let sense = if clickable {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(total_w, total_h), sense);

    if !ui.is_rect_visible(rect) {
        return None;
    }

    let painter = ui.painter();
    let ox = rect.min.x;
    let oy = rect.min.y;

    // ── Price header ──
    for (col_idx, (&price, &w)) in all_prices.iter().zip(col_widths.iter()).enumerate() {
        let gx = ox + LABEL_W + col_x[col_idx];
        let col_w = w as f32 * (SQ + INNER_GAP) - INNER_GAP;
        painter.text(
            egui::pos2(gx + col_w / 2.0, oy + HEADER_H / 2.0),
            Align2::CENTER_CENTER,
            format!("${price}"),
            FontId::monospace(10.0),
            theme::TEXT_DIM,
        );
    }

    let mut click_result: Option<(Resource, u8)> = None;
    let clicked_pos = if response.clicked() {
        response.interact_pointer_pos()
    } else {
        None
    };

    // ── Resource rows ──
    // Index 0 in price_table = scarcest (most expensive). `count` units occupy indices 0..count.
    // Display pos 0 = leftmost = cheapest; array_idx = total - 1 - display_pos.
    // Slot filled when array_idx < count.
    // Cart selects the cheapest `cart_amount` filled slots starting at display_pos = total - count.
    for (row_idx, ((resource, label, color), rgroups)) in
        rows.iter().zip(resource_groups.iter()).enumerate()
    {
        let row_y = oy + HEADER_H + ROW_GAP + row_idx as f32 * (ROW_H + ROW_GAP);
        let count = market.available(*resource) as usize;
        let total = price_table(*resource).len();
        let cart_amount = cart.get(resource).copied().unwrap_or(0) as usize;
        let cheapest_filled = total.saturating_sub(count);

        painter.text(
            egui::pos2(ox + LABEL_W - 2.0, row_y + ROW_H / 2.0),
            Align2::RIGHT_CENTER,
            *label,
            FontId::monospace(10.0),
            *color,
        );

        let mut display_pos = 0usize;
        for (col_idx, &price) in all_prices.iter().enumerate() {
            let group_size = rgroups
                .iter()
                .find(|&&(p, _)| p == price)
                .map_or(0, |&(_, gs)| gs);
            let gx = ox + LABEL_W + col_x[col_idx];

            for s in 0..group_size {
                let dp = display_pos + s;
                let array_idx = total - 1 - dp;
                let filled = array_idx < count;
                let in_cart = filled && dp >= cheapest_filled && dp < cheapest_filled + cart_amount;

                let sq_x = gx + s as f32 * (SQ + INNER_GAP);
                let sq_rect = Rect::from_min_size(egui::pos2(sq_x, row_y), egui::vec2(SQ, ROW_H));

                if in_cart {
                    painter.rect_stroke(sq_rect, 1.0, Stroke::new(1.5, *color), StrokeKind::Inside);
                } else if filled {
                    painter.rect_filled(sq_rect, 1.0, *color);
                } else {
                    painter.rect_filled(sq_rect, 1.0, dim_color(*color));
                }

                for (peer_color, peer_cart) in peer_carts {
                    let peer_amount = peer_cart.get(resource).copied().unwrap_or(0) as usize;
                    let peer_in_cart =
                        filled && dp >= cheapest_filled && dp < cheapest_filled + peer_amount;
                    if peer_in_cart {
                        painter.rect_stroke(
                            sq_rect.expand(1.5),
                            2.0,
                            Stroke::new(1.0, *peer_color),
                            StrokeKind::Outside,
                        );
                    }
                }

                if let Some(pos) = clicked_pos {
                    if pos.y >= row_y && pos.y < row_y + ROW_H && pos.x >= sq_x && pos.x < sq_x + SQ
                    {
                        let amount = if filled {
                            (dp.saturating_sub(cheapest_filled) + 1) as u8
                        } else {
                            0u8
                        };
                        click_result = Some((*resource, amount));
                    }
                }
            }
            display_pos += group_size;
        }
    }

    click_result
}

pub(super) fn city_history_graph(
    ui: &mut Ui,
    history: &[CitySnapshot],
    players_info: &[(PlayerId, PlayerColor)],
    end_game_cities: u8,
    gs: &GameStateView,
) {
    const PAD_L: f32 = 26.0;
    const PAD_B: f32 = 18.0;
    const H: f32 = 342.0;
    const DOT_R: f32 = 4.0;
    const STEP2_CITIES: usize = 7;

    let w = (ui.available_width() - PAD_L).max(100.0);
    let total_w = PAD_L + w;
    let total_h = PAD_B + H;

    let (rect, _) = ui.allocate_exact_size(egui::vec2(total_w, total_h), Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }

    let painter = ui.painter();
    let ox = rect.min.x + PAD_L;
    let oy = rect.min.y;

    // Determine y range — ensure indicator lines are always visible
    let max_cities = history
        .iter()
        .flat_map(|snap| snap.iter().map(|(_, c)| *c))
        .chain(gs.players.iter().map(|p| p.city_count()))
        .max()
        .unwrap_or(1)
        .max(end_game_cities as usize)
        .max(1);

    let rounds = history.len();
    // Include a projected point slot beyond the historical rounds
    let total_points = rounds + 1;
    let x_for = |idx: usize| -> f32 {
        if total_points <= 1 {
            ox
        } else {
            ox + (idx as f32 / (total_points - 1) as f32) * w
        }
    };

    // Draw axes
    painter.line_segment(
        [egui::pos2(ox, oy), egui::pos2(ox, oy + H)],
        Stroke::new(1.0, theme::TEXT_DIM),
    );
    painter.line_segment(
        [egui::pos2(ox, oy + H), egui::pos2(ox + w, oy + H)],
        Stroke::new(1.0, theme::TEXT_DIM),
    );

    // Y-axis label (max value)
    painter.text(
        egui::pos2(ox - 2.0, oy),
        Align2::RIGHT_TOP,
        format!("{max_cities}"),
        FontId::monospace(13.0),
        theme::TEXT_DIM,
    );
    painter.text(
        egui::pos2(ox - 2.0, oy + H),
        Align2::RIGHT_BOTTOM,
        "0",
        FontId::monospace(13.0),
        theme::TEXT_DIM,
    );

    // X-axis round labels (first and last)
    painter.text(
        egui::pos2(ox, oy + H + PAD_B),
        Align2::LEFT_BOTTOM,
        "1",
        FontId::monospace(13.0),
        theme::TEXT_DIM,
    );
    if rounds > 1 {
        painter.text(
            egui::pos2(ox + w, oy + H + PAD_B),
            Align2::RIGHT_BOTTOM,
            format!("{}", rounds + 1),
            FontId::monospace(13.0),
            theme::TEXT_DIM,
        );
    }

    // Draw Step 2 indicator line at 7 cities
    let step2_y = oy + H - (STEP2_CITIES as f32 / max_cities as f32) * H;
    let step2_color = theme::city_graph_step2();
    let dash_len = 4.0_f32;
    let gap_len = 3.0_f32;
    let mut x = ox;
    while x < ox + w {
        let x_end = (x + dash_len).min(ox + w);
        painter.line_segment(
            [egui::pos2(x, step2_y), egui::pos2(x_end, step2_y)],
            Stroke::new(1.0, step2_color),
        );
        x += dash_len + gap_len;
    }
    painter.text(
        egui::pos2(ox - 2.0, step2_y),
        Align2::RIGHT_CENTER,
        "S2",
        FontId::monospace(11.0),
        step2_color,
    );

    // Draw end game indicator line
    let end_y = oy + H - (end_game_cities as f32 / max_cities as f32) * H;
    let end_color = theme::city_graph_end();
    let mut x = ox;
    while x < ox + w {
        let x_end = (x + dash_len).min(ox + w);
        painter.line_segment(
            [egui::pos2(x, end_y), egui::pos2(x_end, end_y)],
            Stroke::new(1.0, end_color),
        );
        x += dash_len + gap_len;
    }
    painter.text(
        egui::pos2(ox - 2.0, end_y),
        Align2::RIGHT_CENTER,
        "E",
        FontId::monospace(11.0),
        end_color,
    );

    // Draw one line per player
    for (player_id, player_color) in players_info {
        let color = player_color_to_egui(*player_color);

        let points: Vec<egui::Pos2> = history
            .iter()
            .enumerate()
            .filter_map(|(round_idx, snap)| {
                snap.iter()
                    .find(|(id, _)| id == player_id)
                    .map(|(_, count)| {
                        let x = x_for(round_idx);
                        let y = oy + H - (*count as f32 / max_cities as f32) * H;
                        egui::pos2(x, y)
                    })
            })
            .collect();

        // Draw line segments
        for pair in points.windows(2) {
            painter.line_segment([pair[0], pair[1]], Stroke::new(2.5, color));
        }

        // Draw dots
        for pt in &points {
            painter.circle_filled(*pt, DOT_R, color);
        }

        // Draw projected next-round point (dimmer) using live city count
        if let Some(&last_pt) = points.last() {
            if let Some(player) = gs.players.iter().find(|p| p.id == *player_id) {
                let proj_count = player.city_count();
                let proj_x = x_for(rounds); // one slot beyond historical points
                let proj_y = oy + H - (proj_count as f32 / max_cities as f32) * H;
                let proj_pt = egui::pos2(proj_x, proj_y);
                let dim = dim_color(color);
                painter.line_segment([last_pt, proj_pt], Stroke::new(2.5, dim));
                painter.circle_filled(proj_pt, DOT_R, dim);
            }
        }
    }
}
