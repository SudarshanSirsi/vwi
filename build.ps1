# VWi Release Build Script
# Builds a single optimized binary and copies it to the project root.

param(
    [string]$OutputPath = ".\vwi.exe"
)

Write-Host "Building VWi release binary..." -ForegroundColor Cyan

# Ensure we're in the script's directory
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $scriptDir

# Build optimized release
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Error "Build failed!"
    exit 1
}

# Copy binary to requested output path
$src = ".\target\release\vwi.exe"
Copy-Item $src $OutputPath -Force

Write-Host "Success! Binary saved to: $OutputPath" -ForegroundColor Green
Write-Host "File size: $([math]::Round((Get-Item $OutputPath).Length / 1MB, 2)) MB" -ForegroundColor Gray

# Optional: print instructions
Write-Host ""
Write-Host "To run VWi:" -ForegroundColor Yellow
Write-Host "  1. Create config dir: mkdir `$env:APPDATA\vwi"
Write-Host "  2. Copy config:       Copy-Item config.example.toml `$env:APPDATA\vwi\config.toml"
Write-Host "  3. Edit config:       notepad `$env:APPDATA\vwi\config.toml"
Write-Host "  4. Run:               .\vwi.exe"
Write-Host ""
Write-Host "To add to startup, press Win+R and run: shell:startup"
Write-Host "Then create a shortcut to vwi.exe in that folder."
