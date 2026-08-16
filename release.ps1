# Local release readiness helper. Publishing is intentionally delegated to the
# GitHub Actions release workflow triggered by a matching version tag.

param(
    [string]$Version = "",
    [string]$Changelog = "",
    [switch]$Publish
)

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

if ($Publish) {
    throw "Local publishing is disabled. Push a reviewed vX.Y.Z tag and use the Windows release workflow."
}

$cargo = Get-Content "Cargo.toml" -Raw
if ($cargo -notmatch '(?m)^version\s*=\s*"([^"]+)"') {
    throw "Cannot read the package version from Cargo.toml"
}
$current = $Matches[1]
if ($Version -and $Version -ne $current) {
    throw "Requested version $Version does not match Cargo.toml version $current. Update source and changelog in a reviewed change first."
}
if ($Changelog) {
    Write-Warning "-Changelog no longer edits files automatically; update CHANGELOG.md in the reviewed source change."
}

Write-Host "Checking release readiness for v$current" -ForegroundColor Cyan
cargo metadata --locked --format-version 1 --no-deps | Out-Null
if ($LASTEXITCODE -ne 0) { throw "Locked dependency validation failed" }
cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) { throw "Formatting check failed" }
cargo clippy --locked --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) { throw "Clippy failed" }
cargo test --locked --all-targets
if ($LASTEXITCODE -ne 0) { throw "Tests failed" }
cargo build --locked --release --bin system-monitor
if ($LASTEXITCODE -ne 0) { throw "Release build failed" }

Write-Host "Local checks passed for v$current." -ForegroundColor Green
Write-Host "Production publishing: review, commit, push, then push tag v$current." -ForegroundColor Cyan
