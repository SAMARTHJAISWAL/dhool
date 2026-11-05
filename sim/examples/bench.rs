//! Rough throughput benchmark: `cargo run --release --example bench`.
//!
//! Builds a busy 320x200 scene (sand, water, oil, lava, burning wood) and
//! reports ticks per second, render passes per second, and the cost of a
//! save. It is a timing note, not a statistically careful benchmark.

use std::time::Instant;

use dhool_sim::{Material, World};

fn scene() -> World {
    let mut w = World::new(320, 200, 2026);
    w.fill_rect(0, 196, 320, 4, Material::Wall);
    w.fill_rect(0, 0, 4, 200, Material::Wall);
    w.fill_rect(316, 0, 4, 200, Material::Wall);
    w.fill_rect(10, 20, 90, 60, Material::Sand);
    w.fill_rect(110, 10, 90, 80, Material::Water);
    w.fill_rect(210, 30, 90, 30, Material::Oil);
    w.fill_rect(40, 150, 80, 30, Material::Wood);
    w.fill_rect(200, 120, 60, 20, Material::Lava);
    w.stamp(80, 148, 4, Material::Fire);
    w
}

fn main() {
    let ticks = 2_000;
    let mut w = scene();
    // Warm up so the first falls are not over-represented.
    for _ in 0..60 {
        w.tick();
    }
    let t = Instant::now();
    for _ in 0..ticks {
        w.tick();
    }
    let secs = t.elapsed().as_secs_f64();
    println!(
        "tick:   {:>8.0} ticks/s  ({:.3} ms/tick, {} x {})",
        f64::from(ticks) / secs,
        secs * 1000.0 / f64::from(ticks),
        w.width(),
        w.height()
    );

    let t = Instant::now();
    for _ in 0..ticks {
        w.render();
    }
    let secs = t.elapsed().as_secs_f64();
    println!(
        "render: {:>8.0} frames/s ({:.3} ms/frame)",
        f64::from(ticks) / secs,
        secs * 1000.0 / f64::from(ticks)
    );

    let t = Instant::now();
    let bytes = w.save();
    println!(
        "save:   {} bytes in {:.3} ms",
        bytes.len(),
        t.elapsed().as_secs_f64() * 1000.0
    );
}
