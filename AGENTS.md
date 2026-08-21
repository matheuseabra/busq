# AGENTS.md

minfetch: a tiny, pane-aware system-info readout (fastfetch alternative) in Rust. Implementation has barely started (`src/main.rs` is a stub); `docs/` is the spec. No CI or release profile exists yet, despite being planned.

## Doc precedence — they conflict

Read order by decision recency: `docs/VISION-REVIEW.md` → `docs/ARCH.md` / `docs/ROADMAP.md` / `docs/COMPLEXITY.md`. The review file is the surviving vision and decision record; where docs disagree, its verdicts win. Known traps if you read only ARCH/ROADMAP:

- Resize behavior: **re-fetch fresh data and re-layout** (H-2). ARCH's "re-layout cached rows, never re-fetch" is overturned.
- Binary size: a **bendable budget measured in CI**, not a hard 1 MB gate (H-11).
- Config: an optional config file is in scope; flags-only was overturned (H-7).
- JSON emission stays feature-gated, out of the default binary (H-1).

## Constraints that change how you code

- Hand-roll `/proc` (Linux) and `sysctl` (macOS) parsing. No sysinfo-class crate by default; a sysinfo-backed path may exist behind a feature flag for exotic systems only.
- Every dependency, flag, and row must earn its binary-size cost; size is checked in CI.
- Icons are ON by default → `unicode-width` correctness is core, not optional.
- One failing fetcher renders `—` in its row; it never aborts other rows or the run.
- Piped/redirected stdout must be plain text: no ANSI, no logo. `NO_COLOR` and explicit color flags always win.
- Non-goals: fastfetch parity, interactive TUI, daemon/watch mode, OS-matched logo library, package counts/GPU charts.

## Build & test

Plain cargo, nothing custom yet:

```sh
cargo build --release
cargo test
```

When implementing Phase 0/1, add what ROADMAP specifies but doesn't exist yet: `[profile.release]` (opt-level z, LTO, codegen-units 1, panic abort, strip) and a CI size-trend check. Don't assume either is already configured.

## Testing conventions (planned, per ROADMAP)

- Fetch parsing: fixtures with mocked `/proc`/sysctl content per supported OS.
- Layout: golden snapshot tests at fixed widths (~120/60/30 cols).
- Always test the non-TTY path explicitly; emitting ANSI into a pipe is treated as a real bug.

## Repo hygiene

`.commandcode/` and `logs/` are local tool artifacts, not project code — leave them alone and don't commit them.
