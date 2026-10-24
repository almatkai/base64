# Original icon — unchanged bytes

- Source: <https://raw.githubusercontent.com/raycast/extensions/6734d85992290066fd31a7f5bee3cca3d3545a1a/extensions/base64/assets/command-icon.png>
- Repository: https://github.com/raycast/extensions
- Pinned source commit: `6734d85992290066fd31a7f5bee3cca3d3545a1a`
- Original path: `extensions/base64/assets/command-icon.png`
- Local path: `assets/command-icon.png` (relative to this V2 crate)
- Size: **80527 bytes**
- SHA-256: `91b4ccba2a5abfaf8eb4541073a5bc10e06b8349a94b0c33538ebf8f0a2eca8c`
- Attribution: Raycast Base64 extension, DanielSinclair and contributors stelo,
  sammarks, ridemountainpig, luisafk. MIT; see `../LICENSE`.

This is the downloaded original PNG, not a redraw, SVG replacement, conversion,
resize, or recompression. V1's existing `assets/icon.svg` is unchanged.
`tezbar_base64_v2::ICON` embeds this exact file for offline host integration.
The beta schema requires the public manifest URL `/extensions/icons/base64.png`;
copy these same bytes to `src/renderer/public/extensions/icons/base64.png` in the host.
An installed-package fallback may use `assets/command-icon.png` without changing
or converting its bytes. The adapter contract test checks the hash and size.
