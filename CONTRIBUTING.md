# Contributing

## Before opening a pull request

Run the same checks used by CI:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --release
```

The coverage gate runs on Linux. To reproduce it locally with a rustup toolchain:

```sh
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --locked
cargo llvm-cov --locked --all-targets --all-features --fail-under-lines 90 --summary-only
```

Keep changes focused. Add or update tests for parsing and layout behavior,
especially for Linux/macOS differences, narrow panes, Unicode, and piped
output. Do not commit `.commandcode/`, `logs/`, or build artifacts.

## Pull requests

Describe the user-visible behavior, supported platforms, tests run, and any
known limitation. A maintainer may ask for a smaller change if a proposal
widens the dependency or feature surface without earning its size.

By contributing, you agree that your work is provided under the repository's
MIT license.
