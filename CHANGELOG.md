# Changelog

All notable changes to busq are documented here.

## [Unreleased]

### Added

- Added Fastfetch-style OS details, Homebrew package counts, and filesystem type to the default rows.

## [1.1.0] - 2026-08-24

### Added

- Added the window manager to the default system statistics.

## [1.0.0] - 2026-08-24

### Changed

- Renamed the project and canonical command from `minfetch` to `busq`.
- Kept `minfetch` as a compatibility binary and retained the old config path as a fallback.

## [0.4.1] - 2026-08-23

### Fixed

- Prefer the terminal emulator name from `TERM_PROGRAM` (including Ghostty), with `TERM` as a fallback.

## [0.4.0] - 2026-08-23

### Added

- `--interactive` / `-i` refreshes the readout every second and exits on `q`.

### Changed

- Icons and the built-in OS logo are opt-in; the default readout is plain.
- Labels use a dimmer terminal style than values when ANSI output is enabled.

## [0.3.0] - 2026-08-23

### Added

- Built-in macOS and Linux ASCII logos with `--logo`.
- Nerd Font icons as the default icon set, with Unicode and no-icon alternatives.
- A recorded terminal demo and README status badges.

### Changed

- Compact Fastfetch-style identity, stat ordering, and memory/disk formatting.
- The default logo is disabled; `--logo` enables it.

## [0.2.0] - 2026-08-23

### Added

- A Homebrew tap formula for the public release.

## [0.1.0] - 2026-08-22

### Added

- Initial public release.
