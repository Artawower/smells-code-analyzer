# Smells Code Analyzer (Rust)

Rust implementation of the original `smells-code-analyzer` CLI.  
The binary reproduces the TypeScript version’s behaviour: it loads the same JSON
configuration, filters candidate files, walks TypeScript syntax trees via
tree-sitter, calls the configured language server for reference counts, and
prints the familiar emoji-styled report.

## Features
- Compatible CLI: `sca --config-file <path> [--threshold <n>]`
- JSON config schema shared with the Node.js tool
- File discovery and content filtering using `globset`/`ignore`
- Tree-sitter based structural matching with nested node targets
- Minimal LSP client (JSON-RPC) for `initialize`, `didOpen`, `references`, `shutdown`
- ASCII/non-ASCII sanitisation pipeline equivalent to the original implementation
- Threshold enforcement with non-zero exit code on smell overflow

## Getting Started
```bash
cd sca-rust
cargo build --release
./target/release/sca --config-file ../config.json
```

You can override the maximum number of allowed “dead” entities:
```bash
./target/release/sca --config-file ../config.json --threshold 50
```

To analyse a specific set of files listed in `changed-files.txt` (newline separated paths), use:
```bash
./target/release/sca --config-file ../config.json --files-from changed-files.txt
```
Paths may be absolute or relative to the list file; entries outside the analysed directory or missing on disk are skipped.

### Snapshot Modes

Generate a JSON snapshot of all detected errors (without emoji):
```bash
./target/release/sca --config-file ../config.json --generate-snapshot errors.json
```

Compare current analysis against a previous snapshot and fail with exit code if new errors are found:
```bash
./target/release/sca --config-file ../config.json --compare-snapshot errors.json
```
The `--compare-snapshot` flag will print all new errors and exit with an error if any new issues are detected.

Both snapshot flags work with `--files-from` to scope the analysis to a specific file list.

> **Note**  
> The language server referenced in the config (e.g. `node .../typescript-language-server`)
> must be accessible on the host machine before running the binary.

## Project Layout
```
src/
  main.rs          # CLI entry + orchestration
  config.rs        # JSON config loading, validation, encoding helpers
  model.rs         # Shared data structures
  sanitize.rs      # Source pre-processing
  analyzer/
    mod.rs         # Analyzer facade + aggregation helpers
    files.rs       # File discovery utilities
    tree.rs        # Tree-sitter traversal and node extraction
    lsp.rs         # Async JSON-RPC LSP client
    report.rs      # Emoji-styled report rendering
```

Logging is powered by `tracing`. Set `RUST_LOG=debug` for verbose diagnostics,
including raw LSP stderr output.

## Testing & Tooling
- `cargo fmt` ensures Rustfmt compliance (already applied).
- `cargo test` runs unit coverage (`sanitize` and `report` modules today).
- `cargo check` validates the build graph (requires access to crates.io).

## Publishing
1. Update `Cargo.toml` metadata (`version`, `description`, `repository`, `readme`, `keywords`, etc.).  
   The manifest already contains sensible defaults; bump the version before each release.
2. Ensure the crate builds cleanly and the README renders:  
   `cargo fmt && cargo check && cargo test && cargo doc --no-deps`.
3. Create a tagged release in git (e.g. `git tag -a vX.Y.Z -m "sca-rust vX.Y.Z"`).
4. Dry-run the publish to crates.io:
   ```bash
   cargo publish --dry-run
   ```
5. Publish for real once the dry-run succeeds:
   ```bash
   cargo publish
   ```
6. (Optional) Build release binaries for GitHub Releases:
   ```bash
   cargo build --release
   # tar/zip target/release/sca for each target triple you cross-compile
   ```
   Consider using `cross` or GitHub Actions to produce multi-platform artefacts.

## Next Steps
- Expand unit coverage for config parsing and tree traversal.
- Add integration tests against a TypeScript fixture to snapshot reports.
- Evaluate caching with pooled tree-sitter parsers for large codebases.
