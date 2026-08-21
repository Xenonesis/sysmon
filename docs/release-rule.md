# SysMon release rule

Production releases are created only by `.github/workflows/windows-release.yml` from a tag matching the version in `Cargo.toml`, for example `v3.8.0`.


## Release steps

1. Update `Cargo.toml` version, `CHANGELOG.md`, `installer.iss` fallback `AppVersion`, and user-facing documentation.
2. Run the local quality commands from the README.
3. Commit and push the reviewed changes.
4. Create and push the matching `vX.Y.Z` tag.
5. Confirm **Build Windows Release** succeeds.
6. Confirm the release contains the installer, paired `.sha256` file and SPDX JSON SBOM, and shows build provenance.
7. Verify the published installer, checksum, and SBOM attestations with `gh attestation verify`.
8. On a clean Windows VM, verify the published checksum, install, launch, update detection and uninstall.

The workflow builds the application and installer, generates the SHA-256 checksum and SPDX SBOM, attests the application binary and all release artifacts, verifies the checksum/SBOM/attestations, and only then publishes the release. No certificate or signing secrets are required.


## Updater asset contract

The updater accepts only `SystemMonitor-X.Y.Z-setup.exe` from an HTTPS release URL in the official `Xenonesis/sysmon` repository together with the exact paired `.exe.sha256` asset. The checksum file must name that installer exactly. Downloads are size bounded and the installer must match the published SHA-256 checksum before execution. Releases additionally carry GitHub build provenance attestations verified by the publication workflow.

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
