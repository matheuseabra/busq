# User reference

## Install and run

Build and run from a checkout:

```sh
cargo run --release
```

Install the binary locally:

```sh
cargo install --path .
```

JSON support is optional. Build it explicitly when a script needs JSON:

```sh
cargo build --release --features json
minfetch --json
```

For exotic systems where the native collectors are unavailable, build with the
optional `sysinfo` fallback:

```sh
cargo build --release --features sysinfo
```

## Flags

| Flag | Effect |
| --- | --- |
| `--help` | Print usage. |
| `--version` | Print the package version. |
| `--config PATH` | Read defaults from a specific config file. |
| `--icons on\|off\|nerd` | Select portable Unicode, no icons (the default), or Nerd Font icons. `--no-icons` is an alias for off. |
| `--color auto\|always\|never` | Select label color behavior. Piped output stays plain. |
| `--color-no`, `--no-color` | Disable color. |
| `--theme subtle\|mono` | Select the label theme; otherwise minfetch detects a terminal hint. |
| `--logo [none\|auto\|PATH]` | Show the built-in OS logo; use `none` to disable it, `auto` explicitly, or load a file. |
| `--no-term` | Omit the uptime row. |
| `--no-terminator` | Omit the final newline. |
| `--verbose` | Print fetch failures to stderr. |
| `--probe`, `--debug-sysinfo` | Print platform, terminal, row, and error detection. |
| `--json` | Emit rows as JSON in a `json` feature build. |

## Configuration

minfetch uses the XDG path when `XDG_CONFIG_HOME` is set. Otherwise it uses the home config path:

```text
$XDG_CONFIG_HOME/minfetch/config
$HOME/.config/minfetch/config
```

Use `--config PATH` to choose a file. Each line has the form `key = value`; `#` starts a
comment. Supported keys are `color`, `icons`, `logo`, `rows`, and `theme`.

```text
color = auto
icons = nerd
logo = none
rows = hostname, user, os, shell, cpu, memory
theme = subtle
```

Command-line flags override config defaults. Unknown keys and invalid row names fail with a
line-numbered error.

Icons and logos are opt-in. Use `icons = on` for portable Unicode, `icons = nerd` for Nerd Font
icons, and `logo = auto` (or bare `--logo`) for the built-in OS logo.

### Theme detection

Theme precedence is: `--theme`, `theme` in the config file, `COLORFGBG` when stdout is a TTY,
then `subtle`. `COLORFGBG` uses its final semicolon-separated value as the ANSI background index:
`0`–`6` and `8` select `subtle`, while `7` and `15` select `mono`. Unset, malformed, or unrecognized
values use the deterministic `subtle` fallback. Piped output and `NO_COLOR` never emit ANSI,
regardless of the selected or detected theme.

## Layout and output

The renderer uses the detected terminal width and height. A logo and rows share a line when the
width allows it. Narrow panes stack the logo above the rows. At 30 columns or less, each label and
value gets its own line. Values truncate with a Unicode-width-aware ellipsis, and rows beyond the
height limit are omitted.

When stdout is not a TTY, minfetch omits the logo and ANSI escapes. A failed fetch renders `—` in
its row and does not stop the other rows. `--verbose` adds the underlying failure to stderr.

## Homebrew

`Formula/minfetch.rb` tracks the public `v0.1.0` source archive. Add this repository as a tap and
install the formula with:

```sh
brew tap matheuseabra/minfetch https://github.com/matheuseabra/minfetch.git
brew install matheuseabra/minfetch/minfetch
```
