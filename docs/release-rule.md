# SysMon release rule

Production releases are created only by `.github/workflows/windows-release.yml` from a tag matching the version in `Cargo.toml`, for example `v3.6.0`.

## One-time repository setup

Add these GitHub Actions secrets:

- `WINDOWS_SIGNING_PFX_BASE64`: base64 of the production Authenticode PFX.
- `WINDOWS_SIGNING_PFX_PASSWORD`: PFX password.

The certificate must be trusted for code signing and contain a private key. Never commit the PFX or password. The workflow derives the thumbprint from the imported certificate and compiles it into the updater as `SYSMON_SIGNER_THUMBPRINT`.

## Release steps

1. Update `Cargo.toml` version, `CHANGELOG.md`, `installer.iss` fallback `AppVersion`, and user-facing documentation.
2. Run the local quality commands from the README.
3. Commit and push the reviewed changes.
4. Create and push the matching `vX.Y.Z` tag.
5. Confirm **Build Signed Windows Release** succeeds.
6. Confirm the release contains the installer, `.sha256` file and SPDX JSON SBOM, and shows build provenance.
7. On a clean Windows VM, verify signature, install, launch, update detection and uninstall.

The workflow fails closed when certificate secrets are missing. It signs both `system-monitor.exe` and `SystemMonitor-X.Y.Z-setup.exe`, verifies the installer's signer matches the thumbprint pinned into the binary, and only then publishes the release.

## Local development signing

`sign-binary.ps1` requires `SYSMON_SIGNER_THUMBPRINT` or `-Thumbprint` by default. `-AllowDevelopmentCertificate` is an explicit local-only escape hatch. Development certificates must never be used for published updates.

## Updater asset contract

The updater accepts an HTTPS `.exe` release asset from the official `Xenonesis/sysmon` repository. Downloads are size bounded and must pass Authenticode validity and publisher-thumbprint checks before execution.

## Version integrity

**The website resolves the download URL dynamically** via the GitHub Releases API
(`/repos/Xenonesis/sysmon/releases/latest`). The JS resolver (`docs/script.js →
resolveDownload()`) finds the `*-setup.exe` asset and updates **all** download
buttons plus the version display elements (`#latestVersion`, `#changelog .section-title`).

### Rules to prevent stale-version incidents

1. **Never commit installer `.exe` files** into `docs/downloads/`. The `.gitignore`
   blocks this. The only correct delivery path is a tagged GitHub Release.

2. **Keep `installer.iss` `#define AppVersion` in sync with `Cargo.toml`.**
   CI overrides it with `/DAppVersion`, but local builds use the fallback and
   must produce correctly versioned output.

3. **Bump the JS cache key** (`CACHE_KEY` in `resolveDownload()`) whenever a
   version mismatch incident occurs or after a major version jump. Current key:
   `sysmon_release_cache_v3`.

4. **Verify the GitHub Release exists** before merging a version bump PR.
   No release → the API returns 404 → the download page shows a fallback message.
