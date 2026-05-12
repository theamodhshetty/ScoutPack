# Security Policy

ScoutPack is designed for local-only repo indexing.

## Guarantees

- No telemetry.
- No cloud calls.
- No API keys.
- No auto-edits.
- No project command execution in v0.1.
- Local SQLite index only.
- `.scoutpackignore` support.
- Sensitive files and generated folders are skipped by default.

## Sensitive Files

ScoutPack skips patterns including `.env`, `.env.*`, `*.pem`, `*.key`, `id_rsa`, `id_ed25519`, `secrets.yaml`, `secrets.json`, `.npmrc`, `.pypirc`, `*.p12`, and `*.mobileprovision`.

## Generated Folders

ScoutPack skips common generated and dependency folders including `node_modules`, `.git`, `.next`, `dist`, `build`, `coverage`, `target`, and `.venv`.

## Reporting

Please open a private security advisory or contact maintainers before public disclosure.

## Current Limitations

ScoutPack is a local indexing tool, not a secret scanner. Sensitive patterns are skipped as a safety baseline, but users should still keep private keys, tokens, and credentials out of repositories.
