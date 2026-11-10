// Clumps' `canMerge`: any two live orbs may merge (vanilla asks for the same value and a merge id).
fn clumps_can_merge(game: &mut crate::game::Game<'_>, into: u32, from: u32, can_merge: &mut bool) {
    crate::game::clumps_glue::can_merge(game, into, from, can_merge);
}
