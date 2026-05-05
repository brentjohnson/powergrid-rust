use egui::{Color32, Painter, Pos2, Stroke, Vec2};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;

/// Stable per-city hash for animation phase variety.
pub fn city_seed(city_id: &str) -> u64 {
    let mut h = DefaultHasher::new();
    city_id.hash(&mut h);
    h.finish()
}

/// Animated selection indicator for a city cluster during BuildCities.
/// Draws pulsing rings + orbiting sparks in the same electric-cyan palette as lightning edges.
/// `city_r` is the per-slot house radius used in map_panel.
pub fn draw_selected_city_glow(painter: &Painter, center: Pos2, city_r: f32, t: f64, seed: u64) {
    let seed_phase = (seed % 1000) as f64 * (std::f64::consts::TAU / 1000.0);
    let pulse = ((t * 3.0 + seed_phase).sin() * 0.5 + 0.5) as f32;
    let pulse2 = 1.0 - pulse;

    // Soft area glow behind the whole cluster
    let glow_a = (pulse * 50.0 + 15.0) as u8;
    painter.circle_filled(
        center,
        city_r * 4.2,
        Color32::from_rgba_unmultiplied(0, 200, 255, glow_a),
    );

    // Primary ring — pulsing size and alpha, matching lightning halo colour
    let ring_a = (pulse * 180.0 + 70.0) as u8;
    painter.circle_stroke(
        center,
        city_r * 3.4 + pulse * city_r * 0.35,
        Stroke::new(
            (city_r * 0.25).max(1.5),
            Color32::from_rgba_unmultiplied(0, 210, 255, ring_a),
        ),
    );

    // Secondary ring — counter-phase so one is always visible
    let ring2_a = (pulse2 * 90.0 + 20.0) as u8;
    painter.circle_stroke(
        center,
        city_r * 4.1 + pulse2 * city_r * 0.3,
        Stroke::new(
            (city_r * 0.12).max(0.8),
            Color32::from_rgba_unmultiplied(0, 220, 255, ring2_a),
        ),
    );

    // Orbiting spark dots — same style as lightning edge sparks
    const NUM_SPARKS: usize = 5;
    for k in 0..NUM_SPARKS {
        let angle = t * 1.8 + k as f64 * std::f64::consts::TAU / NUM_SPARKS as f64 + seed_phase;
        if (t * 7.0 + k as f64 * 2.1 + seed_phase).sin() > 0.1 {
            let orbit_r = city_r * 3.7;
            let sp = Pos2::new(
                center.x + orbit_r * angle.cos() as f32,
                center.y + orbit_r * angle.sin() as f32,
            );
            let sr = (city_r * 0.28).max(1.5);
            painter.circle_filled(sp, sr, Color32::from_rgba_unmultiplied(180, 240, 255, 200));
            painter.circle_filled(sp, sr * 0.4, Color32::WHITE);
        }
    }
}

pub fn elapsed_seconds(ctx: &egui::Context) -> f64 {
    ctx.input(|i| i.time)
}

/// Call while an animation is active to keep the egui pass rerunning at ~30 fps.
/// Without this, reactive winit won't redraw until the next input event.
pub fn keep_animating(ctx: &egui::Context) {
    ctx.request_repaint_after(Duration::from_millis(33));
}

/// Stable per-edge hash. Sorted so direction doesn't matter.
pub fn edge_seed(a: &str, b: &str) -> u64 {
    let (first, second) = if a <= b { (a, b) } else { (b, a) };
    let mut h = DefaultHasher::new();
    first.hash(&mut h);
    second.hash(&mut h);
    h.finish()
}

/// Animated lightning-bolt stroke between two screen points.
/// Subdivides into ~14 segments with perpendicular jitter driven by `t` (seconds)
/// and a stable per-edge `seed`, so each edge wiggles with its own character.
pub fn draw_lightning_edge(
    painter: &Painter,
    from: Pos2,
    to: Pos2,
    t: f64,
    base_width: f32,
    seed: u64,
) {
    const SEGMENTS: usize = 14;

    let diff = to - from;
    let len = diff.length();
    if len < 0.001 {
        return;
    }

    let along = diff / len;
    let perp = Vec2::new(-along.y, along.x);

    // Jitter amplitude scales with line length, capped so short edges still look alive
    let amp = (len as f64 * 0.04).clamp(2.0, 14.0);

    // Two seed-derived phase offsets so different edges have distinct waveforms
    let seed_a = (seed % 1000) as f64 * (std::f64::consts::TAU / 1000.0);
    let seed_b = ((seed >> 16) % 1000) as f64 * (std::f64::consts::TAU / 1000.0);

    // Build jagged polyline — endpoints pinned, interior vertices jittered
    let mut pts: Vec<Pos2> = Vec::with_capacity(SEGMENTS + 1);
    for i in 0..=SEGMENTS {
        let frac = i as f32 / SEGMENTS as f32;
        let base = from + diff * frac;
        if i == 0 || i == SEGMENTS {
            pts.push(base);
        } else {
            let fi = i as f64;
            let jitter = amp
                * ((t * 3.7 + fi * 0.8 + seed_a).sin() + 0.5 * (t * 7.3 + fi * 1.3 + seed_b).sin());
            pts.push(base + perp * jitter as f32);
        }
    }

    // Layer 1: straight halo, alpha-pulsed
    let halo_alpha = ((t * 4.0).sin() * 0.3 + 0.7) as f32; // 0.4 .. 1.0
    let halo_a = (halo_alpha * 70.0) as u8; // 28 .. 70
    painter.line_segment(
        [from, to],
        Stroke::new(
            base_width * 4.0,
            Color32::from_rgba_unmultiplied(0, 200, 255, halo_a),
        ),
    );

    // Layer 2 + 3: jagged mid stroke + bright core per segment
    for w in pts.windows(2) {
        let (p0, p1) = (w[0], w[1]);
        painter.line_segment(
            [p0, p1],
            Stroke::new(
                base_width,
                Color32::from_rgba_unmultiplied(0, 220, 255, 220),
            ),
        );
        painter.line_segment(
            [p0, p1],
            Stroke::new(
                (base_width * 0.35).max(0.5),
                Color32::from_rgba_unmultiplied(200, 245, 255, 240),
            ),
        );
    }

    // Layer 4: spark dots — two interior vertices chosen by a cycling index
    let n = pts.len(); // SEGMENTS + 1 = 15; interior = indices 1..=13
    let tick = (t * 8.0) as u64;
    for k in 0..2u64 {
        let idx = ((seed.wrapping_add(tick).wrapping_add(k.wrapping_mul(7))) % (n as u64 - 2) + 1)
            as usize;
        let r = base_width * 0.9;
        painter.circle_filled(
            pts[idx],
            r,
            Color32::from_rgba_unmultiplied(180, 240, 255, 200),
        );
        painter.circle_filled(pts[idx], r * 0.4, Color32::WHITE);
    }
}
