# Release guide

agentmux releases are built by GitHub Actions from semantic-version tags matching `vMAJOR.MINOR.PATCH`.

## Before tagging

1. Confirm `Cargo.toml` contains the intended version.
2. Confirm the README and roadmap describe the shipped behavior.
3. Run the same checks as CI:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo build --locked --release
cargo package --locked
```

4. Commit the version and release notes, then ensure CI passes on `main`.

## Create the release

```sh
git tag -a v0.1.0 -m "agentmux v0.1.0"
git push origin main
git push origin v0.1.0
```

The release workflow:

1. Builds the locked release profile on Ubuntu.
2. Packages `agentmux` as `agentmux-vMAJOR.MINOR.PATCH-x86_64-unknown-linux-gnu.tar.gz`.
3. Generates a matching SHA-256 checksum.
4. Creates a GitHub Release with generated notes and both files attached.

## Verify the published artifact

Download the archive and checksum from the release page, then run:

```sh
sha256sum --check agentmux-v0.1.0-x86_64-unknown-linux-gnu.tar.gz.sha256
tar -xzf agentmux-v0.1.0-x86_64-unknown-linux-gnu.tar.gz
./agentmux --version
```

If any workflow step fails, fix the issue and create a new patch version. Do not move an already published release tag.
