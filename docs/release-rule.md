# Release Rule — SysMon

**When new features/version are ready, one command ships everything:**

```powershell
.\release.ps1 -Version 2.7.0 -Changelog "Cool new feature" -Sign -Publish
```

## What the rule does (in order)

1. **Bump version** in `Cargo.toml` (if `-Version` given).
2. **Update README.md** — version badge, download link, new changelog entry.
3. **Update website** — `docs/index.html` version tags (GitHub Pages source at `systemmonitor.xenonesis.dev`).
4. **Build** `cargo build --release`.
5. **Create installer** — `create-installer.ps1` produces `dist/SystemMonitor-vX.Y.Z/` + `.zip`.
6. **Delete old builds** — every other `dist/SystemMonitor-v*` folder/zip is removed; only the latest stays.
7. **Sign** (optional, `-Sign`) — Authenticode-sign the exe.
8. **Publish** (optional, `-Publish`):
   - commit + tag `vX.Y.Z` + push
   - create **GitHub release** with the zip attached
   - deploy the website (`deploy-website.ps1 -Deploy`)

## How installed users get the update notification

The app polls `https://api.github.com/repos/Xenonesis/sysmon/releases/latest`
every 24h (and on Ctrl+U). Once the GitHub release for `vX.Y.Z` exists:

- installed users see the **update banner** in-app,
- clicking **Install Update** downloads the installer, verifies the URL, size,
  and **Authenticode signature (must be `Valid`)**, then runs it.

## Hard requirements for the auto-update path to actually work

- **Trusted code-signing certificate** (e.g. DigiCert, Sectigo). Self-signed
  signatures fail the app's `Get-AuthenticodeSignature -eq Valid` check, so
  auto-install refuses them. `sign-binary.ps1` only makes a self-signed cert
  today — get a real cert for production releases.
- The GitHub release must attach `SystemMonitor-vX.Y.Z.zip`; the app only
  accepts installer `.exe` assets from the expected repo path.

## Without `-Publish`

The script still bumps versions, builds, refreshes `dist/`, and updates
README + website files. Run `-Publish` when ready to notify users.

## Caveats

- Website deploy needs `docs/` files present (they are).
- `deploy-website.ps1` handles the GitHub Pages push itself.
- Releases are deliberate: version bump + `release.ps1`, not automatic on
  every commit — auto-releasing on arbitrary commits would spam users.
