# minfetch — Roadmap

Quality-gated, phaseless-in-spirit: each phase has explicit **quality gates** that must pass before
the next phase starts. Milestones are written as trackable tasks with `[ ]` checkboxes; a milestone
is "done" only when its gate is green.

## How to read this document

- Each phase bundles a set of task checklists and one or more **gates**.
- A gate is a concrete, measurable acceptance criterion. It is **not** "merge a PR" — it's a bar we
  can verify with a command, a test, or a manual repro.
- Phases are ordered by dependency, not by "importance." Phase 0 is the prerequisite for everything.

---

## Phase 0 — Skeleton & build health

**Goal:** a building, testable, near-empty binary with CI measuring its size budget.

Tasks:
- [ ] `cargo init` with package name `minfetch`, edition 2021+, no external deps yet.
- [ ] Add release profile per ARCH (opt-level z, lto, codegen-units 1, panic abort, strip).
- [ ] `main.rs` prints `"minfetch 0.1.0"` and exits 0.
- [ ] Add a `cargo size` helper (a make/just target or a script) that reports the binary size in CI
      as a trend metric and fails only on an agreed budget overrun; the budget may bend for earned
      features.
- [ ] CI: build + test + size gate on `macos-latest` and `ubuntu-latest` in the same matrix. Fail
      the job if size exceeds the gate.
- [ ] A trivial unit test (`fn test_version_string_contains_version`) to prove the test harness runs.

**Phase 0 gate:**
- [ ] `cargo build --release && cargo test --release` passes on macOS and Linux.
- [ ] CI records the release binary size against the current budget.

---

## Phase 1 — Flat, single-run fetch & render (v0.1)

**Goal:** minfetch fetches the core info, prints it in a simple k/v list, honors `--color-no` /
`NO_COLOR`, and handles the non-TTY path. ASCII logos can **stack** (above the list) in this phase;
side-by-side layout lands in phase 2.

Tasks:
- [ ] `cli.rs`: `--color-no`, `--version`, `--help`, and `--icons off` to disable the unicode glyph
      labels (icons **on by default** — grill-me resolution) (clap minimal flags).
- [ ] `fetch/base.rs`: hostname, user, OS (`os-release`/`release`), shell (`$SHELL`).
- [ ] `fetch/cpu.rs`: CPU model + core count (macOS `sysctl`, Linux `/proc/cpuinfo`).
- [ ] `fetch/mem.rs`: total + used memory.
- [ ] `fetch/disk.rs`: root filesystem used/total.
- [ ] `fetch/term.rs`: uptime (cheap; from `/proc` or `sysctl`).
- [ ] Fetch context rows by default: kernel, terminal, and desktop/window manager.
- [ ] Fetch CPU temperature and GPU identity; failures render as `—` like every other row.
- [ ] `render/`: plain k/v lines; `—` for a failed fetcher; no width math yet.
- [ ] TTY detection: if stdout is piped/redirected, print plain text with no ANSI and no color.
- [ ] OPTIONAL `--logo pathto.txt` that reads an ASCII file and stacks it above the list.
- [ ] Unit + integration tests for the fetchers (mock `/proc` content in tests, and use a fixture
      `sysctl`-free path on macOS).

**Phase 1 gate:**
- [ ] `cargo test --release` green across CI matrix (macOS + Linux).
- [ ] `minfetch | cat` (piped) yields plain, color-free output that renders correctly in a
      documentation file / text editor.
- [ ] `minfetch --color-no` shows no ANSI escape sequences (verified via `| xxd | grep -c '1b'`).
- [ ] Binary size remains within the measured budget with real fetch code inlined.
- [ ] Manual smoke: run it in a 12-line-high pane; it fits without scrolling.

---

## Phase 2 — Pane-aware rendering (v0.2)

**Goal:** responsive layout: side-by-side logo + info when wide, stacked / single-column when
narrow; long values truncate; `--logo` positioned correctly.

Tasks:
- [ ] Implement the cell-grid render model (ARCH) — width budget and layout rules.
- [ ] Side-by-side layout when `pane_width >= logo_width + info_min_width + gap`.
- [ ] Stacked layout and single-column (labels-on-own-line, icons dropped) fallbacks.
- [ ] Ellipsis truncation for values exceeding the pane width (unicode-width aware).
- [ ] `--logo` respects the re-flow rules; logo dropped when pane too narrow for any horizontal
      layout *and* stacked would overflow height.
- [ ] Pane-resize re-render: on `SIGWINCH`, re-read `TIOCGWINSZ`, re-fetch fresh rows, re-layout,
      and re-print. This remains single-shot resize handling, not watch mode.
      (Requires the small `signal_hook` dep or a `nix` signal handler.)
- [ ] `unicode-width` handled as **core** (not optional) in the renderer, since icons are on by
      default — a mis-wide glyph must never misalign the info column.
- [ ] Height cap: rows beyond terminal height are omitted, not scrolled.

**Phase 2 gate:**
- [ ] Golden / snapshot tests for the three layout modes at fixed widths (e.g. 120, 60, 30 cols).
- [ ] Manual verification in a real terminal: resize a pane to 120/60/30 wide and 8/12/20 tall;
      output never wraps a row or scrolls beyond the pane.
- [ ] A 30-column run shows the single-column k/v fallback with correct alignment.
- [ ] `cargo test --release` green; size gate holds.

---

## Phase 3 — Polish, flags, reliability (v0.3)

**Goal:** battery of user-facing flags, a built-in ASCII logo with casing, robustness against
missing/exotic systems, and packaging for a widget use case.

Tasks:
- [ ] Built-in default logo (small; a few lines) — pick a neutral branch/Tux/Apple mark, keep it
      tiny. Ship as a string.
- [ ] Flags: `--logo none|auto|path`, `--icons on|off`, `--no-terminator` (no trailing newline for
      widget use), and `--color <auto|always|never>`.
- [ ] `term.rs` -> `--no-term` flag to omit uptime if not wanted.
- [ ] Fetch errors are structured: a single failure never aborts the whole run; a `--verbose` flag
      prints the underlying error line in a tooltip style.
- [ ] Optional config file for default rows, logo, icons, and theme; flags override it.
- [ ] Small preset theme set around the subtle default.
- [ ] Feature-gated JSON emitter, excluded from the default binary.
- [ ] Optional feature-gated sysinfo-backed fetcher for exotic systems; hand-rolled parsing remains
      the default.
- [ ] Windows: `--no-icons`/plain path at minimum; a `--no-color` path that works on all three
      platforms. (Full Windows support may move to Phase 4.)
- [ ] CI adds a third job for Windows (build-only, tests conditional where possible).
- [ ] Fuzz/edge tests: empty `$SHELL`, missing `/proc`, non-UTF8 env, unicode in user/hostname.

**Phase 3 gate:**
- [ ] `--help` and `--version` are accurate and consistent.
- [ ] No-ICO, ANSI, piping, and `NO_COLOR` all behave correctly across macOS + Linux in CI + manual
      checks.
- [ ] Error-injection test suite: force each fetcher to fail (fixtures) and assert output still
      renders.
- [ ] Size trend remains within the earned feature budget after all flags + logo.

---

## Phase 4 — Distribution & hardening (v1.0)

**Goal:** a dependable, distributable `v1.0`.

Tasks:
- [ ] Release binary builds for macOS (arm64 + x86_64), Linux (x86_64 + aarch64) via CI artifacts.
- [ ] Reproducible builds (deterministic; document or enforce e.g. `SOURCE_DATE_EPOCH`).
- [ ] Homebrew formula (`brew tap` or upstream formula) + a `cargo install` line in README.
- [ ] A curated `docs/` set: usage, configuration/flag reference, layout behavior.
- [ ] No silently unsupported platform surfacing regressions: add a `--probe`/`--debug-sysinfo`
      flag that dumps what was detected (helps users file good issues).
- [ ] 30-day "no open launch-blockers" criterion: any bug tagged `blocker` must be fixed or
      explicitly deferred with a reason before release.

**Phase 4 gate:**
- [ ] CI builds green and artifact size remains within the measured budget for all target triples.
- [ ] `cargo install minfetch` and the Homebrew formula both work in a clean env.
- [ ] README "usage in a tmux/zsh pane" quickstart verified end-to-end.
- [ ] No open `blocker`-tagged issues.

---

## Non-goals (explicitly out of scope for v1)

- Full `fastfetch` parity (package counts, network info, theme detection, GPU charts).
- A TUI with interactive input / scrolling.
- A daemon/watch mode with continuous sampling.
- GPU load, temperature, or power charts.

---

## Milestone summary

| Milestone | Phase | Gate (what "done" means) |
|-----------|-------|--------------------------|
| 0.1.0 skeleton | 0 | builds + tests + measured size budget on macOS/Linux |
| 0.1.0 | 1 | flat list, no-color, non-TTY path all verified |
| 0.2.0 | 2 | 3 layout modes + resize under manual + snapshot tests |
| 0.3.0 | 3 | full flags, icons, logo, error-injection suite |
| 1.0.0 | 4 | multi-target release artifacts, packaging, no blockers |
