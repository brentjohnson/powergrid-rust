use egui::{RichText, Ui};
use powergrid_core::{
    types::{Phase, PlayerId},
    GameStateView,
};

use crate::{state::player_color_to_egui, theme};

use super::helpers::dim_color;

pub(super) fn phase_tracker(ui: &mut Ui, gs: &GameStateView) {
    #[derive(Clone, Copy, PartialEq)]
    enum Dp {
        Auction,
        Resource,
        Build,
        Bureaucracy,
    }

    let current = match &gs.phase {
        Phase::Auction { .. } | Phase::DiscardPlant { .. } => Some(Dp::Auction),
        Phase::BuyResources { .. } => Some(Dp::Resource),
        Phase::BuildCities { .. } => Some(Dp::Build),
        Phase::Bureaucracy { .. } => Some(Dp::Bureaucracy),
        _ => None,
    };

    let phases = [
        (Dp::Auction, "AUCTION"),
        (Dp::Resource, "RESOURCES"),
        (Dp::Build, "BUILD"),
        (Dp::Bureaucracy, "BUREAUCRACY"),
    ];

    let phase_idx = |dp: Dp| match dp {
        Dp::Auction => 0u8,
        Dp::Resource => 1,
        Dp::Build => 2,
        Dp::Bureaucracy => 3,
    };
    let current_idx = current.map(phase_idx);

    super::helpers::vertical_labeled_section(ui, "PHASE TRACKING", |ui| {
        egui::Grid::new("phase_tracker_grid")
            .num_columns(2)
            .spacing([8.0, 2.0])
            .show(ui, |ui| {
                for (dp, label) in &phases {
                    let is_current = current == Some(*dp);

                    let label_color = if is_current {
                        theme::NEON_AMBER
                    } else {
                        theme::TEXT_DIM
                    };
                    ui.label(RichText::new(*label).color(label_color).small().monospace());

                    let player_ids: Vec<PlayerId> = if *dp == Dp::Auction {
                        gs.player_order.clone()
                    } else {
                        gs.player_order.iter().rev().cloned().collect()
                    };

                    let phase_active: Option<PlayerId> = if !is_current {
                        None
                    } else {
                        match &gs.phase {
                            Phase::Auction {
                                current_bidder_idx, ..
                            } => gs.player_order.get(*current_bidder_idx).copied(),
                            Phase::BuyResources { remaining }
                            | Phase::BuildCities { remaining } => remaining.first().copied(),
                            Phase::Bureaucracy { .. } => None,
                            _ => None,
                        }
                    };

                    ui.horizontal(|ui| {
                        for pid in &player_ids {
                            let is_active = phase_active == Some(*pid);
                            let is_completed = if !is_current {
                                false
                            } else {
                                match &gs.phase {
                                    Phase::Auction { bought, passed, .. } => {
                                        bought.contains(pid) || passed.contains(pid)
                                    }
                                    Phase::BuyResources { remaining }
                                    | Phase::BuildCities { remaining }
                                    | Phase::Bureaucracy { remaining } => !remaining.contains(pid),
                                    _ => false,
                                }
                            };

                            if let Some(p) = gs.player(*pid) {
                                let base = player_color_to_egui(p.color);
                                let is_past = current_idx.is_some_and(|ci| phase_idx(*dp) < ci);
                                let dimmed = is_past || is_completed;

                                let size = egui::Vec2::splat(16.0);
                                let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                                if ui.is_rect_visible(rect) {
                                    let painter = ui.painter();
                                    if is_active {
                                        painter.rect_filled(rect, 2.0, dim_color(base));
                                        painter.rect_stroke(
                                            rect,
                                            2.0,
                                            egui::Stroke::new(2.0, base),
                                            egui::StrokeKind::Outside,
                                        );
                                    } else {
                                        let fill = if dimmed { dim_color(base) } else { base };
                                        painter.rect_filled(rect, 2.0, fill);
                                    }
                                }
                            }
                        }
                    });

                    ui.end_row();
                }
            });
    });
}
