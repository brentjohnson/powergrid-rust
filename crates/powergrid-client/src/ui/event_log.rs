use egui::{text::LayoutJob, Color32, FontId, Ui};
use powergrid_core::GameStateView;

use crate::{state::player_color_to_egui, theme};

pub(super) fn event_log_contents(ui: &mut Ui, gs: &GameStateView) {
    let mut players: Vec<(&str, Color32)> = gs
        .players
        .iter()
        .map(|p| (p.name.as_str(), player_color_to_egui(p.color)))
        .collect();
    players.sort_by_key(|b| std::cmp::Reverse(b.0.len()));

    let font = FontId::monospace(egui::TextStyle::Small.resolve(ui.style()).size);

    theme::neon_frame().show(ui, |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for entry in gs.event_log.iter().rev().take(24) {
                    let mut job = LayoutJob::default();
                    let mut remaining = entry.as_str();

                    while !remaining.is_empty() {
                        let best = players
                            .iter()
                            .filter_map(|(name, color)| {
                                remaining.find(name).map(|pos| (pos, *name, *color))
                            })
                            .min_by_key(|(pos, _, _)| *pos);

                        if let Some((pos, name, color)) = best {
                            if pos > 0 {
                                job.append(
                                    &remaining[..pos],
                                    0.0,
                                    egui::text::TextFormat {
                                        font_id: font.clone(),
                                        color: theme::TEXT_DIM,
                                        ..Default::default()
                                    },
                                );
                            }
                            job.append(
                                name,
                                0.0,
                                egui::text::TextFormat {
                                    font_id: font.clone(),
                                    color,
                                    ..Default::default()
                                },
                            );
                            remaining = &remaining[pos + name.len()..];
                        } else {
                            job.append(
                                remaining,
                                0.0,
                                egui::text::TextFormat {
                                    font_id: font.clone(),
                                    color: theme::TEXT_DIM,
                                    ..Default::default()
                                },
                            );
                            break;
                        }
                    }

                    ui.label(job);
                }
            });
    });
}
