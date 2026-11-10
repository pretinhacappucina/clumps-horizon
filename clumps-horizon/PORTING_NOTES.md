# Porting notes

Source of truth: Clumps branch `26.3` (shallow clone, commit 43dfc0c); Horizon at the pinned commit in `compatibility.json`.

| Clumps (Java, mixin on ExperienceOrb) | clumps-horizon (Rust) |
|---|---|
| `canMerge` replaced: other alive and not same orb | `runtime.rs::can_merge` on hook `experience_orb_can_merge` |
| `merge` / absorb into existing, XP summed | `plan_merge` + `on_merge` on hook `experience_orb_merge` (RETURN, sees other mods' cancel) |
| `playerTouch` gives whole clump | `on_pickup`: extra counts via `native_give_experience`, engine takes the last |
| Platform glue (Fabric/NeoForge) | none needed; Horizon hooks are platform-independent |

Differences: Horizon stores orbs as `value x count` and adds counts after the merge hook, hence the divisor search in `plan_merge`. Exact XP is preserved or the merge is refused.
Layout: `runtime.rs` (pure Rust core, also the injected module) / `glue.rs` (Horizon `Game` adapter) / `*.rs` helpers + `horizon-mixin.toml`.
Note: the brief says target 26.3.2; the branch's own version field reads 26.3.
