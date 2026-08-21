# minfetch

`minfetch` is a tiny, pane-aware system-info readout for terminals. It prints
the shortest useful summary of a machine that still fits in a small pane.

The project is early-stage. macOS and Linux are the primary targets; the
current implementation is a single-shot readout with plain-text output,
optional icons, and width-aware rendering.

## Build and run

Requires a current stable Rust toolchain.

```sh
cargo run --release
cargo test --release
```

Useful flags:

```text
--help                 Show usage
--version              Show the version
--icons on|off         Enable or disable row icons
--color-no             Disable color output
--logo none|auto|PATH  Select the neutral logo, disable it, or load a file
```

Piped output is plain text and never includes ANSI escapes or the logo.

## Project direction

See [`docs/VISION-REVIEW.md`](docs/VISION-REVIEW.md) for the product direction
and [`docs/ROADMAP.md`](docs/ROADMAP.md) for the quality-gated plan.

## Contributing

Small, focused changes are welcome. Start with [`CONTRIBUTING.md`](CONTRIBUTING.md),
run the local checks, and explain platform-specific behavior in the pull
request.

## License

MIT. See [`LICENSE`](LICENSE).
