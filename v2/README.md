# Base64 — Native V2 Rust library

This is a **compiled-in** Tezbar Native V2 adapter, not an external process, plugin
binary, WASM module, or directly installable store package. The existing V1
TypeScript, package metadata, tests, icon, and MIT license are untouched.

Reference host: **Tezbar beta `fd300e276356587726566026f072563eb9c34cb1`**.
Checked read-only with `git show` against beta's `NATIVE_V2_FORMAT.md`, manifest
schema, `v2_runtime.rs`, `v2_sdk.rs`, `native.rs`, `common.rs`, and platform routes.
The beta schema requires `icon: "/extensions/icons/base64.png"`.
See `tests/fixtures/PROVENANCE.md`.

## Build and test

From this directory:

```sh
cargo build --locked
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo +1.85.0 test --locked
cargo package --list --allow-dirty
```

`rust-version = "1.85"`; the checked-in lockfile was resolved for Rust 1.85 and
tests pass on 1.85.0 as well as the current toolchain. Keep `Cargo.lock` in host
integration and review dependency updates. Runtime dependencies are `base64`,
`serde_json`, and `url`; JSON Schema, SHA-256, Tokio, and the beta fixture support
are test-only dependencies. This is a library crate with no executable target.
Packaging explicitly includes only library sources, manifest, unchanged icon,
license, docs, adapter and test fixtures; it excludes `target` and `node_modules`.

From the repository root, V1 checks and optional oracle regeneration:

```sh
pnpm install --frozen-lockfile --ignore-scripts
pnpm test
pnpm typecheck
node --import tsx v2/tests/generate_v1_vectors.mjs
```

Tests cover all three commands, real beta `CoreContext`/request ABI via the
unchanged production wrapper, exact roots/envelopes, preferences, stale/forged
action rejection, target selection, search/refresh/dispose, failure propagation,
HTTP(S)-only opening, JavaScript whitespace including FEFF, canonical trailing
bits, V1's unusual padding acceptance, and invalid UTF-8 replacement. An oracle
fixture contains **1018 deterministic vectors generated from untouched V1**.
The adapter harness substitutes only Tauri AppHandle/Emitter and a recording
router; it does not access the real clipboard, paste keys, browser or window.

**Not tested:** a complete Tezbar rebuild, actual macOS or Windows UI, clipboard
permissions, focus restoration and OS paste. Both manifest verification entries
remain `not-run`. Cross-platform declarations are intended support, not QA claims.

## Library interface

```rust,ignore
pub fn commands() -> &'static [&'static str]; // index, encode, decode
pub async fn handle(
    host: &impl Host,
    command: &str,
    operation: &str,
    payload: &serde_json::Value,
    state: &mut serde_json::Value,
) -> Result<serde_json::Value, String>;
```

`Host::request` returns a Send boxed future. `tezbar_adapter.rs` wraps the actual
`CoreContext` and exposes Tezbar's concrete `handle(ctx, command, operation,
payload, state)` signature. `ICON` exports all 80527 original PNG bytes and
`MANIFEST` exports the bundled JSON, both available offline.

Operations use the actual Native V2 names:

- `extension:run-command`: `index` initializes from clipboard (read failure gives
  the original empty UX); `encode`/`decode` validate before writing and hide only
  after a successful copy. Errors remain errors, including host hide failures.
- `extension:search-text-changed`: `payload.searchText` updates results live;
  never rereads clipboard. Root is `List` with `__hasServerSearch`, `List.Section`,
  `List.Item` and `List.EmptyView`; action metadata uses `actionIds`.
- `extension:invoke-action`: `payload.actionId` resolves an authoritative state
  target, not supplied content/URL. Revisions invalidate IDs after search/relaunch.
  Default action is captured from launch `payload.preferences.defaultAction`.
- `extension:refresh-session`: returns the current view without new clipboard
  reads or invalidating IDs. `extension:load-more` returns `unchanged`.
- `extension:dispose-session`: clears targets and marks the session inactive.

The host owns session IDs, installed-command checks, cancellation and preference
loading (`native.rs`). No-view commands do not create a view session. All OS
requests are awaited and propagate errors; no background tasks or persistence.

| Effect | Core route | Positional args |
| --- | --- | --- |
| Read clipboard | `clipboard:read` | `[]` — returns a string |
| Copy | `clipboard:write` | `[text]` |
| Paste | `core:platform:paste-text` | `[text]` |
| Open HTTP(S) URL | `shell:open` | `[url]` |
| Hide after clipboard commands/open | `window:hide` | `[]` |

Beta's paste-text route itself writes the text, hides Tezbar, waits for focus,
and sends Cmd/Ctrl+V; this adapter must not invoke it with an object payload or
invent a `clipboard:paste` route. Copy keeps the interactive view. Paste/open
return `no-view`; open calls hide only after successful shell open.

## Integrate into Tezbar beta (requires host code changes and rebuild)

These are instructions for a later, explicitly authorized **host** change. This
repository does not apply them or modify an installed extension/app checkout.
A manifest or `native-v2.json` marker alone cannot register executable Rust code.

1. Vendor this V2 crate at a stable path, e.g. `src-tauri/native-extensions/base64`.
   Add to `src-tauri/Cargo.toml`:

   ```toml
   tezbar-base64-v2 = { path = "native-extensions/base64" }
   ```

   Update and review Tezbar's own Cargo.lock (the host resolves its own graph).
   Preserve the crate's LICENSE and attribution. Copy `tezbar_adapter.rs` to
   `src-tauri/src/native_core/extensions/v2_base64.rs`, unchanged.

2. Add `mod v2_base64;` in `native_core/extensions/mod.rs`. In `v2_runtime.rs`:
   add `"base64"` to the `slug()` allowlist; add these match arms to `commands`
   and `handle` respectively:

   ```rust,ignore
   "base64" => super::v2_base64::commands(),
   "base64" => super::v2_base64::handle(ctx, command, operation, payload, state).await,
   ```

   Canonical ID is `com.tezbar.native.base64`; replacement is `raycast.base64`.
   Keep the existing installed-command/session checks. Do not route to JS or
   invent an external-process loader.

3. Append this crate's **manifest object** to `v2_catalog.json`. It is schema 2,
   with `index`, `encode`, `decode`, `defaultAction`, both platform declarations
   and both verification entries. Validate against beta's actual schema. Update
   any tests asserting the old hard-coded catalog count (11 becomes 12), and add
   compiled coverage tests for all three Base64 commands.

4. Copy `assets/command-icon.png` byte-for-byte to the host's
   **`src/renderer/public/extensions/icons/base64.png`** (beta's Vite `publicDir`). Keep the manifest URL exactly
   `/extensions/icons/base64.png` as required by beta's schema. See
   `assets/PROVENANCE.md` for the pinned Raycast URL/hash. No conversion,
   substitution, resizing, or recompression is needed or permitted.

   Beta's `v2_catalog::install` currently writes only JSON. For offline
   installed-package icon fallback, add a Base64-specific asset write in that
   installer, before the marker/database success is recorded:

   ```rust,ignore
   if slug == "base64" {
       std::fs::create_dir_all(root.join("assets")).map_err(|e| e.to_string())?;
       std::fs::write(
           root.join("assets/command-icon.png"),
           super::v2_base64::ICON,
       ).map_err(|e| e.to_string())?;
   }
   ```

   The resulting fallback file is under the **managed canonical package**
   `extensions/packages/com.tezbar.native.base64/assets/command-icon.png`.
   If that file is needed by a fallback asset resolver, resolve it explicitly
   there; do not overwrite V1's icon or rewrite the beta catalog's absolute URL.
   This snippet is an integration requirement, not functionality already present
   in beta or an operation performed by the library/tests.

5. Build/test the host and inspect actual compiled coverage with its V2 CLI.
   Only **after rebuilding/registration**, install the canonical ID through the
   host's normal Native V2 catalog install route. It creates managed package
   JSON/marker/database state; do not replace a user's V1 package manually.
   Confirm `commands()` and runtime dispatch report all three as Rust.

6. Run real UI QA separately on macOS and Windows: clipboard initialization,
   typing Unicode, both result rows, each preference/default action, URL rejection,
   copy/paste/open focus, malformed input and denied clipboard access. Record
   platform QA artifacts/commit before changing `verification` from `not-run`.

## Attribution

Original MIT Base64 extension: DanielSinclair, stelo, sammarks, ridemountainpig,
luisafk; copyright (c) 2021 Raycast. Tezbar V1 adaptation: almatkai.
Rust behavior is translated from this repo's V1 source. The original MIT text is
retained in `LICENSE`. No V1 source, icon or installer metadata was replaced.
The beta host test fixture is Apache-2.0; its license is retained in
`tests/fixtures/TEZBAR-LICENSE.txt`.
