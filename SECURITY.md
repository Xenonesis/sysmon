# Security policy

## Supported version

Security fixes are applied to the latest released version of SysMon.

## Reporting a vulnerability

Use GitHub's private **Report a vulnerability** feature for `Xenonesis/sysmon`. Do not open a public issue containing an exploit, sensitive machine data or an update-verification bypass.

Include the affected version, Windows version, reproduction steps, impact and a minimal proof of concept. Remove personal process names, paths and session data that are not needed to reproduce the issue.

## Security boundaries

- Monitoring works as a standard user; elevation is requested for specific privileged actions.
- Diagnostic sessions and action audits are local files and may contain system or process metadata.
- Automatic updates require HTTPS, the official release repository, a bounded download, and a SHA-256 checksum match against the checksum file published with the release.
- Published builds must use the release workflow, which publishes the installer together with its SHA-256 checksum, an SPDX SBOM and a GitHub build provenance attestation. Locally built installers are development artifacts, not production updates.

Never commit passwords, access tokens or private diagnostic exports.
