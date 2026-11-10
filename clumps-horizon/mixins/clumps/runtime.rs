// Clumps for Horizon: the testable core. This file is both the standalone Cargo
// library and the injected `crate::game::clumps` module (no generated copy exists),
// so it uses only `std` and plain Rust types. The game is reached through `Host`,
// which `glue.rs` implements for Horizon's `Game`.
//
// Original: Clumps by Jaredlll08 (MIT), branch 26.3, `MixinExperienceOrb`:
//   canMerge        -> `can_merge`
//   merge / tryMergeToExisting -> `plan_merge` + `on_merge`
//   playerTouch     -> `on_pickup`
// See PORTING_NOTES.md for the full mapping and the deliberate differences.

pub const ID: &str = "clumps";

// Horizon keeps an orb as `value` points per `count`: a pickup gives `value` and takes one
// count. Clumps keeps a value -> amount map and sets `value` to the sum. Horizon adds the
// merged orb's count AFTER its merge hook runs, so a mod cannot leave `count == 1`. Instead
// the keeper is written so that `value * count` is exactly the combined XP once Horizon has
// added the count (see `plan_merge`). `count` is kept at or under this unless both orbs
// already have the same value (vanilla's own stacking), because a host player collects one
// count per pickup.
pub const MAX_COUNT: u32 = 32;
// How far `plan_merge` looks for a count; anything beyond is refused (equal values excepted).
pub const SEARCH_LIMIT: u64 = 4096;
// Horizon clamps a pickup's value to 1..=i32::MAX.
pub const MAX_VALUE: u32 = i32::MAX as u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Orb {
    pub value: u32,
    pub count: u32,
}

impl Orb {
    pub fn new(value: u32, count: u32) -> Self {
        Orb { value, count }
    }

    // The points this orb is worth in all: what its pickups give together.
    pub fn xp(self) -> u64 {
        u64::from(self.value) * u64::from(self.count)
    }

    fn valid(self) -> bool {
        self.value > 0 && self.count > 0
    }
}

// An orb of a level as the host lists it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seen {
    pub id: u32,
    pub orb: Orb,
}

pub trait Host {
    // False when the mod is turned off in Options > Mods (`native_on`).
    fn enabled(&self) -> bool;
    // Every orb of the level `dimension`, in the level's order.
    fn orbs(&self, dimension: &str) -> Vec<Seen>;
    // Writes an orb's value and count. False, and nothing written, for an orb that isn't there.
    fn set_orb(&mut self, dimension: &str, id: u32, orb: Orb) -> bool;
    // Gives points to the player as an orb would (Mending first). False when it gave nothing.
    fn give_experience(&mut self, player: u128, points: u32) -> bool;
}

// `ExperienceOrb.canMerge`, which Clumps replaces with `other.isAlive() && !this.is(other)`
// (and, for a new orb, `other.isAlive()`): value and merge id no longer matter. `from` is 0 for
// an orb that is only spawning. Horizon only asks about live orbs whose boxes meet, which is
// vanilla's reach (1 block at spawn, 0.5 around an orb every 20 ticks): Clumps adds no radius.
pub fn can_merge<H: Host>(host: &H, into: u32, from: u32, vanilla: bool) -> bool {
    if !host.enabled() {
        return vanilla;
    }
    from == 0 || into != from
}

// What to write into the keeper before Horizon adds `from.count` to its own, so that the
// merged orb is worth `into + from` exactly. `None` refuses the merge: the two orbs stay as
// they are, and nothing is lost.
//
// Horizon's result is `count = written.count + from.count` and `value = written.value`, so
// the final count `c` must be at least `from.count + 1`, must divide the total, and the value
// is `total / c`. The smallest such `c` wins: the fewest pickups for a host player, and the
// biggest orb.
pub fn plan_merge(into: Orb, from: Orb) -> Option<Orb> {
    if !into.valid() || !from.valid() {
        return None;
    }
    let total = into.xp() + from.xp();
    let inc = u64::from(from.count);
    let natural = u64::from(into.count) + inc;
    let hi = u64::from(MAX_COUNT)
        .max(natural.min(SEARCH_LIMIT))
        .min(u64::from(u32::MAX));
    let mut c = inc + 1;
    while c <= hi {
        if total % c == 0 {
            let value = total / c;
            if (1..=u64::from(MAX_VALUE)).contains(&value) {
                return Some(Orb::new(value as u32, (c - inc) as u32));
            }
        }
        c += 1;
    }
    // Vanilla's own stacking: same value, counts add. Exact whatever the counts.
    if into.value == from.value && natural <= u64::from(u32::MAX) {
        return Some(into);
    }
    None
}

// After `native_experience_orb_merge`'s listeners ran: `into` is about to take `from` (0: a
// new orb of `from_value` that is spawning). Writes the keeper, or cancels the merge.
pub fn on_merge<H: Host>(
    host: &mut H,
    dimension: &str,
    into: u32,
    from: u32,
    from_value: u32,
    canceled: &mut bool,
) {
    if !host.enabled() || *canceled {
        return;
    }
    if into == from {
        *canceled = true;
        return;
    }
    let seen = host.orbs(dimension);
    let find = |id: u32| seen.iter().find(|s| s.id == id).map(|s| s.orb);
    let Some(keeper) = find(into) else {
        *canceled = true;
        return;
    };
    let absorbed = if from == 0 {
        Orb::new(from_value, 1)
    } else {
        match find(from) {
            Some(orb) => orb,
            None => {
                *canceled = true;
                return;
            }
        }
    };
    match plan_merge(keeper, absorbed) {
        None => *canceled = true,
        Some(write) => {
            if write != keeper && !host.set_orb(dimension, into, write) {
                *canceled = true;
            }
        }
    }
}

// Whether the pickup hook has to be asked for at all: with this off the server does not swap
// the player in, so the mod costs nothing when it is turned off.
pub fn listens<H: Host>(host: &H) -> bool {
    host.enabled()
}

// After `native_experience_orb_pickup`'s listeners ran, for a server player taking an orb.
// Horizon takes one count per pickup. Clumps' `playerTouch` gives the whole clump and
// discards the orb, so the other counts go to the player here, each as an orb of `value`
// (Mending sees each one, as in Clumps), and the orb is left with the one count Horizon
// takes. A give that fails keeps its count on the orb: no XP is lost, none given twice.
pub fn on_pickup<H: Host>(
    host: &mut H,
    player: u128,
    value: &mut u32,
    count: &mut u32,
    canceled: bool,
) {
    if !host.enabled() || canceled || *count <= 1 || *value == 0 {
        return;
    }
    let extra = *count - 1;
    let mut given = 0;
    while given < extra && host.give_experience(player, *value) {
        given += 1;
    }
    *count -= given;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine_adds(written: Orb, from: Orb) -> Orb {
        Orb::new(written.value, written.count.saturating_add(from.count))
    }

    #[test]
    fn xp_is_value_times_count() {
        assert_eq!(Orb::new(37, 3).xp(), 111);
    }

    #[test]
    fn two_equal_orbs_stay_vanilla() {
        let into = Orb::new(3, 1);
        let from = Orb::new(3, 1);
        let w = plan_merge(into, from).unwrap();
        assert_eq!(engine_adds(w, from), Orb::new(3, 2));
    }

    #[test]
    fn different_values_keep_every_point() {
        let into = Orb::new(3, 1);
        let from = Orb::new(5, 1);
        let w = plan_merge(into, from).unwrap();
        let end = engine_adds(w, from);
        assert_eq!(end.xp(), 8);
        assert_eq!(end, Orb::new(4, 2));
    }

    #[test]
    fn odd_totals_use_the_smallest_divisor() {
        let from = Orb::new(5, 1);
        let end = engine_adds(plan_merge(Orb::new(7, 1), from).unwrap(), from);
        assert_eq!(end.xp(), 12);
        assert_eq!(end, Orb::new(6, 2));
        let from = Orb::new(11, 1);
        let end = engine_adds(plan_merge(Orb::new(10, 1), from).unwrap(), from);
        assert_eq!(end, Orb::new(7, 3));
    }

    #[test]
    fn a_total_that_cannot_be_split_is_refused() {
        // 3 + 4 = 7 is prime and above no divisor between 2 and 32 but 7 itself: count 7 is
        // allowed (<= MAX_COUNT), value 1.
        let from = Orb::new(4, 1);
        let end = engine_adds(plan_merge(Orb::new(3, 1), from).unwrap(), from);
        assert_eq!(end.xp(), 7);
        // A prime above MAX_COUNT has no allowed count at all.
        assert_eq!(plan_merge(Orb::new(37, 1), Orb::new(4, 1)), None);
    }

    #[test]
    fn zero_orbs_are_refused() {
        assert_eq!(plan_merge(Orb::new(0, 1), Orb::new(3, 1)), None);
        assert_eq!(plan_merge(Orb::new(3, 0), Orb::new(3, 1)), None);
        assert_eq!(plan_merge(Orb::new(3, 1), Orb::new(3, 0)), None);
    }

    #[test]
    fn big_orbs_with_the_same_value_merge_past_the_cap() {
        let into = Orb::new(1, 40_000);
        let from = Orb::new(1, 5);
        let w = plan_merge(into, from).unwrap();
        let end = engine_adds(w, from);
        assert_eq!(end.xp(), 40_005);
    }

    #[test]
    fn value_never_passes_the_clamp() {
        let big = Orb::new(MAX_VALUE, 1);
        let end_of = |a: Orb, b: Orb| plan_merge(a, b).map(|w| engine_adds(w, b));
        // MAX + MAX = 2 * MAX: count 2 would give value MAX: allowed.
        assert_eq!(end_of(big, big).map(|o| o.xp()), Some(2 * u64::from(MAX_VALUE)));
        assert!(end_of(big, big).unwrap().value <= MAX_VALUE);
    }

    #[test]
    fn the_planned_orb_always_has_a_count_to_add_to() {
        for a in 1..40u32 {
            for b in 1..40u32 {
                for ca in 1..5u32 {
                    for cb in 1..5u32 {
                        let (i, f) = (Orb::new(a, ca), Orb::new(b, cb));
                        if let Some(w) = plan_merge(i, f) {
                            assert!(w.count >= 1 && w.value >= 1);
                            let end = engine_adds(w, f);
                            assert_eq!(end.xp(), i.xp() + f.xp(), "{i:?} + {f:?}");
                        }
                    }
                }
            }
        }
    }
}
