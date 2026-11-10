// Integration tests: the mod's hooks driven by a model of Horizon's orb engine
// (tests/common/mod.rs). Numbering follows the list of the porting brief.

mod common;

use clumps_horizon::{Host, Orb};
use common::Sim;

const OVERWORLD: &str = "minecraft:overworld";
const NETHER: &str = "minecraft:the_nether";
const P1: u128 = 0xA1;
const P2: u128 = 0xA2;

// ---- 1. two nearby orbs merge ----

#[test]
fn t01_two_nearby_orbs_merge_even_with_different_values() {
    let mut g = Sim::new(OVERWORLD);
    g.orb(1, 0.0, 3).orb(2, 0.2, 5);
    g.scan_all();
    assert_eq!(g.orbs.len(), 1);
    assert_eq!(g.orb_xp(), 8);
}

#[test]
fn t01_vanilla_alone_would_not_merge_those() {
    let mut g = Sim::new(OVERWORLD);
    g.enabled = false;
    g.orb(1, 0.0, 3).orb(2, 0.2, 5);
    g.scan_all();
    assert_eq!(g.orbs.len(), 2);
}

// ---- 2. multiple orbs merge ----

#[test]
fn t02_many_orbs_collapse_into_few() {
    let mut g = Sim::new(OVERWORLD);
    for (i, v) in [1u32, 3, 5, 11, 17, 37, 73, 149].iter().enumerate() {
        g.orb(i as u32 + 1, i as f64 * 0.05, *v);
    }
    let before = g.orb_xp();
    for _ in 0..4 {
        g.scan_all();
    }
    assert!(g.orbs.len() <= 2, "left {:?}", g.orbs);
    assert_eq!(g.orb_xp(), before);
}

#[test]
fn t02_orbs_spawning_next_to_an_orb_join_it() {
    let mut g = Sim::new(OVERWORLD);
    g.orb(1, 0.0, 11);
    for (n, v) in [3u32, 5, 7, 11, 13].into_iter().enumerate() {
        g.award_orb([0.1, 0.0, 0.0], v, n as i32);
    }
    // 26 + 11 = 37 is prime above the count cap, so that one spawn is refused and stays its own orb.
    assert!(g.orbs.len() <= 2, "{:?}", g.orbs);
    assert_eq!(g.orb_xp(), 11 + 3 + 5 + 7 + 11 + 13);
}

// ---- 3. orbs outside the radius stay apart ----

#[test]
fn t03_far_orbs_stay_separate() {
    let mut g = Sim::new(OVERWORLD);
    g.orb(1, 0.0, 3).orb(2, 2.0, 3);
    g.scan_all();
    assert_eq!(g.orbs.len(), 2);
}

#[test]
fn t03_the_reach_is_vanillas_not_wider() {
    // A scanning orb reaches 0.5 past its box: boxes 0.5 wide at 0 and 1.0 only touch faces.
    let mut g = Sim::new(OVERWORLD);
    g.orb(1, 0.0, 3).orb(2, 1.0, 5);
    g.scan_all();
    assert_eq!(g.orbs.len(), 2, "touching faces do not merge");
    let mut g = Sim::new(OVERWORLD);
    g.orb(1, 0.0, 3).orb(2, 0.99, 5);
    g.scan_all();
    assert_eq!(g.orbs.len(), 1);
}

#[test]
fn t03_a_new_orb_far_from_the_others_spawns_on_its_own() {
    let mut g = Sim::new(OVERWORLD);
    g.orb(1, 0.0, 3);
    g.award_orb([5.0, 0.0, 0.0], 3, 0);
    assert_eq!(g.orbs.len(), 2);
}

// ---- 4 and 5. XP aggregation, orbs of different amounts ----

#[test]
fn t04_t05_the_merged_orb_is_worth_the_sum() {
    for (a, b) in [(37u32, 11u32), (3, 5), (73, 149), (1, 1), (617, 1237), (2477, 2477)] {
        let mut g = Sim::new(OVERWORLD);
        g.orb(1, 0.0, a).orb(2, 0.1, b);
        g.scan_all();
        assert_eq!(g.orb_xp(), u64::from(a + b), "{a} + {b}");
    }
}

#[test]
fn t05_the_orb_is_worth_one_value_times_a_small_count() {
    let mut g = Sim::new(OVERWORLD);
    g.orb(1, 0.0, 37).orb(2, 0.1, 11);
    g.scan_all();
    assert_eq!((g.orbs[0].value, g.orbs[0].count), (24, 2));
}

#[test]
fn t05_a_total_no_count_can_make_is_refused_and_nothing_is_lost() {
    // 37 + 4 = 41 is prime and above the count cap: the orbs stay as they are.
    let mut g = Sim::new(OVERWORLD);
    g.orb(1, 0.0, 37).orb(2, 0.1, 4);
    g.scan_all();
    assert_eq!(g.orbs.len(), 2);
    assert_eq!(g.orb_xp(), 41);
}

// ---- 6. independent clusters ----

#[test]
fn t06_clusters_far_apart_merge_separately() {
    let mut g = Sim::new(OVERWORLD);
    g.orb(1, 0.0, 3).orb(2, 0.1, 5).orb(3, 0.2, 7);
    g.orb(11, 50.0, 1).orb(12, 50.1, 1);
    g.scan_all();
    assert_eq!(g.orbs.len(), 2);
    let near: u64 = g.orbs.iter().filter(|o| o.pos[0] < 10.0).map(|o| o.xp()).sum();
    let far: u64 = g.orbs.iter().filter(|o| o.pos[0] > 10.0).map(|o| o.xp()).sum();
    assert_eq!((near, far), (15, 2));
}

// ---- 7. immediate pickup ----

#[test]
fn t07_a_server_player_takes_the_whole_clump_in_one_touch() {
    let mut g = Sim::new(OVERWORLD);
    g.orb(1, 0.0, 37).orb(2, 0.1, 11);
    g.scan_all();
    g.player(P1, 0.0);
    g.guest_tick(0);
    assert_eq!(g.players[0].xp, 48);
    assert_eq!(g.players[0].takes, 1);
    assert!(g.orbs.is_empty(), "the orb is gone after the one touch");
}

#[test]
fn t07_without_the_mod_a_touch_gives_one_count() {
    let mut g = Sim::new(OVERWORLD);
    g.enabled = false;
    g.orb_counted(1, 0.0, 24, 2);
    g.player(P1, 0.0);
    g.guest_tick(0);
    assert_eq!(g.players[0].xp, 24);
    assert_eq!(g.orbs.len(), 1);
}

#[test]
fn t07_the_local_player_still_takes_a_count_per_touch_but_loses_nothing() {
    // Horizon asks no pickup hook for the local player: this is the engine's own pace.
    let mut g = Sim::new(OVERWORLD);
    g.orb(1, 0.0, 37).orb(2, 0.1, 11);
    g.scan_all();
    g.player(P1, 0.0);
    for _ in 0..10 {
        g.host_tick(0);
    }
    assert_eq!(g.players[0].xp, 48);
    assert_eq!(g.players[0].takes, 2);
    assert!(g.orbs.is_empty());
}

// ---- 8. multiple players ----

#[test]
fn t08_two_players_at_one_orb_share_nothing_twice() {
    let mut g = Sim::new(OVERWORLD);
    g.orb_counted(1, 0.0, 24, 2);
    g.player(P1, 0.0).player(P2, 0.0);
    g.guest_tick(0);
    g.guest_tick(1);
    assert_eq!(g.players[0].xp, 48);
    assert_eq!(g.players[1].xp, 0);
    assert_eq!(g.player_xp(), 48);
}

#[test]
fn t08_two_players_two_orbs_one_each() {
    let mut g = Sim::new(OVERWORLD);
    g.orb_counted(1, 0.0, 24, 2).orb_counted(2, 0.0, 10, 3);
    g.player(P1, 0.0).player(P2, 0.0);
    g.guest_tick(0);
    g.guest_tick(1);
    assert_eq!(g.player_xp() + g.orb_xp(), 48 + 30);
}

// ---- 9 and 10. no duplicated XP, no lost XP ----

#[test]
fn t09_t10_a_failed_give_keeps_its_count_on_the_orb() {
    let mut g = Sim::new(OVERWORLD);
    g.gives_fail = true;
    g.orb_counted(1, 0.0, 24, 3);
    g.player(P1, 0.0);
    g.guest_tick(0);
    assert_eq!(g.players[0].xp, 24, "the engine takes its one count");
    assert_eq!(g.orb_xp(), 48, "the other two counts are still on the orb");
}

#[test]
fn t09_t10_a_cancel_by_another_mod_gives_nothing() {
    let mut g = Sim::new(OVERWORLD);
    g.foreign_cancel_pickup = true;
    g.orb_counted(1, 0.0, 24, 3);
    g.player(P1, 0.0);
    g.guest_tick(0);
    assert_eq!(g.players[0].xp, 0);
    assert_eq!(g.orb_xp(), 72);
}

#[test]
fn t09_t10_a_cancel_by_another_mod_stops_the_merge() {
    let mut g = Sim::new(OVERWORLD);
    g.foreign_cancel_merge = true;
    g.orb(1, 0.0, 3).orb(2, 0.1, 5);
    g.scan_all();
    assert_eq!(g.orbs.len(), 2);
    assert_eq!((g.orbs[0].value, g.orbs[1].value), (3, 5));
}

// A small deterministic generator, so a failure repeats.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self, n: u64) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 33) % n
    }
}

#[test]
fn t09_t10_experience_is_conserved_through_merges_and_pickups() {
    const VALUES: [u32; 12] = [1, 3, 5, 11, 17, 37, 73, 149, 307, 617, 1237, 2477];
    for seed in 0..300u64 {
        let mut rng = Lcg(seed + 1);
        let mut g = Sim::new(OVERWORLD);
        g.pick = |n| n / 2;
        let mut spawned: u64 = 0;
        let hosts = rng.next(2) == 0;
        g.player(P1, 0.0).player(P2, 0.3);
        for step in 0..120u32 {
            // Orbs spawn around the players and a far pile.
            for _ in 0..rng.next(3) {
                let v = VALUES[rng.next(12) as usize];
                let x = [0.0, 0.2, 0.4, 20.0][rng.next(4) as usize] + rng.next(10) as f64 * 0.01;
                spawned += u64::from(v);
                g.award_orb([x, 0.0, 0.0], v, rng.next(40) as i32);
            }
            if step % 20 == 1 {
                g.scan_all();
            }
            if hosts {
                g.host_tick(0);
            } else {
                g.guest_tick(0);
            }
            g.guest_tick(1);
            assert_eq!(g.total_xp(), spawned, "seed {seed} step {step}: {:?}", g.orbs);
            assert!(g.orbs.iter().all(|o| o.value > 0 && o.count > 0));
        }
    }
}

// ---- 11. dimensions ----

#[test]
fn t11_a_merge_asked_for_another_level_does_nothing() {
    let mut g = Sim::new(OVERWORLD);
    g.orb(1, 0.0, 3).orb(2, 0.1, 5);
    let mut canceled = false;
    clumps_horizon::on_merge(&mut g, NETHER, 1, 2, 5, &mut canceled);
    assert!(canceled, "no such orb in that level: the merge is stopped");
    assert_eq!((g.orbs[0].value, g.orbs[0].count), (3, 1));
}

#[test]
fn t11_levels_with_the_same_ids_merge_on_their_own() {
    let mut nether = Sim::new(NETHER);
    nether.orb(1, 0.0, 3).orb(2, 0.1, 5);
    let mut overworld = Sim::new(OVERWORLD);
    overworld.orb(1, 0.0, 3).orb(2, 30.0, 5);
    nether.scan_all();
    overworld.scan_all();
    assert_eq!(nether.orbs.len(), 1);
    assert_eq!(overworld.orbs.len(), 2);
    assert!(!Host::orbs(&nether, NETHER).is_empty());
    assert!(Host::orbs(&nether, OVERWORLD).is_empty());
}

// ---- 12. disabled ----

#[test]
fn t12_turned_off_the_mod_answers_with_vanilla() {
    let mut g = Sim::new(OVERWORLD);
    g.enabled = false;
    assert!(!clumps_horizon::can_merge(&g, 1, 2, false));
    assert!(clumps_horizon::can_merge(&g, 1, 2, true));
    assert!(!clumps_horizon::listens(&g));
}

#[test]
fn t12_turned_off_vanilla_stacking_still_works() {
    let mut g = Sim::new(OVERWORLD);
    g.enabled = false;
    // Same value, ids 40 apart: vanilla merges them, the count adds.
    g.orb(1, 0.0, 3).orb(41, 0.1, 3);
    g.scan_all();
    assert_eq!(g.orbs.len(), 1);
    assert_eq!((g.orbs[0].value, g.orbs[0].count), (3, 2));
}

#[test]
fn t12_turned_off_nothing_is_written() {
    let mut g = Sim::new(OVERWORLD);
    g.enabled = false;
    let mut canceled = false;
    g.orb(1, 0.0, 3).orb(2, 0.1, 5);
    clumps_horizon::on_merge(&mut g, OVERWORLD, 1, 2, 5, &mut canceled);
    assert!(!canceled);
    assert_eq!((g.orbs[0].value, g.orbs[0].count), (3, 1));
}

// ---- 13. lifecycle ----

#[test]
fn t13_the_keeper_stays_and_the_absorbed_orb_leaves() {
    let mut g = Sim::new(OVERWORLD);
    g.orb(7, 0.0, 3).orb(9, 0.1, 5);
    g.orbs[1].age = 2;
    g.scan_all();
    assert_eq!(g.orbs.len(), 1);
    assert_eq!(g.orbs[0].id, 7);
    assert_eq!(g.removed, vec![9]);
    assert_eq!(g.orbs[0].age, 2, "the smaller age is kept");
}

#[test]
fn t13_a_spawn_merge_restarts_the_age() {
    let mut g = Sim::new(OVERWORLD);
    g.orb(1, 0.0, 3);
    g.award_orb([0.0, 0.0, 0.0], 5, 0);
    assert_eq!(g.orbs.len(), 1);
    assert_eq!(g.orbs[0].age, 0);
    assert_eq!(g.orb_xp(), 8);
}

#[test]
fn t13_a_picked_up_clump_leaves_no_orb_behind() {
    let mut g = Sim::new(OVERWORLD);
    g.orb(1, 0.0, 3).orb(2, 0.1, 5).orb(3, 0.2, 11);
    g.scan_all();
    g.player(P1, 0.0);
    g.guest_tick(0);
    assert!(g.orbs.is_empty());
    assert_eq!(g.players[0].xp, 19);
    assert_eq!(g.removed.len(), 3, "two merged away, one picked up");
}

#[test]
fn t13_the_orb_is_never_written_with_zero() {
    let mut g = Sim::new(OVERWORLD);
    g.orb(1, 0.0, 3);
    assert!(!g.set_orb(OVERWORLD, 1, Orb::new(0, 1)));
    assert!(!g.set_orb(OVERWORLD, 1, Orb::new(1, 0)));
    assert!(!g.set_orb(OVERWORLD, 99, Orb::new(1, 1)));
}
