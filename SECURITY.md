# Security policy

## Supported version

Security fixes are applied to the latest released version of SysMon.

## Reporting a vulnerability

Use GitHub's private **Report a vulnerability** feature for `Xenonesis/sysmon`. Do not open a public issue containing an exploit, signing material, sensitive machine data or an update-verification bypass.

Include the affected version, Windows version, reproduction steps, impact and a minimal proof of concept. Remove personal process names, paths and session data that are not needed to reproduce the issue.

## Security boundaries

- Monitoring works as a standard user; elevation is requested for specific privileged actions.
- Diagnostic sessions and action audits are local files and may contain system or process metadata.
- Automatic updates require HTTPS, the official release repository, a bounded download, valid Authenticode and the publisher certificate pinned into the running build.
- Published builds must use the signed release workflow. Self-signed builds are development artifacts, not production updates.

Never commit certificates, PFX files, passwords, access tokens or private diagnostic exports.
