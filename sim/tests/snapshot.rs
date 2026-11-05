//! Save/load (run-length snapshot) tests.

use dhool_sim::{DecodeError, Material, World};

fn materials(w: &World) -> Vec<Material> {
    w.cells().iter().map(|c| c.mat).collect()
}

fn scene() -> World {
    let mut w = World::new(120, 80, 5);
    w.fill_rect(0, 70, 120, 10, Material::Sand);
    w.fill_rect(10, 40, 30, 20, Material::Water);
    w.stamp(80, 30, 12, Material::Wood);
    w.paint(60, 10, 8, Material::Oil);
    for _ in 0..50 {
        w.tick();
    }
    w
}

#[test]
fn round_trip_preserves_every_material() {
    let w = scene();
    let bytes = w.save();
    let back = World::load(&bytes, 1).expect("decodes");
    assert_eq!((back.width(), back.height()), (w.width(), w.height()));
    assert_eq!(materials(&back), materials(&w));
}

#[test]
fn round_trip_of_every_material_id() {
    let mut w = World::new(dhool_sim::MATERIAL_COUNT, 3, 1);
    for (x, m) in Material::ALL.iter().enumerate() {
        w.set(x as i32, 1, *m);
    }
    let back = World::load(&w.save(), 1).unwrap();
    assert_eq!(materials(&back), materials(&w));
}

#[test]
fn empty_world_is_tiny() {
    let w = World::new(320, 200, 1);
    // Header plus one run of 64,000 empties.
    assert!(w.save().len() <= 12, "{} bytes", w.save().len());
}

#[test]
fn busy_scene_stays_compact() {
    let w = scene();
    let raw = w.width() * w.height();
    assert!(
        w.save().len() * 4 < raw,
        "{} bytes for {raw} cells",
        w.save().len()
    );
}

#[test]
fn loaded_world_keeps_simulating() {
    let w = scene();
    let mut back = World::load(&w.save(), 3).unwrap();
    let water = back.count(Material::Water);
    for _ in 0..100 {
        back.tick();
    }
    assert_eq!(back.count(Material::Water), water);
}

#[test]
fn load_is_deterministic_for_a_seed() {
    let bytes = scene().save();
    let mut a = World::load(&bytes, 42).unwrap();
    let mut b = World::load(&bytes, 42).unwrap();
    for _ in 0..100 {
        a.tick();
        b.tick();
    }
    assert_eq!(a.cells(), b.cells());
}

#[test]
fn rejects_garbage() {
    assert_eq!(World::load(b"", 1).unwrap_err(), DecodeError::Truncated);
    assert_eq!(World::load(b"DH", 1).unwrap_err(), DecodeError::Truncated);
    assert_eq!(
        World::load(b"PNG\x01\x01\x00\x01\x00", 1).unwrap_err(),
        DecodeError::BadMagic
    );
    assert_eq!(
        World::load(b"hello world", 1).unwrap_err(),
        DecodeError::BadMagic
    );
}

#[test]
fn rejects_unknown_version() {
    let mut bytes = World::new(4, 4, 1).save();
    bytes[3] = 9;
    assert_eq!(
        World::load(&bytes, 1).unwrap_err(),
        DecodeError::BadVersion(9)
    );
}

#[test]
fn rejects_bad_dimensions() {
    // 0 x 4
    assert_eq!(
        World::load(b"DHL\x01\x00\x00\x04\x00\x00\x00", 1).unwrap_err(),
        DecodeError::BadSize
    );
    // 65535 x 65535 must not try to allocate four billion cells.
    assert_eq!(
        World::load(b"DHL\x01\xff\xff\xff\xff\x00\x00", 1).unwrap_err(),
        DecodeError::BadSize
    );
}

#[test]
fn rejects_unknown_material() {
    let mut bytes = World::new(4, 4, 1).save();
    bytes[8] = 200;
    assert_eq!(
        World::load(&bytes, 1).unwrap_err(),
        DecodeError::BadMaterial(200)
    );
}

#[test]
fn rejects_runs_that_do_not_cover_the_grid() {
    let mut w = World::new(4, 4, 1);
    w.set(0, 0, Material::Sand);
    let bytes = w.save();
    // Drop the last run: too short.
    assert_eq!(
        World::load(&bytes[..bytes.len() - 2], 1).unwrap_err(),
        DecodeError::LengthMismatch
    );
    // Add an extra run: too long.
    let mut long = bytes.clone();
    long.extend_from_slice(&[2, 0]);
    assert_eq!(
        World::load(&long, 1).unwrap_err(),
        DecodeError::LengthMismatch
    );
    // Cut a varint in half.
    let mut cut = World::new(400, 1, 1).save();
    cut.pop();
    assert_eq!(World::load(&cut, 1).unwrap_err(), DecodeError::Truncated);
}

#[test]
fn errors_have_messages() {
    assert_eq!(
        DecodeError::BadMaterial(7).to_string(),
        "unknown material id 7"
    );
}
