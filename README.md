# Base64 for Tezbar

Encode and decode Base64 text interactively or directly from the clipboard.

## Commands

- **Base64** shows encoded and decoded results with copy, paste, and URL actions.
- **Base64 Encode Clipboard** replaces clipboard text with Unicode-safe Base64.
- **Base64 Decode Clipboard** validates and decodes clipboard text.

## Compatibility

The [Native V2 Rust port](v2/README.md) implements all three commands using
Tezbar beta's compiled-in adapter API and the original Raycast PNG. It includes
Rust 1.85 tests and integration instructions; using it in Tezbar requires host
registration and a rebuild. The JavaScript version below remains available.

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
