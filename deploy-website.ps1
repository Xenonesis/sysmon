# System Monitor Website Deployment Script
# This script helps deploy the website to GitHub Pages

param(
    [switch]$Deploy,
    [switch]$Test
)

$repoUrl = "https://github.com/Xenonesis/sysmon.git"
$websiteFiles = @("index.html", "styles.css", "script.js")

Write-Host "System Monitor Website Deployment Tool" -ForegroundColor Cyan
Write-Host "=====================================" -ForegroundColor Cyan

if ($Test) {
    Write-Host "`nTesting website locally..." -ForegroundColor Yellow

    # Check if all required files exist
    $allFilesExist = $true
    foreach ($file in $websiteFiles) {
        if (Test-Path $file) {
            Write-Host "✓ $file found" -ForegroundColor Green
        } else {
            Write-Host "✗ $file missing" -ForegroundColor Red
            $allFilesExist = $false
        }
    }

    if ($allFilesExist) {
        Write-Host "`nAll website files are present!" -ForegroundColor Green

        # Try to start a local server
        Write-Host "`nStarting local web server on http://localhost:8000" -ForegroundColor Yellow
        Write-Host "Press Ctrl+C to stop the server" -ForegroundColor Gray

        try {
            # Use Python's built-in server if available
            if (Get-Command python -ErrorAction SilentlyContinue) {
                python -m http.server 8000
            }
            elseif (Get-Command python3 -ErrorAction SilentlyContinue) {
                python3 -m http.server 8000
            }
            else {
                Write-Host "Python not found. Please install Python or use another web server." -ForegroundColor Red
                Write-Host "You can also open index.html directly in your browser." -ForegroundColor Yellow
            }
        }
        catch {
            Write-Host "Could not start local server. You can open index.html directly in your browser." -ForegroundColor Yellow
        }
    } else {
        Write-Host "`nSome files are missing. Please ensure all website files are created." -ForegroundColor Red
    }

    exit
}

if ($Deploy) {
    Write-Host "`nDeploying website to GitHub Pages..." -ForegroundColor Yellow

    # Check if all required files exist
    $allFilesExist = $true
    foreach ($file in $websiteFiles) {
        if (-not (Test-Path $file)) {
            Write-Host "✗ $file missing" -ForegroundColor Red
            $allFilesExist = $false
        }
    }

    if (-not $allFilesExist) {
        Write-Host "`nCannot deploy: Some website files are missing." -ForegroundColor Red
        exit 1
    }

    # Check if git is available
    if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
        Write-Host "Git is not installed. Please install Git first." -ForegroundColor Red
        exit 1
    }

    # Check if we're in a git repository
    if (-not (Test-Path ".git")) {
        Write-Host "Not in a git repository. Please run this from the project root." -ForegroundColor Red
        exit 1
    }

    try {
        # Create docs folder for GitHub Pages
        if (-not (Test-Path "docs")) {
            New-Item -ItemType Directory -Path "docs" | Out-Null
        }

        # Copy website files to docs folder
        Copy-Item "index.html" "docs\" -Force
        Copy-Item "styles.css" "docs\" -Force
        Copy-Item "script.js" "docs\" -Force

        Write-Host "✓ Website files copied to docs/ folder" -ForegroundColor Green

        # Add and commit the changes
        git add docs/
        git commit -m "Deploy website to GitHub Pages"

        Write-Host "✓ Changes committed" -ForegroundColor Green

        # Push to main branch
        git push origin main

        Write-Host "✓ Changes pushed to GitHub" -ForegroundColor Green

        Write-Host "`n🎉 Deployment successful!" -ForegroundColor Green
        Write-Host "`nNext steps:" -ForegroundColor Cyan
        Write-Host "1. Go to your GitHub repository" -ForegroundColor White
        Write-Host "2. Navigate to Settings > Pages" -ForegroundColor White
        Write-Host "3. Set Source to 'Deploy from a branch'" -ForegroundColor White
        Write-Host "4. Set Branch to 'main' and folder to '/docs'" -ForegroundColor White
        Write-Host "5. Click Save" -ForegroundColor White
        Write-Host "`nYour website will be available at: https://xenonesis.github.io/sysmon/" -ForegroundColor Green

    }
    catch {
        Write-Host "`n❌ Deployment failed: $($_.Exception.Message)" -ForegroundColor Red
        exit 1
    }

    exit
}

# Default help message
Write-Host "`nUsage:" -ForegroundColor Yellow
Write-Host "  .\deploy-website.ps1 -Test     # Test website locally" -ForegroundColor White
Write-Host "  .\deploy-website.ps1 -Deploy   # Deploy to GitHub Pages" -ForegroundColor White
Write-Host "`nExamples:" -ForegroundColor Gray
Write-Host "  .\deploy-website.ps1 -Test" -ForegroundColor White
Write-Host "  .\deploy-website.ps1 -Deploy" -ForegroundColor White