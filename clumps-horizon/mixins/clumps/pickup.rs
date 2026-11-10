// Clumps' `playerTouch`: a server player takes the whole clump in one touch.
fn clumps_pickup(
    game: &mut crate::game::Game<'_>,
    player: u128,
    value: &mut u32,
    count: &mut u32,
    canceled: &bool,
    result: (),
) {
    let () = result;
    crate::game::clumps_glue::picked_up(game, player, value, count, *canceled);
}
