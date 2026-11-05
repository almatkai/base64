# Base64 for Tezbar

Encode and decode Base64 text interactively or directly from the clipboard.

## Commands

- **Base64** shows encoded and decoded results with copy, paste, and URL actions.
- **Base64 Encode Clipboard** replaces clipboard text with Unicode-safe Base64.
- **Base64 Decode Clipboard** validates and decodes clipboard text.

## Native Rust installable extension (schema 3, process)

This repository also builds a standalone, installable Tezbar extension:
`com.tezbar.native.base64` (`extension.json`, schema 3, `execution: process`,
protocol 1), replacing `raycast.base64`. It is a separate executable
(`bin/tezbar-base64`) that speaks Tezbar's JSONL stdio protocol through the
immutable SDK pin `tezbar-extension-sdk` @ `aaec571673ab64a85a136cf6bd054ef660c4d93c`.
No Tezbar/Tauri crate, Node, Bun or JavaScript runtime is involved at runtime.
The Raycast TypeScript source under `src/` stays unchanged alongside it.

- Engine: the reviewed [Native V2 library](v2/README.md) (`v2/`, unchanged)
  provides all behavior. `src-rust/main.rs` serves it with SDK `serve()`;
  `src-rust/host.rs` maps its five routes to typed host requests only:
  `clipboard:read` → `host.clipboard.read`, `clipboard:write` →
  `host.clipboard.write`, `core:platform:paste-text` → `host.clipboard.pasteText`,
  `shell:open` → `host.shell.open`, `window:hide` → `host.window.visibility {visible:false}`.
  Any other route is rejected locally. The manifest grants exactly these capabilities.
- Commands: `index` (view), `encode` and `decode` (no-view), all `runtime: rust` on
  macOS and Windows; `defaultAction` preference preserved verbatim. No gaps.
- State lives only in the SDK session (no persistent storage, no migration paths).

```sh
export CARGO_TARGET_DIR="$PWD/target"
cargo test --locked -j2                       # host mapping + real-pipe process tests
(cd v2 && CARGO_TARGET_DIR="$PWD/../target/v2" cargo test --locked -j2)  # preserved V2 tests
cargo build --locked -j2 --release --target aarch64-apple-darwin --bin tezbar-base64
python3 scripts/package.py --target aarch64-apple-darwin \
  --binary target/aarch64-apple-darwin/release/tezbar-base64
python3 scripts/smoke-package.py dist/base64-aarch64-apple-darwin.zip
```

The ZIP contains `extension.json`, `bin/tezbar-base64[.exe]`, `resources/`
(the original icon), `licenses/`, `LICENSE` and `NOTICE`; its `files` map
inventories every member except the manifest. `dist/` also gets the archive
`.sha256`, executable `.sha256`, `files.json` and `artifact.json` sidecars.
`artifacts` stays empty in source; release URLs/digests are pinned by the trusted
host catalog. Tests use a recording fake host and never touch the real clipboard,
browser or window. Real Tezbar desktop QA has not been done on either platform,
and the Windows target has not been built here.

## Compatibility

The [Native V2 Rust port](v2/README.md) implements all three commands using
Tezbar beta's compiled-in adapter API and the original Raycast PNG. It includes
Rust 1.85 tests. Its compiled-in integration path is superseded by the
installable process package above, which reuses the same library. The JavaScript version below remains available.

- Windows: built and exercised in Tezbar on Windows 11, including clipboard round trips.
- macOS: uses only cross-platform Node.js and Raycast APIs and retains the upstream macOS-compatible design.
- Linux: not currently declared.

## Local verification

```powershell
pnpm install
pnpm typecheck
pnpm test
```

## Credits

Adapted for Tezbar from Raycast's MIT-licensed `base64` extension at upstream commit `6734d85992290066fd31a7f5bee3cca3d3545a1a`. Original author and contributors remain credited in `package.json`.
