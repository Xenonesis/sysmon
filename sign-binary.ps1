param(
    [Parameter(Mandatory = $true)]
    [string]$FilePath,
    [string]$Thumbprint = $env:SYSMON_SIGNER_THUMBPRINT,
    [switch]$AllowDevelopmentCertificate
)

$resolved = Resolve-Path -LiteralPath $FilePath -ErrorAction Stop
$certificate = $null

if (-not [string]::IsNullOrWhiteSpace($Thumbprint)) {
    $normalized = $Thumbprint.Replace(' ', '')
    $candidate = "Cert:\CurrentUser\My\$normalized"
    if (Test-Path -LiteralPath $candidate) {
        $certificate = Get-Item -LiteralPath $candidate
    }
}

if (-not $certificate -and $AllowDevelopmentCertificate) {
    $subject = 'CN=System Monitor Development Only'
    $certificate = Get-ChildItem Cert:\CurrentUser\My |
        Where-Object Subject -eq $subject |
        Select-Object -First 1
    if (-not $certificate) {
        $certificate = New-SelfSignedCertificate -Subject $subject -Type CodeSigningCert -CertStoreLocation Cert:\CurrentUser\My
    }
    Write-Warning 'Using an untrusted development certificate. Never publish this artifact.'
}

if (-not $certificate) {
    throw 'No signing certificate found. Set SYSMON_SIGNER_THUMBPRINT or pass -AllowDevelopmentCertificate for local-only builds.'
}
if (-not $certificate.HasPrivateKey) {
    throw 'The selected signing certificate has no private key.'
}

$signature = Set-AuthenticodeSignature `
    -LiteralPath $resolved.Path `
    -Certificate $certificate `
    -TimestampServer 'http://timestamp.digicert.com' `
    -HashAlgorithm SHA256

if ($signature.Status -ne 'Valid' -and -not $AllowDevelopmentCertificate) {
    throw "Signature failed validation: $($signature.StatusMessage)"
}

Write-Host "Signed $($resolved.Path) with $($certificate.Thumbprint) ($($signature.Status))"
