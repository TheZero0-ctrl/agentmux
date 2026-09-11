# Releasing agentmux

Releases are created from annotated or lightweight semantic-version tags.

```sh
git tag v0.1.0
git push origin v0.1.0
```

The release workflow builds the locked release profile, packages the Linux
x86_64 binary, generates a SHA-256 checksum, and creates a GitHub Release with
generated notes. The tag must match `vMAJOR.MINOR.PATCH`.

Before tagging locally, run the same checks used by CI:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo build --locked --release
cargo package --locked --allow-dirty
```
