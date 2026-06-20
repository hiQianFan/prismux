# tokscale-core vendor notes

- Upstream: https://github.com/junhoyeo/tokscale
- Upstream commit: cbbd0dffda93a3a4588fc08fd631ca10bba73ff1
- Upstream license: MIT, copied to `LICENSE.upstream`
- Vendored path: `vendor/tokscale-core`
- Local policy: keep parser logic unchanged when possible; isolate OpenMux-specific mapping in `crates/omx-usage-tokscale`.

Local manifest changes:

- Replaced upstream workspace-inherited package metadata with explicit values.
- Replaced upstream workspace-inherited dependency declarations with explicit versions copied from upstream root `Cargo.toml`.

Upgrade policy:

- Refresh this directory from a fixed upstream commit.
- Reapply only manifest/vendor metadata changes.
- Run OpenMux adapter fixtures before accepting parser behavior changes.
