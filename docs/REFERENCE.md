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

## Flags

| Flag | Effect |
| --- | --- |
| `--help` | Print usage. |
| `--version` | Print the package version. |
| `--config PATH` | Read defaults from a specific config file. |
| `--icons on\|off` | Enable or disable row icons. `--no-icons` is an alias for off. |
| `--color auto\|always\|never` | Select label color behavior. Piped output stays plain. |
| `--color-no`, `--no-color` | Disable color. |
| `--theme subtle\|mono` | Select the label theme. |
| `--logo none\|auto\|PATH` | Disable the logo, use the neutral logo, or load a file. |
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
icons = on
logo = auto
rows = hostname, user, os, shell, cpu, memory
theme = subtle
```

Command-line flags override config defaults. Unknown keys and invalid row names fail with a
line-numbered error.

## Layout and output

The renderer uses the detected terminal width and height. A logo and rows share a line when the
width allows it. Narrow panes stack the logo above the rows. At 30 columns or less, each label and
value gets its own line. Values truncate with a Unicode-width-aware ellipsis, and rows beyond the
height limit are omitted.

When stdout is not a TTY, minfetch omits the logo and ANSI escapes. A failed fetch renders `—` in
its row and does not stop the other rows. `--verbose` adds the underlying failure to stderr.
