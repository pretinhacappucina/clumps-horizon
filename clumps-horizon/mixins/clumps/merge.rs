// After the listeners: keep every point of the two orbs, or cancel the merge (both orbs stay).
fn clumps_merge(
    game: &mut crate::game::Game<'_>,
    dimension: &str,
    into: u32,
    from: u32,
    from_value: u32,
    canceled: &mut bool,
    value: (),
) {
    let () = value;
    crate::game::clumps_glue::merged(game, dimension, into, from, from_value, canceled);
}
