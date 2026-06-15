# Contributing

OpenMux is currently an early-stage Rust workspace. The project should stay
small and predictable while the account-switching core is being built.

## Branch Strategy

Use GitHub Flow for now:

- `main` is the only long-lived branch.
- Create short-lived branches from `main`.
- Merge back to `main` through pull requests.
- Delete feature branches after merge.

This is intentionally lighter than Git Flow. OpenMux does not yet have a stable
release cadence, multiple maintained release lines, or a large team that would
justify permanent `develop` and `release/*` branches.

Recommended branch names:

```text
feature/<description>
bugfix/<description>
hotfix/<description>
refactor/<description>
docs/<description>
chore/<description>
```

Examples:

```text
feature/codex-import
feature/account-registry
bugfix/codex-auth-path
docs/readme-roadmap
chore/ci
```

When the project starts publishing stable releases, release branches can be
introduced only when needed:

```text
release/v0.1.0
hotfix/v0.1.1-auth-backup
```

## Versioning

OpenMux uses Semantic Versioning.

During `0.x`, the public API and CLI may still change:

- `0.MINOR.0`: meaningful feature milestone or breaking CLI/core change
- `0.MINOR.PATCH`: bug fix, docs, or internal improvement
- `1.0.0`: stable CLI surface and stable core plugin API

Version numbers should stay synchronized through the workspace package version
in the root `Cargo.toml`.

Release tags should use the `v` prefix:

```text
v0.1.0
v0.1.1
v1.0.0
```

Suggested early milestones:

- `v0.1.0`: Codex detect/import/list/use/current/doctor
- `v0.2.0`: vault abstraction and safer secret storage
- `v0.3.0`: Claude Code plugin
- `v0.4.0`: quota/status model
- `v1.0.0`: stable CLI and plugin API

## Commit Style

Use Conventional Commits where practical:

```text
feat(core): add account registry
feat(codex): detect auth path
fix(cli): return non-zero code for unknown platform
docs: explain branch strategy
chore(ci): add cargo checks
```

## Pull Request Checklist

- `cargo fmt --all`
- `cargo clippy --all-targets --all-features`
- `cargo test`
- README or docs updated when behavior changes
- No tokens, auth payloads, or private paths committed

## Safety Rules

OpenMux will operate on auth files and account state. Changes touching account
switching should be conservative:

- Never print raw tokens.
- Never store raw auth material in the registry.
- Back up active auth state before replacing it.
- Use atomic writes for active auth files.
- Roll back when a switch fails halfway.
- Keep diagnostics useful without exposing secrets.
