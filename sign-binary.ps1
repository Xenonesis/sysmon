param(
    [Parameter(Mandatory=$true)]
    [string]$FilePath
)

if (-not (Test-Path $FilePath)) {
    Write-Host "❌ Error: File '$FilePath' not found!" -ForegroundColor Red
    exit 1
}

$certSubject = "CN=System Monitor Dev"
$certStore = "Cert:\CurrentUser\My"

# Find existing certificate
$cert = Get-ChildItem -Path $certStore | Where-Object { $_.Subject -eq $certSubject } | Select-Object -First 1

if (-not $cert) {
    Write-Host "Creating new self-signed code signing certificate ($certSubject)..." -ForegroundColor Cyan
    $cert = New-SelfSignedCertificate -Subject $certSubject -Type CodeSigningCert -CertStoreLocation $certStore
    
    # Export and import into Trusted Root to make the signature valid
    Write-Host "Adding certificate to Trusted Root Certification Authorities..." -ForegroundColor Cyan
    $TempStore = "$env:TEMP\sysmon_cert.cer"
    Export-Certificate -Cert $cert -FilePath $TempStore | Out-Null
    Import-Certificate -FilePath $TempStore -CertStoreLocation Cert:\CurrentUser\Root | Out-Null
    Remove-Item $TempStore -ErrorAction SilentlyContinue
}

if ($cert) {
    Write-Host "Signing '$FilePath'..." -ForegroundColor Cyan
    $sig = Set-AuthenticodeSignature -Certificate $cert -FilePath $FilePath -TimestampServer "http://timestamp.digicert.com"
    if ($sig.Status -eq "Valid") {
        Write-Host "✅ Successfully signed $FilePath" -ForegroundColor Green
    } else {
        Write-Host "❌ Signature Failed: $($sig.StatusMessage)" -ForegroundColor Red
        exit 1
    }
} else {
    Write-Host "❌ Could not find or create code signing certificate." -ForegroundColor Red
    exit 1
}
