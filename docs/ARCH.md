# minfetch — Architecture

**minfetch** is a tiny, statically-linked alternative to `fastfetch` that fetches only the most
essential system info, can render a small ASCII-art logo beside the output, and is responsive to
terminal (pane) resizes.

## Guiding principles

1. **Tiny binary.** The whole point is being a small pane widget. Binary size and memory footprint
   are design constraints, not afterthoughts.
2. **Fast.** Fetch and print must complete in milliseconds. It is not a diagnostics suite.
3. **Pane-aware output.** The layout adapts to the current terminal width/height, so it stays
   readable as a widget, not a full-screen report.
4. **No ceremony is its own feature.** Fraught with the "works on my machine" risk, so build a CI
   matrix of target platforms and test against real terminals.

## Recommended stack

### Language: Rust

Primary: **Rust** with a careful `Cargo.toml` diet (see *dependencies*).

Rationale:

- Produces a genuine single static binary, small when built with `opt-level = "z"`, LTO, and
  `strip = true`. This is the clearest win over Go (Go runtime floor ~1.5 MB) and any JVM/Node/Python
  (not viable for a "tiny" widget).
- First-class access to termios/TTY via `libc` / `nix`, and a large ecosystem of TTY and sysinfo
  crates.
- `std::fs`, `std::process`, and `std::env` cover most fetches with zero dependencies.
- Cross-compilation for macOS, Linux, and Windows is well supported.

Not selected: **Go** (simpler concurrency but larger binaries and heavier TTY ecosystem), **C**
(no memory safety, slow to build features), **Zig** (viable but smaller ecosystem for sysinfo).

> Decision point: the **language is the one change that is extremely expensive to make later.**
> Everything else in this document assumes Rust. If you disagree, say so now — the rest of the
> plan (ROADMAP, complexity) depends on it.

### Dependencies (deliberately minimal)

Goal: keep the dependency count low, but not for its own sake — only drop a dep where hand-rolling
is trivial and safe.

Planned dependencies:

| Crate | Where | Why |
|-------|-------|-----|
| `clap` | CLI parsing | Flags like `--logo`, `--color-no`, `--no-icons`. Alternatives: hand-rolled arg parse (fine for 3-4 flags; keep in triage review). |
| `nix` (termios) OR `libc` (single feature) | TTY detection & resize | Raw mode, `ioctl(TIOCGWINSZ)` for width/height. Prefer `nix` with a narrow feature set. |
| *(none by default)* | CPU/mem/disk | **Hand-rolled** `/proc` (Linux) + `sysctl` (macOS) parsing. An optional feature-gated sysinfo-backed path covers exotic systems. |
| `colored` OR `nu-ansi-term` | ANSI colors | `colored` is tiny and zero-dep. |
| `unicode-width` | Text alignment | Correct width of East Asian / emoji glyphs for pane-aware wrapping. |
| `owo-colors` (optional) | Always-color output | Only if we do text-emphasis beyond plain ANSI. |

Runtime dependencies intentionally excluded:

- No async runtime (no `tokio`/`async-std`). Everything here is synchronous I/O that returns in
  microseconds.
- No serde/JSON in the default binary; feature-gated JSON output may add it when enabled.
- No DBus, no GUI toolkits. Failures are reported as plain missing-value rows, not errors.

Optional feature flags:

- `json-output` (serde/serde_json) — optional feature for `--json` output.
- Custom logo files (`--logo path/to/art.txt`) reads a text file at runtime — standard lib only.

### Build profile (release)

```toml
[profile.release]
opt-level = "z"      # prefer size
lto = true           # whole-program optimization
codegen-units = 1
panic = "abort"      # smaller binary; we have no long-lived state to corrupt
strip = true
```

Target binary size is a measured, bendable budget on Linux/macOS; the CI trend check catches
unearned growth without imposing a permanent hard 1 MB cutoff.

## Architecture overview

### Module layout

```
src/
  main.rs          # entry; parse args, orchestrate the pipeline
  cli.rs           # flag definitions + parsing (clap)
  fetch/
    mod.rs         # trait/type for a fetched "info row"; each impl is independent
    base.rs        # hostname, user, OS, shell (all std-lib)
    cpu.rs         # CPU model + core count
    mem.rs         # memory usage (sysctl /proc hand-rolled)
    disk.rs        # root disk usage
    term.rs        # uptime, terminal name, kernel, desktop/WM
  render/
    mod.rs         # takes InfoRows + logo, produces a layout
    layout.rs      # column math: logo | gaps | info rows, respecting terminal width
    color.rs       # ANSI color helpers
  tty.rs           # TIOCGWINSZ handling, color support detection
```

### The fetch pipeline

1. `cli.rs` parses flags → `Config` (strictly additive; no hidden defaults).
2. `tty.rs` reads terminal size. **If `stdout` is not a TTY** (piped), it does a fallback: a
   conservative fixed width (e.g. 80 columns) and no ANSI colors. Test this path explicitly — piping
   to a file must produce plain output.
3. Each fetcher runs independently and returns a `Result<Row, FetchErr>`; **one fetcher failing must
   never block the others**. Failed rows render as `—` (em dash) or are omitted.
4. `render/layout.rs` assembles rows and the optional ASCII logo into a single framebuffer (an
   in-memory grid of cells), then emits them once per frame.
5. **Responsiveness:** the renderer re-reads terminal size and rebuilds the framebuffer **every
   print**. On resize, minfetch re-fetches fresh data and re-renders rather than using stale rows.

### Rendering model: a cell grid

The heart of responsiveness is a simple `<Cell>` grid:

- Grid width = terminal width; height = `min(rows_needed, terminal_height)`.
- Each row is a sequence of cells; ANSI colors are attached to runs, not per-cell, to keep the
  output buffer small.
- The **logo and the info columns are laid out side by side** when width allows; when the pane is
  narrow, they stack vertically or the logo is dropped. Rules:
  - Width `>= logo_width + info_min_width + gap` → side-by-side.
  - Else → stack vertically (logo above info).
  - If width `<= info_min_width` → strip to a single-column k/v list (labels and values on their
    own lines), dropping icons.
- Long paths/values are **truncated with an ellipsis** rather than wrapped, so a row never exceeds
  the pane width.
- **Icons (unicode glyphs) are on by default** (`--icons off` to disable) — confirmed in the
  /grill-me pass. This makes `unicode-width` correct handling **core**, not optional: invalid-width
  icons would break the whole column alignment.

## Terminal responsiveness

- Detect width/height via `ioctl(TIOCGWINSZ)`, cached per frame.
- Color detection: `TERM`/`NO_COLOR`/`CLICOLOR_FORCE` conventions. If `NO_COLOR` or stdout is
  piped, disable ANSI.
- `SIGWINCH` handling (optional, roadmap phase 3) uses `signal_hook` or a raw `poll()` — the
  tiniest approach is `nix::sys::signal` + reading `TIOCGWINSZ` on the signal in the main loop.
  Since dependencies are already lean, prefer the smallest option (`signal_hook` is a single small
  crate).
- **Re-print semantic:** on `SIGWINCH` minfetch re-fetches sources, re-lays out fresh rows against
  the new width, and re-prints. It remains a single-shot tool; resize handling is not watch mode.

## Cross-platform

| Platform | Status | Sources |
|----------|--------|---------|
| macOS | Primary | `sysctl`/`sysctlByName` for CPU, `vm_stat` parse or `sysctl` for mem, `statfs` for disk, shell from `$SHELL` |
| Linux | Primary | `/proc/cpuinfo`, `/proc/meminfo`, `/proc/version`, `statfs` |
| BSD/other Unix | Best-effort | `sysctl` fallbacks |
| Windows | Stretch goal | `GetSystemInfo`, may swap in a small helper crate or vendor a shim |

The CI matrix (ROADMAP) reflects this ordering.

## People-facing decisions vs. hard constraints

- **Branding/labels are the easy part** — LCD-casing the label strings and any logo is trivial
  later. Layout and binary size are the hard constraints.
- **Logo behavior (side/strip/unset) is constrained by rendering, not by content.** Decide the logo
  policy once near the end of phase 1; it's cheap to flip, cheap to redo.

## Validation gate

A pristine `Cargo.toml` verified to build with the release profile and pass a `target-size` check of
The release binary has a measured, bendable size budget. See ROADMAP for the CI trend check and
quality gates; useful features may earn budget beyond the initial target.
