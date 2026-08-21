# Security policy

## Supported version

Security fixes are applied to the latest released version of SysMon.

## Reporting a vulnerability

Use GitHub's private **Report a vulnerability** feature for `Xenonesis/sysmon`. Do not open a public issue containing an exploit, sensitive machine data or an update-verification bypass.

Include the affected version, Windows version, reproduction steps, impact and a minimal proof of concept. Remove personal process names, paths and session data that are not needed to reproduce the issue.

## Security boundaries

- Monitoring works as a standard user; elevation is requested for specific privileged actions.
- Diagnostic sessions, opt-in timeline history, and action audits are local files and may contain system or process metadata. Timeline storage excludes command lines, executable paths, working directories, usernames, and remote IP addresses.
- Automatic updates require HTTPS, the official release repository, exact paired installer/checksum asset names, a bounded download, and a SHA-256 checksum match against the checksum file published with the release.
- Published builds must use the release workflow, which publishes the installer together with its SHA-256 checksum and SPDX SBOM, and verifies GitHub build attestations before publication. Missing or invalid release evidence fails publication. Locally built installers are development artifacts, not production updates.

Never commit passwords, access tokens or private diagnostic exports.
