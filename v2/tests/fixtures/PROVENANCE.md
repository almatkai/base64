# Beta contract fixtures

Read-only reference: Tezbar `origin/beta`, full commit
`fd300e276356587726566026f072563eb9c34cb1`.

These two files were copied byte-for-byte with `git show <commit>:<path>`:

| Fixture | Source path | SHA-256 |
| --- | --- | --- |
| `beta_common.rs` | `src-tauri/src/native_core/common.rs` | `280a0a2f0dd04a8570b320aafc0c3ece0f5de8ddb21c363645567d6a35d84da7` |
| `extension-v2.schema.json` | `schemas/extension-v2.schema.json` | `6d5afda1b652605982104bdf16d1547547b98dfd707a7b2969316406c49f32af` |

`adapter_contract.rs` compiles the unchanged production `tezbar_adapter.rs`
against the actual beta `CoreContext` and router implementation, not a handwritten
substitute. Only Tauri's `AppHandle`/`Emitter` are stubbed, and emission panics.
All tested requests use a recording router with no real clipboard, browser, paste,
or window actions. Two upstream common.rs tests also run against temporary files.
This is not a full Tezbar application build or OS UI test.

The Tezbar host fixtures are distributed under Apache-2.0; the upstream license
is retained in `TEZBAR-LICENSE.txt`. The Rust port and original Raycast icon use
the MIT license retained at the crate root.

The schema validates `manifest.json` using JSON Schema draft 2020-12. Beta requires
an absolute `/extensions/icons/...` or `/extensions/artwork/...` icon URL.
Hashes in the contract test guard accidental fixture edits. Keep beta snapshot
updates explicit and review them together with the production adapter.
