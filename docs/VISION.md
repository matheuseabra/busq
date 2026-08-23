# minfetch vision and review verdicts

This is the single product vision and decision record. The round-1 verdicts
are folded into the direction; companion docs must follow them.

## Vision

minfetch gives a terminal user the shortest truthful summary of a machine that
fits in a small pane. It is single-shot and resize-aware, with identity,
hardware, temperature, GPU, and context rows; plain-text, config, and
feature-gated scripted output are supported.

Hand-rolled `/proc` and `sysctl` parsing is the default. A feature-gated
sysinfo-backed path may cover exotic systems. Dependencies and features stay
within a measured, bendable size budget reported by CI.

Icons and the logo are opt-in, and a small set of themes sits around the subtle default. Resize
re-fetches and re-lays out fresh data. A
failed fetcher renders `—` without aborting other rows. Piped output has no
ANSI or logo; `NO_COLOR` and explicit color flags win.

An optional config file sets default rows, logo, icons, and theme; flags
override it. Feature-gated JSON exposes the same rows to scripts and stays out
of the default binary. macOS and Linux are primary targets; Windows is a
stretch goal.

## Round 1 verdicts

| ID | Decision | Verdict |
|----|----------|---------|
| H-1 | Feature-gated `--json` | In vision |
| H-2 | Re-fetch on resize | In vision |
| H-3 | Icons off by default | In vision |
| H-4 | OS-matched logos | Off mission |
| H-5 | Windows first-class in v1 | Off mission |
| H-6 | Feature-gated sysinfo fallback | In vision |
| H-7 | Optional config file | In vision |
| H-8 | Resident `--watch` loop | Off mission |
| H-9 | Context rows by default | In vision |
| H-10 | Preset color themes | In vision |
| H-11 | Bendable measured size budget | In vision |
| H-12 | Temperature and GPU rows | In vision |

The product remains intentionally smaller than fastfetch: no interactive TUI
(the `--interactive` refresh loop only redraws the readout), daemon/watch mode,
OS-matched logo library, full theme system, package counts, or GPU
load/temperature/power charts.
