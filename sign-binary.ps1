param(
    [Parameter(Mandatory=$true)]
    [string]$FilePath
)

if (-not (Test-Path $FilePath)) {
    Write-Host "[FAIL] Error: File '$FilePath' not found!" -ForegroundColor Red
    exit 1
}

$certSubject = "CN=System Monitor Dev"
$certStore = "Cert:\CurrentUser\My"

# Find existing certificate
$cert = Get-ChildItem -Path $certStore | Where-Object { $_.Subject -eq $certSubject } | Select-Object -First 1

if (-not $cert) {
    Write-Host "Creating new self-signed code signing certificate ($certSubject)..." -ForegroundColor Cyan
    $cert = New-SelfSignedCertificate -Subject $certSubject -Type CodeSigningCert -CertStoreLocation $certStore
}

if ($cert) {
    Write-Host "Signing '$FilePath'..." -ForegroundColor Cyan
    $sig = Set-AuthenticodeSignature -Certificate $cert -FilePath $FilePath -TimestampServer "http://timestamp.digicert.com"
    if ($sig.Status -eq "Valid" -or $sig.Status -eq "UnknownError" -or $sig.Status -eq "UntrustedRoot") {
        Write-Host "[OK] Successfully signed $FilePath (Status: $($sig.Status))" -ForegroundColor Green
    } else {
        Write-Host "[FAIL] Signature Failed: $($sig.StatusMessage) (Status: $($sig.Status))" -ForegroundColor Red
        exit 1
    }
} else {
    Write-Host "[FAIL] Could not find or create code signing certificate." -ForegroundColor Red
    exit 1
}
