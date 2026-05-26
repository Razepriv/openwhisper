# Release signing

OpenWhisper installers are auto-update enabled via Tauri's built-in updater.
That requires a minisign keypair: the public key ships in
`src-tauri/tauri.conf.json::plugins.updater.pubkey`; the private key signs
each release artifact so the in-app updater can verify the download.

This document is for **release managers**. Regular contributors don't need it.

## Generating a new keypair

```bash
bun run tauri signer generate -w ~/.openwhisper/updater.key --ci
```

This produces two files in `~/.openwhisper/`:

| File | Purpose | Visibility |
|------|---------|-----------|
| `updater.key` | Private signing key | **NEVER commit. NEVER share.** |
| `updater.key.pub` | Public verification key | Goes in `tauri.conf.json` |

The current public key embedded in the repo is the one we use for v0.1.x.
If you ever need to rotate (key compromise, new release manager), update
`tauri.conf.json::plugins.updater.pubkey` AND keep the old key around to
sign one final "we rotated keys, install this version" release — otherwise
existing installs can't trust the new key.

## Local production build

```bash
# Set the env var to the *contents* of the private key file
export TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.openwhisper/updater.key)"
# (Optional) if the key was generated with --password instead of --ci
# export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="<your password>"

bun run tauri build
```

Outputs end up under `src-tauri/target/release/bundle/`:
- `msi/OpenWhisper_<version>_x64_en-US.msi` — Windows MSI
- `nsis/OpenWhisper_<version>_x64-setup.exe` — Windows NSIS setup
- `dmg/OpenWhisper_<version>_<arch>.dmg` — macOS (when built on macOS)
- `deb/openwhisper_<version>_amd64.deb` — Linux Debian (when built on Linux)
- `appimage/openwhisper_<version>_amd64.AppImage` — Linux AppImage

Each gets a `.sig` companion file that the updater verifies on download.

## CI release builds

The `.github/workflows/release.yml` workflow is triggered by pushing a tag
matching `v*` (e.g. `v0.1.0`). It expects three repository secrets:

| Secret | Required for | How to set |
|--------|-------------|-----------|
| `TAURI_SIGNING_PRIVATE_KEY` | Auto-updater signature on every platform | `cat ~/.openwhisper/updater.key` → paste into repo Settings → Secrets |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | If the key was generated with a password | The password string |
| `APPLE_CERTIFICATE` etc. | Notarised macOS `.dmg` (optional) | See Tauri docs |

PR builds and dev/test workflows do **not** sign artifacts — the secrets
are only available on tagged release builds from the main branch.

## What "signing" does and doesn't do

- ✅ It allows the OpenWhisper in-app updater to verify that an installer
  downloaded from a release came from us, not a man-in-the-middle.
- ❌ It does NOT make Windows SmartScreen / macOS Gatekeeper happy. Those
  need a **separate** OS-level code-signing cert:
  - Windows: an EV code-signing cert (~$300-700/yr from a CA like Sectigo,
    DigiCert, etc.).
  - macOS: an Apple Developer ID Application cert ($99/yr Apple Developer
    Program subscription) plus notarisation.
- The Handy upstream's `signCommand` (Azure Trusted Signing) was removed
  during Phase 0 rebrand. To re-enable, add a `signCommand` back to
  `tauri.conf.json::bundle.windows`.

## Verifying a downloaded installer manually

```bash
# Pull the .sig file from the release alongside the installer:
curl -L https://github.com/Razepriv/openwhisper/releases/download/v0.1.0/OpenWhisper_0.1.0_x64-setup.exe.sig -o setup.exe.sig
curl -L https://github.com/Razepriv/openwhisper/releases/download/v0.1.0/OpenWhisper_0.1.0_x64-setup.exe -o setup.exe

# Verify with minisign (https://jedisct1.github.io/minisign/):
minisign -V -P "<contents of tauri.conf.json pubkey>" -m setup.exe -x setup.exe.sig
```

## Key custody

The current v0.1.x signing key was generated 2026-05-26 on a Windows dev
machine. It lives at `~/.openwhisper/updater.key` and is **not** in any
git repo. Before the project takes outside contributors, the key should
move to a hardware token (YubiKey supports minisign-compatible signing
via age) or a key-management service (1Password / AWS KMS / similar).
