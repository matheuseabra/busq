# minfetch — Complexity Assessment

An honest, rough estimate of how much engineering effort minfetch involves, where the risk lives,
and why the "tiny tool" framing both helps and hurts.

## Overall verdict

**Moderate, small-to-medium project — a genuine ~1–2 week part-time effort from scratch to v1, or
~3–5 focused days of heads-down work.** The core is small (a few hundred lines of Rust). The real
work is not the fetch logic — that's scattered but simple. Three factors inflate it: cross-platform
system probing, terminal responsiveness, and release/distribution hardening.

Effort estimate (using rough, hour-based buckets; ranges reflect cross-platform uncertainty):

| Area | Effort estimate | Risk |
|------|-----------------|------|
| Fetch core (base, CPU, mem, disk) | Low | Low comprehension, **medium** per-OS fragility |
| Pane-aware renderer | Medium | Medium — unicode/width edge cases |
| TTY + resize handling | Low–Medium | Low for v1, medium for SIGWINCH/widget |
| CLI + flags + icons/logo | Low | Low |
| Cross-platform (win/mac/linux) | Medium–High | **High** — this is where the time goes |
| Test suite + CI matrix | Medium | Medium (golden tests worth it) |
| Distribution (brew, artifacts, repro builds) | Medium | Low engineering, some ops |

**Bottom line:** if targeting just macOS + Linux with a no-color, no-logo v0 is the goal, this is
a one-to-two-day project. The jump to v1 (full flags + packaging + hardening) is where complexity
triples.

## Where the real complexity lives

### 1. Cross-platform system probing (the dominant cost)

Fetching "CPU model", "memory used", "disk usage" is trivial on one OS and subtly wrong on another.

- **Linux:** `/proc/cpuinfo`, `/proc/meminfo`, `statfs` — cheap and usually reliable, but every
  distro does something slightly different. IRQ-per-cpu, cgroup memory limits, different meminfo
  lines across kernel versions.
- **macOS:** `sysctl` for CPU model, `vm_stat`/`sysctl hw.memsize` for memory; hostname/shell are
  env-based. Tolerable, but `sysctl` keys differ between x86_64 and arm64 in edge cases.
- **Windows:** completely different APIs (`GetSystemInfo`, `GlobalMemoryStatusEx`). Either vendor a
  helper crate (heavier) or accept a reduced feature set.

**Risk multiplier:** the failure modes are *silent* — a "wrong-looking" memory number is worse than
an error. This justifies the `--probe`/`--debug-sysinfo` flag in Phase 4.

### 2. Terminal responsiveness (surprisingly for a "tiny" tool)

"Responsive to pane resizes" sounds trivial and is genuinely subtle:

- Terminal width/height come from one `ioctl(TIOCGWINSZ)`, but **when** you re-read it (SIGWINCH vs.
  per-frame) and how you avoid flickering on resize are design choices.
- **Unicode width** is the wolf in sheep's clothing. A path containing East-Asian glyphs or a
  combining char can be 1 or 2 columns wide, and naive `chars().count()` misaligns the whole column.
  The `unicode-width` crate is the cheap fix; hand-rolling it is a rabbit hole.
- **No-TTY path** must be explicit and *tested*. A tool that prints ANSI garbage when piped into a
  file is a silent bug users hit constantly.

**Takeaway:** schedule a dedicated test for each of: wide pane, narrow pane, very-narrow pane,
zero-height, piped, `NO_COLOR`, and unicode-in-values.

### 3. Size discipline

Staying "tiny" is a *constraint that fights the tool's own feature growth*. Every flag, every logo,
every dependency is a few more KB. Per the review verdicts we **hand-roll `/proc` + `sysctl` parsing
by default**, with an optional feature-gated sysinfo-backed path for exotic systems. The size budget
is bendable but measured in CI, so flag, logo, and dependency accrual remain visible through
`opt-level = "z"`, LTO, `panic = "abort"`, and `strip`.

This is a project-management risk more than a technical one: the tool is simple enough that scope
creep (fastfetch parity) is the biggest threat to the "tiny" identity.

### 4. The "works on my machine" trap

A sysinfo tool is only as trustworthy as the *matrix of machines it's survived*. The CI matrix
(macOS + Linux now, Windows later, arm64 later) is not gravy — it is the primary quality mechanism
for a project whose bugs are silently-wrong numbers.

## Complexity by module (for task gating)

| Module | Lines (estimate) | Cyclomatic hotspots |
|--------|------------------|---------------------|
| `fetch/*` | ~150 | per-OS conditionals; error recovery per fetcher |
| `render/layout.rs` | ~120 | width-budget rules; 3-mode switch |
| `tty.rs` | ~60 | signal handling, no-TTY fallback |
| `cli.rs` | ~50 | flag matrix (incl. `--icons off`), help text |
| `main.rs` | ~30 | pipeline orchestration |
| tests | ~200 | fixtures for each OS's fetch strings; golden layout snapshots |

Estimated total code: **~600–700 lines of non-test Rust, ~200 lines of tests.** That's the *small*
claim — the complexity is spread thin across OS-specific code paths and terminal edge cases, not
concentrated in any one file.

## What would make this dramatically simpler or harder

**Simpler:**
- Drop Windows entirely → entire module area disappears; size and layout work stay identical.
- Single OS (macOS only) → fetch layer collapses to a handful of `sysctl`s; effort drops ~40%.
- No logo at all → renderer has no horizontal layout branch; effort drops ~15%.

**Harder:**
- Full fastfetch-style vertical integration (GPU charts, full themes, network) → multiplies the
  fetch layer and every cross-platform test; not the identity of this tool.
- A live-refreshing widget loop (re-sample on a timer) → adds a threading/async dimension we would
  otherwise avoid for v1.

## Recommended posture

1. **Do not** chase fastfetch parity — set `non-goals` in stone in the ROADMAP.
2. **Do** build macOS + Linux first (they cover the "pane widget" use case) and treat Windows as a
   Phase 4 best-effort.
3. **Do** invest early in the tiny CI matrix — it is the cheapest insurance against the silently-
   wrong-number failure mode.
4. **Do** gate new flags on the size budget; it is the concrete mechanism that preserves the
   product's identity.

This is a *good* project size: substantial enough to reward good structure and testing, small
enough to finish. The risk profile is dominated by cross-platform correctness and scope control,
not by algorithmic difficulty.
