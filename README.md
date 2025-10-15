<div align="center">
  <img src="./sca-node/images/image.webp" width="160" height="160" alt="Smells Code Analyzer logo" />
</div>

# ✨ Smells code analyzer

<div align="center">
  <span>
    <a href="https://www.paypal.me/darkawower" title="Paypal" target="_blank">
      <img src="https://img.shields.io/badge/paypal-donate-blue.svg" alt="Donate via PayPal" />
    </a>
  </span>
  <span>
    <a href="https://patreon.com/artawower" target="_blank" title="Donate to this project using Patreon">
      <img src="https://img.shields.io/badge/patreon-donate-orange.svg" alt="Donate via Patreon" />
    </a>
  </span>
  <a href="https://wakatime.com/badge/user/dc4b055e-22c9-4977-bee4-51539164ae23/project/018c3624-755b-4e12-b942-49820de78842.svg">
    <img src="https://wakatime.com/badge/user/dc4b055e-22c9-4977-bee4-51539164ae23/project/018c3624-755b-4e12-b942-49820de78842.svg" alt="Wakatime badge" />
  </a>
</div>

Smells Code Analyzer finds unused or suspicious TypeScript code by combining tree-sitter parsing, ripgrep-based file discovery, and Language Server Protocol reference lookups. Both implementations share the same configuration schema and CLI flags.

## Implementations

### 🚀 Rust (Active)
- Location: `sca-rust/`
- CLI binary: `sca`
- Preferred distribution: crates.io (or prebuilt binaries)

```bash
cd sca-rust
cargo build --release
./target/release/sca --config-file ../config.json
```

Override the threshold if needed:

```bash
./target/release/sca --config-file ../config.json --threshold 50
```

Publishing instructions live in `sca-rust/README.md`.

### 🟠 Node.js (Deprecated)
- Location: `sca-node/`
- CLI entry: `bin/sca`
- Still works for existing users; no new features planned.

Install & build:

```bash
cd sca-node
npm install
npm run build
./bin/sca --config-file ../config.json
```

Development mode:

```bash
npm run dev -- --config-file ../config.json
```

Publishing details for the npm package are documented in `sca-node/README.md`.

## Configuration

Create a JSON file (e.g. `smells-code-analyzer.json`) at your project root with the following structure:

```json
{
  "lspExecutable": "node",
  "lspArgs": [
    "./node_modules/@angular/language-server",
    "--stdio",
    "--ngProbeLocations",
    "./node_modules/@angular/language-server/bin",
    "--tsProbeLocations",
    "./node_modules/typescript/lib"
  ],
  "showPassed": false,
  "grammar": "typescript",
  "projectRootPath": "/absolute/path/to/project",
  "analyzeDirectory": "/absolute/path/to/project/src",
  "fileMatchingRegexp": "**/*.ts",
  "showProgress": true,
  "encoding": "utf-8",
  "referenceNodes": [
    {
      "type": "interface_declaration",
      "refType": "type_identifier",
      "children": [{ "type": "property_identifier" }]
    },
    {
      "type": "class_declaration",
      "refType": "type_identifier",
      "children": [
        { "type": "public_field_definition", "refType": "property_identifier" },
        {}
      ]
    }
  ],
  "fileExcludeRegexps": ["**/*.spec.ts", "**/*.stories.ts"],
  "contentMatchingRegexp": "interface|class",
  "lspName": "angular-ls"
}
```

Run either CLI with the `--config-file` flag pointing to your config.

## Contributing
Contributions are welcome! Please read `sca-node/CONTRIBUTE.org` (shared style guide) before submitting PRs. Donations via [PayPal](https://www.paypal.me/darkawower) or [Patreon](https://www.patreon.com/artawower) are appreciated.
