// Horizon's `Game` as the core's `Host`. Injected beside `crate::game::clumps`, so it sees
// the `pub(super)` server helpers of `game::native_server` and `game::native_hooks`.
// Every call here is a method whose signature was read in the pinned source:
//   native_on(&self, &str) -> bool                              (native_hooks.rs)
//   native_experience_orbs(&self, &str, [f64;3], [f64;3])       (native_server.rs, API 101)
//   native_set_experience_orb(&mut self, &str, u32, Option<u32>, Option<u32>, Option<u32>) -> bool (API 128)
//   native_give_experience(&mut self, u128, u32) -> bool        (API 101)

use crate::game::clumps::{Host, ID, Orb, Seen};

// An orb's box must meet this one to be listed: all of any level (the border is 30 000 000).
const ALL_MIN: [f64; 3] = [-1.0e9, -1.0e9, -1.0e9];
const ALL_MAX: [f64; 3] = [1.0e9, 1.0e9, 1.0e9];

pub struct GameHost<'a, 'b>(pub &'a mut crate::game::Game<'b>);

impl Host for GameHost<'_, '_> {
    fn enabled(&self) -> bool {
        self.0.native_on(ID)
    }

    fn orbs(&self, dimension: &str) -> Vec<Seen> {
        self.0
            .native_experience_orbs(dimension, ALL_MIN, ALL_MAX)
            .into_iter()
            .map(|o| Seen {
                id: o.id,
                orb: Orb::new(o.value, o.count),
            })
            .collect()
    }

    fn set_orb(&mut self, dimension: &str, id: u32, orb: Orb) -> bool {
        self.0
            .native_set_experience_orb(dimension, id, Some(orb.value), Some(orb.count), None)
    }

    fn give_experience(&mut self, player: u128, points: u32) -> bool {
        self.0.native_give_experience(player, points)
    }
}

pub(super) fn can_merge(game: &mut crate::game::Game<'_>, into: u32, from: u32, can_merge: &mut bool) {
    let host = GameHost(game);
    *can_merge = crate::game::clumps::can_merge(&host, into, from, *can_merge);
}

pub(super) fn merged(
    game: &mut crate::game::Game<'_>,
    dimension: &str,
    into: u32,
    from: u32,
    from_value: u32,
    canceled: &mut bool,
) {
    let mut host = GameHost(game);
    crate::game::clumps::on_merge(&mut host, dimension, into, from, from_value, canceled);
}

pub(super) fn listened(game: &crate::game::Game<'_>, listened: &mut bool) {
    // Read-only hook: ask for the pickup hook while the mod is on.
    if game.native_on(ID) {
        *listened = true;
    }
}

pub(super) fn picked_up(
    game: &mut crate::game::Game<'_>,
    player: u128,
    value: &mut u32,
    count: &mut u32,
    canceled: bool,
) {
    let mut host = GameHost(game);
    crate::game::clumps::on_pickup(&mut host, player, value, count, canceled);
}
