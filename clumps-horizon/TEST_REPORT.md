# Test report

Environment: cloud Linux container, Rust stable (not the pinned 1.95.0: static.rust-lang.org was unreachable), cargo, git, network via allowlisted proxy.

## Unit tests (executed, 9/9 passed)
`cargo test --lib`: plan_merge arithmetic (sums exact, divisor choice, refusals, clamp, same-value stacking, exhaustive small-range conservation).

## Integration tests (executed, 29/29 passed)
`cargo test --test clumps` drives the mod's real hook functions against a **Rust model** of Horizon's orb engine (`tests/common/mod.rs`, ported by reading game.rs, lan_actions.rs, native_server.rs at the pinned commit). Covers brief items 1-13, including a 300-seed randomized XP-conservation run. **It is a model, not the real game**: engine behavior is only as faithful as that port.

## Build / validation (executed)
- `horizon-mixins plan` on the pristine pinned Horizon tree: status ok (4 injections, 2 modules).
- `horizon-mixins apply` + `cargo check -p horizon-client` on the staged tree: **compiled, no warnings from this mod**. (Needed `libasound2-dev`, `libudev-dev`.)
- `tools/package.py` (template's official packager): produced `clumps-26.3.0.hnmod` (source-only bundle: manifest + mixin sources).

## Not executed
- Running the mod inside a live Horizon client/server (no GUI/game runtime here): real merge/pickup behavior, dedicated server and LAN behavior are **unverified**.
- `horizon-mixins check` (`cargo test` on the staged Horizon tree) and clippy/fmt with the pinned 1.95.0 toolchain.
- The GitHub Actions workflow has **not been run**.
- Issues #4953 / #6452 were not checked (not reached).

## Known issues
See README "Known limitations" (local-player pickup is per-count; some totals are refused).
