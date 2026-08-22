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
cargo install --path .
```

Homebrew can install the tagged public release through a tap:

```sh
brew tap matheuseabra/minfetch https://github.com/matheuseabra/minfetch.git
brew install matheuseabra/minfetch/minfetch
```

Useful flags:

```text
--help                 Show usage
--version              Show the version
--config PATH          Load optional defaults from a config file
--icons on|off         Enable or disable row icons (`--no-icons` is an alias)
--color-no             Disable color output
--color auto|always|never
                       Select color behavior (never emits ANSI when piped)
--theme subtle|mono    Select the color theme (otherwise detect a terminal hint)
--probe                Dump platform, terminal, row, and error detection
--json                 Emit rows as JSON (feature build only)
--no-term              Omit the uptime row
--no-terminator        Omit the final newline
--verbose              Print fetch failures to stderr
--logo none|auto|PATH  Select the neutral logo, disable it, or load a file
```

Piped output is plain text and never includes ANSI escapes or the logo.

An optional config file at `$XDG_CONFIG_HOME/minfetch/config` (or
`$HOME/.config/minfetch/config`) accepts `color`, `icons`, `logo`, `rows`, and
`theme` keys as `key = value`; command-line flags override its defaults. Use
`--config PATH` to select another file.

Without an explicit theme, a TTY `COLORFGBG` hint selects `subtle` for dark ANSI
backgrounds (`0`–`6` and `8`) or `mono` for light backgrounds (`7` and `15`); missing
or malformed hints fall back to `subtle`. Piped output and `NO_COLOR` remain
ANSI-free.

JSON output is feature-gated: build with `cargo build --features json` before
using `--json`.

## tmux and zsh

Install `minfetch` so it is on your `PATH`, then run it in a pane:

```sh
tmux split-window -h 'minfetch --no-term'
```

For a compact plain-text pane, use:

```sh
minfetch --no-term --no-icons --color never
```

minfetch takes one snapshot and exits. It does not start a watch loop.

## Project direction

See [`docs/VISION.md`](docs/VISION.md) for the product direction
and [`docs/ROADMAP.md`](docs/ROADMAP.md) for the quality-gated plan. See the
[`user reference`](docs/REFERENCE.md) for flags, configuration, and layout behavior.

## Contributing

Small, focused changes are welcome. Start with [`CONTRIBUTING.md`](CONTRIBUTING.md),
run the local checks, and explain platform-specific behavior in the pull
request.

## License

MIT. See [`LICENSE`](LICENSE).
