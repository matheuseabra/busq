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

## Phase 0 — Skeleton & build health (locally complete)

**Goal:** a building, testable, near-empty binary with CI measuring its size budget.

Tasks:
- [x] `cargo init` with package name `minfetch`, edition 2021+, and a dependency-light baseline.
- [x] Add release profile per ARCH (opt-level z, lto, codegen-units 1, panic abort, strip).
- [x] `main.rs` prints `"minfetch 0.1.0"` and exits 0.
- [x] Add a `cargo size` helper (a make/just target or a script) that reports the binary size in CI
      as a trend metric and fails only on an agreed budget overrun; the budget may bend for earned
      features.
- [x] CI: build + test + size gate on `macos-latest` and `ubuntu-latest` in the same matrix. Fail
      the job if size exceeds the gate.
- [x] A trivial unit test (`fn test_version_string_contains_version`) to prove the test harness runs.

**Phase 0 gate:**
- [x] `cargo build --release && cargo test --release` passes locally.
- [x] CI records the release binary size against the current budget on both primary OSes.

---

## Phase 1 — Flat, single-run fetch & render (v0.1)

**Goal:** minfetch fetches the core info, prints it in a simple k/v list, honors `--color-no` /
`NO_COLOR`, and handles the non-TTY path. ASCII logos can **stack** (above the list) in this phase;
side-by-side layout lands in phase 2.

Tasks:
- [x] CLI: `--color-no`, `--version`, `--help`, and `--icons on|off` to control the Unicode glyph
      labels (icons **on by default** — review verdict).
- [x] Fetch hostname, user, OS, and shell.
- [x] Fetch CPU model + core count, Linux `/proc` or macOS `sysctl`/`vm_stat` memory, root disk,
      uptime, and context rows.
- [ ] Fetch CPU temperature and GPU identity; failures render as `—` like every other row.
- [x] Plain k/v rendering; `—` for unavailable rows; no ANSI output.
- [x] Piped output is plain text and suppresses the logo.
- [x] Optional `--logo PATH` loads an ASCII logo; Phase 2A controls stacking or side-by-side placement.
- [x] Unit fixtures for Linux-style CPU and memory parser input.
- [ ] Platform fixtures for macOS `sysctl` and remaining fetch paths.

**Phase 1 gate:**
- [x] `cargo test --release` green locally.
- [x] `minfetch | cat` (piped) yields plain, color-free output that renders correctly in a
      documentation file / text editor.
- [x] `minfetch --color-no` emits no ANSI escape sequences.
- [ ] Binary size and manual pane fit verified on both primary OSes.

---

## Phase 2A — Pure pane-aware rendering (v0.2)

**Goal:** a pure, width/height-driven layout: side-by-side logo + info when wide, stacked /
single-column when narrow; long values truncate; `--logo` is positioned correctly.

Tasks:
- [x] Implement the minimal width-budget render model.
- [x] Side-by-side layout when `pane_width >= logo_width + info_min_width + gap`.
- [x] Stacked layout and single-column (labels-on-own-line, icons dropped) fallbacks.
- [x] Ellipsis truncation for values exceeding the pane width with `unicode-width`.
- [x] `--logo` respects the re-flow rules; logo dropped when pane too narrow for any horizontal
      layout *and* stacked would overflow height.
- [x] `unicode-width` handled as **core** (not optional) in the renderer, since icons are on by
      default — a mis-wide glyph must never misalign the info column.
- [x] Height cap: rows beyond terminal height are omitted, not scrolled.

**Phase 2 gate:**
- [x] Fixed-width tests cover side-by-side, stacked, single-column, truncation, Unicode width,
      and height cap.
- [ ] Manual verification in a real terminal: resize a pane to 120/60/30 wide and 8/12/20 tall;
      output never wraps a row or scrolls beyond the pane. A 30x8 PTY smoke passed; the full
      width/height matrix remains pending.
- [x] A 30-column run shows the single-column k/v fallback with correct alignment.
- [x] `cargo test --release` green locally; size gate remains to verify.

## Phase 2B — Terminal integration

**Goal:** use real terminal dimensions and re-fetch fresh data on resize without
turning minfetch into a resident/watch process.

Tasks:
- [x] Read width/height with `ioctl(TIOCGWINSZ)` on Unix, retaining the fixed
      fallback for pipes and test environments.
- [x] Observe `SIGWINCH` only during the single fetch/render snapshot; if it
      arrives, re-read dimensions and re-fetch once, then exit normally.
- [x] Add an integration test for the non-TTY fallback and fixed-width checks.

Decision note: resize handling is deliberately bounded to the active snapshot.
The command never waits for SIGWINCH, so it remains single-shot rather than
becoming a resident/watch process.

**Phase 2B gate:**

- [x] No output row wraps or exceeds the detected width in fixed-width and PTY
      smoke checks.
- [x] A SIGWINCH observed during collection produces fresh values and never
      uses cached rows; later resizes require a future resident mode, which is
      out of scope.
- [x] `cargo test --locked --release` and Clippy pass locally.

---

## Phase 3 — Polish, flags, reliability (v0.3, started)

**Goal:** battery of user-facing flags, a built-in ASCII logo with casing, robustness against
missing/exotic systems, and packaging for a widget use case.

Tasks:
- [x] Built-in default neutral logo (three small lines), omitted from piped output.
- [x] Flags: `--logo none|auto|path` and `--icons on|off`.
- [x] `--no-terminator` omits the trailing newline for widget use.
- [x] `--color <auto|always|never>` colors labels on a TTY while keeping piped output plain.
- [x] `--no-term` omits uptime when not wanted.
- [x] Fetch errors are structured: a single failure never aborts the whole run; `--verbose` prints
      the underlying error line in a tooltip style on stderr.
- [x] Optional config file for default rows, logo, icons, and theme; flags override it.
- [x] Small preset theme set around the subtle default (`subtle` and `mono`).
- [x] Feature-gated JSON emitter, excluded from the default binary.
- [ ] Optional feature-gated sysinfo-backed fetcher for exotic systems; hand-rolled parsing remains
      the default.
- [x] Windows: `--no-icons`/plain path at minimum; a `--no-color` path that works on all three
      platforms. (Full Windows support may move to Phase 4.)
- [x] CI adds a third job for Windows (build-only, tests conditional where possible).
- [x] Fuzz/edge tests: empty `$SHELL`, missing `/proc`, non-UTF8 env, unicode in user/hostname.

**Phase 3 gate:**
- [x] `--help` and `--version` are accurate and consistent.
- [x] No-icons, ANSI, piping, and `NO_COLOR` are covered by the macOS/Linux CI matrix and a local
      PTY check.
- [x] Error-injection test suite: force each fetcher to fail (fixtures) and assert output still
      renders.
- [x] Size trend remains within the earned feature budget after all flags + logo.

---

## Phase 4 — Distribution & hardening (v1.0, started)

**Goal:** a dependable, distributable `v1.0`.

Tasks:
- [x] Release binary builds for macOS (arm64 + x86_64), Linux (x86_64 + aarch64) via CI artifacts.
- [x] Reproducible builds enforced in CI with a fixed `SOURCE_DATE_EPOCH` and byte-identical
      release binaries.
- [ ] Homebrew formula (`brew tap` or upstream formula).
- [x] `cargo install --path .` documented for local installation.
- [x] A curated `docs/` reference covers usage, configuration/flags, and layout behavior.
- [x] No silently unsupported platform surfacing regressions: `--probe`/`--debug-sysinfo` dumps
      what was detected to help users file good issues.
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
| 0.2.0-a | 2A | pure 3-mode layout + Unicode-width tests |
| 0.2.0-b | 2B | real terminal dimensions + fresh-data resize behavior |
| 0.3.0 | 3 | full flags, icons, logo, error-injection suite |
| 1.0.0 | 4 | multi-target release artifacts, packaging, no blockers |
