//! Behavioural tests: each one sets up a small scene, runs it, and checks
//! an outcome you could see on screen.

#![allow(clippy::cast_precision_loss)]

use Material::*;
use dhool_sim::{Material, World};

/// A `w` x `h` world with a one-cell wall border (a closed box).
fn boxed(w: usize, h: usize, seed: u32) -> World {
    let mut world = World::new(w, h, seed);
    let (wi, hi) = (w as i32, h as i32);
    world.fill_rect(0, 0, wi, 1, Wall);
    world.fill_rect(0, hi - 1, wi, 1, Wall);
    world.fill_rect(0, 0, 1, hi, Wall);
    world.fill_rect(wi - 1, 0, 1, hi, Wall);
    world
}

fn run(world: &mut World, ticks: usize) {
    for _ in 0..ticks {
        world.tick();
    }
}

/// Height of the material column in each interior x of a boxed world.
fn column_heights(world: &World, mat: Material) -> Vec<usize> {
    let h = world.height() as i32;
    (1..world.width() as i32 - 1)
        .map(|x| (1..h - 1).filter(|&y| world.get(x, y) == Some(mat)).count())
        .collect()
}

/// Mean row (0 = top) of all cells of a material.
fn mean_row(world: &World, mat: Material) -> f64 {
    let w = world.width();
    let rows: Vec<usize> = world
        .cells()
        .iter()
        .enumerate()
        .filter(|(_, c)| c.mat == mat)
        .map(|(i, _)| i / w)
        .collect();
    rows.iter().sum::<usize>() as f64 / rows.len() as f64
}

// ---------------------------------------------------------------- movement

#[test]
fn sand_falls_one_cell_per_tick() {
    let mut w = World::new(5, 10, 1);
    w.set(2, 0, Sand);
    for expected in 1..=5 {
        w.tick();
        assert_eq!(w.get(2, expected), Some(Sand), "after {expected} ticks");
        assert_eq!(w.count(Sand), 1);
    }
}

#[test]
fn sand_comes_to_rest_on_the_floor() {
    let mut w = boxed(7, 12, 1);
    w.set(3, 1, Sand);
    run(&mut w, 30);
    assert_eq!(w.get(3, 10), Some(Sand));
}

#[test]
fn sand_piles_into_a_slope() {
    let mut w = boxed(61, 40, 3);
    // Pour a steady stream onto the middle of the floor.
    for _ in 0..300 {
        w.set(30, 1, Sand);
        w.tick();
    }
    run(&mut w, 200);
    let heights = column_heights(&w, Sand);
    let peak = heights.iter().copied().max().unwrap();
    let spread = heights.iter().filter(|&&h| h > 0).count();
    assert!(peak > 5, "should pile up, peak {peak}");
    assert!(
        spread > 2 * peak - 4,
        "should spread into a slope, {spread} wide for peak {peak}"
    );
    // No cliff steeper than one cell: that is the 45 degree angle of repose.
    for pair in heights.windows(2) {
        assert!(pair[0].abs_diff(pair[1]) <= 1, "{heights:?}");
    }
}

#[test]
fn piles_do_not_lean_to_one_side() {
    let mut w = boxed(81, 40, 11);
    for _ in 0..400 {
        w.set(40, 1, Sand);
        w.tick();
    }
    run(&mut w, 200);
    let heights = column_heights(&w, Sand);
    let left: usize = heights[..39].iter().sum();
    let right: usize = heights[40..].iter().sum();
    let total = left + right;
    assert!(
        left.abs_diff(right) * 10 < total,
        "left {left} right {right}"
    );
}

#[test]
fn water_levels_out() {
    let mut w = boxed(42, 30, 5);
    // A tall column of water against the left wall.
    w.fill_rect(1, 4, 6, 25, Water);
    run(&mut w, 800);
    let heights = column_heights(&w, Water);
    let (lo, hi) = (heights.iter().min().unwrap(), heights.iter().max().unwrap());
    assert!(hi - lo <= 1, "surface should be flat: {heights:?}");
}

#[test]
fn oil_floats_on_water() {
    let mut w = boxed(30, 40, 2);
    // Oil starts underneath the water.
    w.fill_rect(1, 28, 28, 10, Oil);
    w.fill_rect(1, 10, 28, 18, Water);
    run(&mut w, 1500);
    assert!(
        mean_row(&w, Oil) + 8.0 < mean_row(&w, Water),
        "oil {:.1} should sit above water {:.1}",
        mean_row(&w, Oil),
        mean_row(&w, Water)
    );
    // The bottom row is all water, the top of the liquid all oil.
    assert!((1..29).all(|x| w.get(x, 38) == Some(Water)));
}

#[test]
fn sand_sinks_through_water() {
    let mut w = boxed(20, 40, 4);
    w.fill_rect(1, 10, 18, 29, Water);
    w.fill_rect(5, 1, 10, 4, Sand);
    run(&mut w, 400);
    // All the sand ends up on the bottom, under the water.
    assert!(
        mean_row(&w, Sand) > 35.0,
        "sand at row {:.1}",
        mean_row(&w, Sand)
    );
    assert!(mean_row(&w, Water) < mean_row(&w, Sand));
}

#[test]
fn steam_rises_through_water() {
    let mut w = boxed(10, 30, 6);
    w.fill_rect(1, 10, 8, 19, Water);
    w.set(5, 27, Steam);
    run(&mut w, 40);
    let rows: Vec<usize> = (0..30)
        .filter(|&y| (0..10).any(|x| w.get(x, y) == Some(Steam)))
        .map(|y| y as usize)
        .collect();
    assert!(
        rows.iter().all(|&y| y < 10),
        "steam should bubble up: {rows:?}"
    );
}

#[test]
fn lava_flows_slower_than_water() {
    let spread = |mat: Material| {
        let mut w = boxed(80, 20, 8);
        w.fill_rect(1, 10, 6, 9, mat);
        run(&mut w, 60);
        column_heights(&w, mat).iter().filter(|&&h| h > 0).count()
    };
    assert!(spread(Lava) * 2 < spread(Water));
}

// ---------------------------------------------------------- conservation

#[test]
fn sand_and_water_are_conserved_in_a_closed_box() {
    let mut w = boxed(50, 50, 9);
    w.fill_rect(5, 5, 20, 20, Sand);
    w.fill_rect(25, 2, 20, 30, Water);
    w.paint(20, 30, 6, Oil);
    let before = (w.count(Sand), w.count(Water), w.count(Oil), w.count(Wall));
    for _ in 0..20 {
        run(&mut w, 50);
        let now = (w.count(Sand), w.count(Water), w.count(Oil), w.count(Wall));
        assert_eq!(now, before);
    }
}

#[test]
fn steam_condenses_back_into_the_same_amount_of_water() {
    let mut w = boxed(30, 30, 10);
    w.fill_rect(5, 15, 20, 10, Steam);
    let steam = w.count(Steam);
    let mut saw_both = false;
    for _ in 0..1000 {
        w.tick();
        saw_both |= w.count(Steam) > 0 && w.count(Water) > 0;
        assert_eq!(w.count(Steam) + w.count(Water), steam);
    }
    assert!(saw_both);
    assert_eq!(w.count(Steam), 0);
    assert_eq!(w.count(Water), steam);
}

// -------------------------------------------------------------- reactions

#[test]
fn fire_burns_wood_away() {
    let mut w = boxed(30, 40, 12);
    w.fill_rect(10, 20, 10, 10, Wood);
    let wood = w.count(Wood);
    w.fill_rect(10, 19, 10, 1, Fire);
    let mut max_smoke = 0;
    let mut max_fire = 0;
    for _ in 0..3000 {
        w.tick();
        max_smoke = max_smoke.max(w.count(Smoke));
        max_fire = max_fire.max(w.count(Fire));
    }
    assert_eq!(w.count(Wood), 0, "all {wood} wood should burn");
    assert!(max_fire > 10, "the fire should spread");
    assert!(max_smoke > 0, "burning should make smoke");
    assert_eq!(w.count(Fire), 0, "the fire should burn out");
    // What is left is ash or nothing.
    for c in w.cells() {
        assert!(matches!(c.mat, Empty | Wall | Ash | Smoke), "{:?}", c.mat);
    }
}

#[test]
fn fire_burns_out_on_its_own() {
    let mut w = boxed(20, 20, 13);
    w.set(10, 10, Fire);
    run(&mut w, 200);
    assert_eq!(w.count(Fire), 0);
}

#[test]
fn oil_burns() {
    let mut w = boxed(30, 20, 14);
    w.fill_rect(1, 14, 28, 5, Oil);
    w.set(15, 13, Fire);
    run(&mut w, 800);
    assert_eq!(w.count(Oil), 0);
}

#[test]
fn water_puts_fire_out_and_turns_to_steam() {
    let mut w = World::new(8, 8, 15);
    w.set(3, 4, Fire);
    w.set(4, 4, Water);
    // A wall under the water so it cannot fall away first.
    w.set(4, 5, Wall);
    w.tick();
    assert_eq!(w.count(Fire), 0, "fire is out");
    assert_eq!(w.count(Water), 0);
    assert_eq!(w.count(Steam), 1, "water boiled into steam");
}

#[test]
fn a_pool_of_water_survives_a_small_fire() {
    let mut w = boxed(20, 20, 16);
    w.fill_rect(1, 10, 18, 9, Water);
    w.fill_rect(8, 8, 4, 2, Fire);
    let water = w.count(Water);
    run(&mut w, 80);
    assert_eq!(w.count(Fire), 0);
    assert!(
        w.count(Water) >= water - 8,
        "only touching cells should boil"
    );
}

#[test]
fn acid_dissolves_sand_and_is_used_up() {
    let mut w = boxed(20, 30, 17);
    w.fill_rect(1, 20, 18, 9, Sand);
    w.fill_rect(5, 5, 10, 3, Acid);
    let (sand, acid) = (w.count(Sand), w.count(Acid));
    run(&mut w, 600);
    let eaten = sand - w.count(Sand);
    let spent = acid - w.count(Acid);
    assert!(eaten > 0, "acid should eat sand");
    assert_eq!(eaten, spent, "one acid cell per dissolved cell");
}

#[test]
fn acid_does_not_dissolve_wall() {
    let mut w = boxed(20, 20, 18);
    w.fill_rect(5, 5, 10, 12, Acid);
    let walls = w.count(Wall);
    let acid = w.count(Acid);
    run(&mut w, 1000);
    assert_eq!(w.count(Wall), walls);
    assert_eq!(w.count(Acid), acid, "nothing to eat, so nothing is spent");
}

#[test]
fn acid_eats_stone_wood_and_ice_too() {
    for target in [Stone, Wood, Ice, Plant] {
        let mut w = boxed(12, 20, 19);
        w.fill_rect(1, 14, 10, 5, target);
        w.fill_rect(1, 4, 10, 4, Acid);
        let before = w.count(target);
        run(&mut w, 400);
        assert!(w.count(target) < before, "{target:?}");
    }
}

#[test]
fn lava_and_water_make_stone_and_steam() {
    let mut w = boxed(30, 30, 20);
    w.fill_rect(1, 20, 28, 9, Water);
    w.fill_rect(10, 2, 10, 4, Lava);
    let mut saw_steam = false;
    for _ in 0..400 {
        w.tick();
        saw_steam |= w.count(Steam) > 0;
    }
    assert!(w.count(Stone) > 0, "quenched lava should turn to stone");
    assert!(saw_steam, "and boil some water");
}

#[test]
fn lava_ignites_wood() {
    let mut w = boxed(20, 20, 21);
    w.fill_rect(1, 15, 18, 4, Wood);
    w.fill_rect(5, 12, 10, 3, Lava);
    let wood = w.count(Wood);
    let mut saw_fire = false;
    for _ in 0..300 {
        w.tick();
        saw_fire |= w.count(Fire) > 0;
    }
    assert!(saw_fire);
    assert!(w.count(Wood) < wood);
}

#[test]
fn plants_grow_into_water() {
    let mut w = boxed(20, 20, 22);
    w.fill_rect(1, 10, 18, 9, Water);
    w.set(10, 18, Plant);
    let total = w.count(Water) + w.count(Plant);
    run(&mut w, 300);
    assert!(
        w.count(Plant) > 20,
        "plant should spread: {}",
        w.count(Plant)
    );
    assert_eq!(
        w.count(Water) + w.count(Plant),
        total,
        "growth consumes water 1:1"
    );
}

#[test]
fn plants_do_not_grow_in_air() {
    let mut w = boxed(20, 20, 23);
    w.set(10, 10, Plant);
    run(&mut w, 500);
    assert_eq!(w.count(Plant), 1);
}

#[test]
fn ice_freezes_water() {
    let mut w = boxed(20, 20, 24);
    w.fill_rect(1, 10, 18, 9, Water);
    w.fill_rect(1, 9, 18, 1, Ice);
    let ice = w.count(Ice);
    run(&mut w, 600);
    assert!(w.count(Ice) > ice);
}

#[test]
fn ice_melts_near_fire() {
    let mut w = boxed(20, 20, 25);
    w.fill_rect(5, 10, 10, 4, Ice);
    let ice = w.count(Ice);
    let mut melted = false;
    for _ in 0..60 {
        w.fill_rect(5, 14, 10, 1, Fire);
        w.tick();
        melted |= w.count(Water) > 0 || w.count(Steam) > 0;
    }
    assert!(melted);
    assert!(w.count(Ice) < ice);
}

#[test]
fn smoke_rises_and_fades() {
    let mut w = boxed(20, 40, 26);
    w.fill_rect(5, 30, 10, 5, Smoke);
    let start = mean_row(&w, Smoke);
    run(&mut w, 10);
    assert!(mean_row(&w, Smoke) < start, "smoke should rise");
    run(&mut w, 300);
    assert_eq!(w.count(Smoke), 0, "smoke should fade away");
}

#[test]
fn wall_never_changes() {
    let mut w = boxed(30, 30, 27);
    w.fill_rect(10, 10, 10, 10, Wall);
    let snapshot: Vec<bool> = w.cells().iter().map(|c| c.mat == Wall).collect();
    w.fill_rect(1, 1, 28, 8, Lava);
    w.fill_rect(1, 20, 28, 9, Acid);
    w.stamp(5, 15, 3, Fire);
    run(&mut w, 500);
    let after: Vec<bool> = w.cells().iter().map(|c| c.mat == Wall).collect();
    assert_eq!(snapshot, after);
}

// ------------------------------------------------------------ determinism

fn busy_scene(seed: u32) -> World {
    let mut w = boxed(64, 48, seed);
    w.fill_rect(4, 4, 20, 10, Sand);
    w.fill_rect(30, 4, 20, 10, Water);
    w.fill_rect(10, 30, 30, 4, Wood);
    w.stamp(20, 28, 2, Fire);
    w.fill_rect(45, 20, 10, 5, Lava);
    w.paint(52, 30, 4, Oil);
    w.paint(8, 20, 3, Acid);
    w
}

#[test]
fn same_seed_gives_an_identical_grid() {
    let mut a = busy_scene(99);
    let mut b = busy_scene(99);
    for _ in 0..500 {
        a.tick();
        b.tick();
    }
    assert_eq!(a.cells(), b.cells());
    a.render();
    b.render();
    assert_eq!(a.pixels(), b.pixels());
}

#[test]
fn different_seeds_diverge() {
    let mut a = busy_scene(1);
    let mut b = busy_scene(2);
    run(&mut a, 200);
    run(&mut b, 200);
    assert_ne!(a.cells(), b.cells());
}

#[test]
fn rendering_does_not_affect_the_simulation() {
    let mut a = busy_scene(7);
    let mut b = busy_scene(7);
    for _ in 0..300 {
        a.tick();
        a.render();
        b.tick();
    }
    assert_eq!(a.cells(), b.cells());
}

#[test]
fn nothing_escapes_or_appears_from_nowhere_at_the_edges() {
    // No border walls: material must stop at the grid edge, not wrap.
    let mut w = World::new(16, 16, 28);
    w.fill_rect(0, 0, 16, 4, Water);
    let water = w.count(Water);
    run(&mut w, 300);
    assert_eq!(w.count(Water), water);
    assert!((0..16).all(|x| w.get(x, 15) == Some(Water)));
}
