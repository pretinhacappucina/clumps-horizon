// A model of Horizon's experience orb code at the pinned commit
// (fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca), ported from the functions it names:
//   Game::merge_new_orb / orb_merges / merge_orbs      crates/horizon-client/src/game.rs
//   Game::tick_experience (the local player's pickup)  crates/horizon-client/src/game.rs
//   Game::guest_orbs (server players, the pickup hook) crates/horizon-client/src/game/lan_actions.rs
//   native_experience_orbs / native_set_experience_orb / native_give_experience  game/native_server.rs
// The hooks are wired in the order the real injections run them. This is NOT the real game:
// it checks this mod's logic against the engine's documented behaviour (see TEST_REPORT.md).

#![allow(dead_code)]

use clumps_horizon::{Host, Orb, Seen};

#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    lo: [f64; 3],
    hi: [f64; 3],
}

impl Aabb {
    fn inflate(self, d: f64) -> Aabb {
        Aabb {
            lo: self.lo.map(|v| v - d),
            hi: self.hi.map(|v| v + d),
        }
    }

    // AABB.intersects: touching faces do not count.
    fn intersects(self, o: Aabb) -> bool {
        (0..3).all(|a| self.lo[a] < o.hi[a] && self.hi[a] > o.lo[a])
    }
}

#[derive(Clone, Debug)]
pub struct SimOrb {
    pub id: u32,
    pub pos: [f64; 3],
    pub value: u32,
    pub count: u32,
    pub age: u32,
    pub ticks: u32,
}

impl SimOrb {
    // ExperienceOrb's box: 0.5 wide and high, standing on its position.
    fn bounds(&self) -> Aabb {
        Aabb {
            lo: [self.pos[0] - 0.25, self.pos[1], self.pos[2] - 0.25],
            hi: [self.pos[0] + 0.25, self.pos[1] + 0.5, self.pos[2] + 0.25],
        }
    }

    // ExperienceOrb.canMerge: the same value, and ids a multiple of 40 apart.
    fn vanilla_can_merge(&self, id: u32, value: u32) -> bool {
        self.value == value && (self.id as i32).wrapping_sub(id as i32) % 40 == 0
    }

    pub fn xp(&self) -> u64 {
        u64::from(self.value) * u64::from(self.count)
    }
}

#[derive(Clone, Debug)]
pub struct SimPlayer {
    pub uuid: u128,
    pub pos: [f64; 3],
    pub xp: u64,
    pub xp_delay: u32,
    pub alive: bool,
    pub takes: u32,
}

pub struct Sim {
    pub dim: String,
    pub orbs: Vec<SimOrb>,
    pub players: Vec<SimPlayer>,
    // The player whose state is swapped in while a pickup hook runs (`native_acting_id`).
    pub acting: Option<u128>,
    // Options > Mods.
    pub enabled: bool,
    // Another mod that cancels every merge / pickup after this one's HEAD ran.
    pub foreign_cancel_merge: bool,
    pub foreign_cancel_pickup: bool,
    // `Util.getRandom(orbs, random)`: which of the touching orbs a server player takes.
    pub pick: fn(usize) -> usize,
    pub next_entity: u32,
    pub removed: Vec<u32>,
    pub gives_fail: bool,
}

impl Sim {
    pub fn new(dim: &str) -> Self {
        Sim {
            dim: dim.to_owned(),
            orbs: Vec::new(),
            players: Vec::new(),
            acting: None,
            enabled: true,
            foreign_cancel_merge: false,
            foreign_cancel_pickup: false,
            pick: |_| 0,
            next_entity: 100,
            removed: Vec::new(),
            gives_fail: false,
        }
    }

    pub fn orb(&mut self, id: u32, x: f64, value: u32) -> &mut Self {
        self.orb_counted(id, x, value, 1)
    }

    pub fn orb_counted(&mut self, id: u32, x: f64, value: u32, count: u32) -> &mut Self {
        self.orbs.push(SimOrb {
            id,
            pos: [x, 0.0, 0.0],
            value,
            count,
            age: 10,
            ticks: 0,
        });
        self
    }

    pub fn player(&mut self, uuid: u128, x: f64) -> &mut Self {
        self.players.push(SimPlayer {
            uuid,
            pos: [x, 0.0, 0.0],
            xp: 0,
            xp_delay: 0,
            alive: true,
            takes: 0,
        });
        self
    }

    // Every orb scans on this tick (`tickCount % 20 == 1`).
    pub fn scan_all(&mut self) {
        for o in &mut self.orbs {
            o.ticks = 1;
        }
        self.merge_orbs();
        for o in &mut self.orbs {
            o.ticks = 0;
        }
    }

    pub fn orb_xp(&self) -> u64 {
        self.orbs.iter().map(SimOrb::xp).sum()
    }

    pub fn player_xp(&self) -> u64 {
        self.players.iter().map(|p| p.xp).sum()
    }

    pub fn total_xp(&self) -> u64 {
        self.orb_xp() + self.player_xp()
    }

    // ---- game.rs ----

    // `orb_merges`: vanilla's answer, the mod's HEAD, the listeners, the mod's RETURN.
    fn orb_merges(&mut self, into: u32, into_value: u32, from: u32, from_value: u32, vanilla: bool) -> bool {
        let _ = (into_value, from_value);
        let can = clumps_horizon::can_merge(&*self, into, from, vanilla);
        if !can {
            return false;
        }
        let mut canceled = self.foreign_cancel_merge;
        let dim = self.dim.clone();
        clumps_horizon::on_merge(self, &dim, into, from, from_value, &mut canceled);
        !canceled
    }

    // `merge_new_orb`: an orb that is spawning.
    pub fn merge_new_orb(&mut self, at: [f64; 3], value: u32, merge_id: i32) -> bool {
        let area = Aabb {
            lo: at.map(|v| v - 0.5),
            hi: at.map(|v| v + 0.5),
        };
        let mut target = None;
        for i in 0..self.orbs.len() {
            let o = &self.orbs[i];
            if !o.bounds().intersects(area) {
                continue;
            }
            let (id, orb_value, vanilla) = (o.id, o.value, o.vanilla_can_merge(merge_id as u32, value));
            if self.orb_merges(id, orb_value, 0, value, vanilla) {
                target = Some(i);
                break;
            }
        }
        if let Some(i) = target {
            self.orbs[i].count += 1;
            self.orbs[i].age = 0;
            true
        } else {
            false
        }
    }

    // `award_orb`: a fresh orb when nothing takes the new one.
    pub fn award_orb(&mut self, at: [f64; 3], value: u32, merge_id: i32) {
        if !self.merge_new_orb(at, value, merge_id) {
            self.next_entity += 1;
            self.orbs.push(SimOrb {
                id: self.next_entity,
                pos: at,
                value,
                count: 1,
                age: 0,
                ticks: 0,
            });
        }
    }

    // `merge_orbs`: ExperienceOrb.scanForMerges.
    pub fn merge_orbs(&mut self) {
        let mut removed = vec![false; self.orbs.len()];
        for i in 0..self.orbs.len() {
            if removed[i] || self.orbs[i].ticks % 20 != 1 {
                continue;
            }
            let area = self.orbs[i].bounds().inflate(0.5);
            for j in 0..self.orbs.len() {
                if j == i || removed[j] || !self.orbs[j].bounds().intersects(area) {
                    continue;
                }
                let (a, b) = (&self.orbs[i], &self.orbs[j]);
                let vanilla = b.vanilla_can_merge(a.id, a.value);
                let (into, into_value, from, from_value) = (a.id, a.value, b.id, b.value);
                if !self.orb_merges(into, into_value, from, from_value, vanilla) {
                    continue;
                }
                let (count, age) = (self.orbs[j].count, self.orbs[j].age);
                self.orbs[i].count = self.orbs[i].count.saturating_add(count);
                self.orbs[i].age = self.orbs[i].age.min(age);
                removed[j] = true;
            }
        }
        let mut gone = removed.iter();
        let mut ids = Vec::new();
        self.orbs.retain(|o| {
            let dead = *gone.next().unwrap_or(&false);
            if dead {
                ids.push(o.id);
            }
            !dead
        });
        self.removed.extend(ids);
    }

    // `tick_experience`: the local (host) player. No pickup hook is asked here.
    pub fn host_tick(&mut self, who: usize) {
        if self.players[who].xp_delay > 0 {
            self.players[who].xp_delay -= 1;
        }
        self.merge_orbs();
        let p = self.players[who].clone();
        let (min, max) = (
            [p.pos[0] - 0.3 - 1.0, p.pos[1] - 0.5, p.pos[2] - 0.3 - 1.0],
            [p.pos[0] + 0.3 + 1.0, p.pos[1] + 1.8 + 0.5, p.pos[2] + 0.3 + 1.0],
        );
        let mut delay = self.players[who].xp_delay;
        let mut gained = 0u64;
        let mut gone = Vec::new();
        self.orbs.retain_mut(|o| {
            let touching = o.pos[0] + 0.25 > min[0]
                && o.pos[0] - 0.25 < max[0]
                && o.pos[1] + 0.5 > min[1]
                && o.pos[1] < max[1]
                && o.pos[2] + 0.25 > min[2]
                && o.pos[2] - 0.25 < max[2];
            if touching && delay == 0 {
                delay = 2;
                gained += u64::from(o.value);
                o.count -= 1;
                if o.count == 0 {
                    gone.push(o.id);
                }
                return o.count > 0;
            }
            true
        });
        self.removed.extend(gone);
        self.players[who].xp_delay = delay;
        self.players[who].xp += gained;
        if gained > 0 {
            self.players[who].takes += 1;
        }
    }

    // `guest_orbs`: a server player (LAN guest, dedicated server).
    pub fn guest_tick(&mut self, who: usize) {
        if self.players[who].xp_delay > 0 {
            self.players[who].xp_delay -= 1;
        }
        let p = self.players[who].clone();
        if !p.alive || p.xp_delay > 0 {
            return;
        }
        let (min, max) = (
            [p.pos[0] - 1.3, p.pos[1] - 0.5, p.pos[2] - 1.3],
            [p.pos[0] + 1.3, p.pos[1] + 1.8 + 0.5, p.pos[2] + 1.3],
        );
        let touching: Vec<usize> = self
            .orbs
            .iter()
            .enumerate()
            .filter(|(_, o)| {
                o.pos[0] + 0.25 > min[0]
                    && o.pos[0] - 0.25 < max[0]
                    && o.pos[1] + 0.5 > min[1]
                    && o.pos[1] < max[1]
                    && o.pos[2] + 0.25 > min[2]
                    && o.pos[2] - 0.25 < max[2]
            })
            .map(|(i, _)| i)
            .collect();
        if touching.is_empty() {
            return;
        }
        let index = touching[(self.pick)(touching.len()) % touching.len()];
        let mut orb = self.orbs[index].clone();
        let mut canceled = false;
        // `native_experience_orb_pickup_listened`: the mod's RETURN sets it while it is on.
        let listened = clumps_horizon::listens(&*self);
        if listened {
            // The hook runs with the player swapped in.
            self.acting = Some(p.uuid);
            // Another mod's listener may cancel; the mod's RETURN sees it.
            canceled = self.foreign_cancel_pickup;
            let (mut value, mut count) = (orb.value, orb.count);
            clumps_horizon::on_pickup(self, p.uuid, &mut value, &mut count, canceled);
            self.acting = None;
            orb.value = value;
            orb.count = count;
            if canceled {
                return;
            }
            orb.value = orb.value.clamp(1, i32::MAX as u32);
            orb.count = orb.count.max(1);
            self.orbs[index].value = orb.value;
            self.orbs[index].count = orb.count;
        }
        if orb.count <= 1 {
            let o = self.orbs.remove(index);
            self.removed.push(o.id);
        } else {
            self.orbs[index].count -= 1;
        }
        self.players[who].xp_delay = 2;
        self.players[who].xp += u64::from(orb.value);
        self.players[who].takes += 1;
    }
}

// ---- native_server.rs, behind the core's `Host` ----

impl Host for Sim {
    fn enabled(&self) -> bool {
        self.enabled
    }

    // `native_experience_orbs`: empty for a level that is not in place.
    fn orbs(&self, dimension: &str) -> Vec<Seen> {
        if dimension != self.dim {
            return Vec::new();
        }
        self.orbs
            .iter()
            .filter(|o| o.count > 0)
            .map(|o| Seen {
                id: o.id,
                orb: Orb::new(o.value, o.count),
            })
            .collect()
    }

    // `native_set_experience_orb`: false for a 0 value or count, or an orb that is not there.
    fn set_orb(&mut self, dimension: &str, id: u32, orb: Orb) -> bool {
        if dimension != self.dim || orb.value == 0 || orb.count == 0 {
            return false;
        }
        let Some(o) = self.orbs.iter_mut().find(|o| o.id == id) else {
            return false;
        };
        o.value = orb.value;
        o.count = orb.count;
        true
    }

    // `native_give_experience`: only the player whose state is swapped in, and only alive.
    fn give_experience(&mut self, player: u128, points: u32) -> bool {
        if self.gives_fail || self.acting != Some(player) {
            return false;
        }
        let Some(p) = self.players.iter_mut().find(|p| p.uuid == player) else {
            return false;
        };
        if !p.alive {
            return false;
        }
        p.xp += u64::from(points);
        true
    }
}
