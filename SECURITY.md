# Security Policy

ScoutPack is designed for local-only repo indexing.

## Guarantees

- No telemetry.
- No cloud calls.
- No API keys.
- No auto-edits.
- No project command execution in v0.1.
- Sensitive files are skipped by default.

## Sensitive Files

ScoutPack skips patterns including `.env`, `.env.*`, `*.pem`, `*.key`, `id_rsa`, `id_ed25519`, `*.p12`, and `*.mobileprovision`.

## Reporting

Please open a private security advisory or contact maintainers before public disclosure.

