# Releasing busq

1. Create a release PR that moves the relevant entries from `Unreleased` to a dated version in
   `CHANGELOG.md`, and bumps `version` in `Cargo.toml`.
2. Run `cargo test --locked --release` and `cargo clippy --locked --all-targets --all-features -- -D warnings`.
3. Merge the release PR into `main`, then confirm `cargo run --release -- --version` reports the intended version.
4. Tag the merged commit and push it: `git tag -a vX.Y.Z -m "busq vX.Y.Z" && git push origin vX.Y.Z`.
5. The Release workflow verifies the tag, uploads macOS/Linux binaries, and creates the GitHub release from that version's changelog section.
6. Update `Formula/busq.rb` to the tagged source archive and its SHA-256, then push that formula change to `main`.
7. Verify the public install with `brew update && brew upgrade matheuseabra/busq/busq` and `brew test matheuseabra/busq/busq`.
