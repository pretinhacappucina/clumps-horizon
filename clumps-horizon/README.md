# clumps-horizon

Native Rust port of [Clumps](https://github.com/jaredlll08/Clumps) (branch 26.3, MIT) for Horizon, as a `horizon-mixins` source mod.

## Features
- **Merging:** any two live XP orbs whose boxes meet merge, whatever their value (Clumps' `canMerge` replacement). The keeper's value/count are written so the merged orb is worth exactly the sum (no XP lost or duplicated).
- **Immediate pickup:** a server player touching an orb receives the whole clump in one touch (Mending sees each count, as in Clumps).
- Runs on the Horizon server/LAN-host side through the `experience_orb_can_merge`, `experience_orb_merge`, `experience_orb_pickup_listened` and `experience_orb_pickup` hooks (all hook_version 1).
- Toggle in Options > Mods (`native_on("clumps")`). Off = vanilla behaviour, no cost.

## Compatibility
Pinned to Horizon Nightly commit `fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca` (2026-11-08) and horizon-mixins `e8cd0aa`. See `compatibility.json`. Other revisions need the pin updated and re-validated.

## Build / install
Use the template workflow: `python3 tools/horizon.py` (see `tools/`), which stages the pinned Horizon sources, applies `mixins/clumps` and builds a native Horizon. Packaging into `.hnmod` is done by `tools/package.py`. **No `.hnmod` is shipped here: see TEST_REPORT.md.**

## Known limitations
- Clumps has no config file in the 26.3 branch beyond the mod itself; none was invented. Horizon exposes no config API used here besides the on/off toggle.
- Horizon adds the absorbed orb's count after the merge hook, so a merged orb is stored as `value x count`, not Clumps' single big value. Totals with no valid divisor within the count cap (32, except same-value stacking) are **refused** and the orbs stay separate.
- The local (host) player's pickup does not go through the pickup hook in Horizon, so it takes one count per touch (about 2 ticks apart) instead of the whole clump at once. Server/LAN guests get the full instant pickup.
- Merge reach is vanilla's (Horizon only asks about orbs whose boxes meet); Clumps adds no extra radius that this port could verify, so none was added.
