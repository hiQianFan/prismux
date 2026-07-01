# Security Policy

Prismux handles local credentials, auth snapshots, backups, and account
metadata. Please do not report suspected credential leaks in public issues.

## Reporting a Vulnerability

Use GitHub private vulnerability reporting for this repository when available.

If private vulnerability reporting is not available, open a minimal public issue
asking for a private contact channel. Do not include tokens, `auth.json`,
`.credentials.json`, snapshots, backups, registry files, or private account file
contents in public issues.

## What to Report

Please report issues such as:

- raw token or auth payload disclosure in stdout/stderr
- raw auth material stored in registry metadata
- active credential replacement without backup
- snapshot hash verification bypass
- rollback failures that leave credentials in a corrupted state
- private credential files written with overly broad permissions where the
  platform supports private permissions

## Supported Versions

Prismux is in the `0.x` phase. Security fixes target the latest release unless a
later policy says otherwise.

## Security Boundary

- Codex account snapshots may contain raw auth payloads and must remain private.
- Claude OAuth account snapshots may contain raw credential payloads and must
  remain private.
- Registry metadata must not contain raw access tokens, refresh tokens, API
  keys, or full auth payloads.
- Backups may contain credentials and must be treated as sensitive.
- Prismux does not implement Anthropic OAuth token exchange and does not call
  private Anthropic endpoints.

