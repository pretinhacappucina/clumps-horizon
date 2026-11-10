// Ask the server to swap a player in for the pickup hook, as long as the mod is on.
fn clumps_listened(game: &crate::game::Game<'_>, listened: &mut bool, value: ()) {
    let () = value;
    crate::game::clumps_glue::listened(game, listened);
}
