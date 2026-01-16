#!/usr/bin/env pwsh
# Script to automatically update SECURITY.md with current version information from Cargo.toml

$ErrorActionPreference = "Stop"

# Get script directory and project root
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir
$CargoToml = Join-Path $ProjectRoot "Cargo.toml"
$SecurityMd = Join-Path $ProjectRoot "SECURITY.md"

Write-Host "Updating SECURITY.md with current version information..." -ForegroundColor Cyan

# Check if files exist
if (-not (Test-Path $CargoToml)) {
    Write-Host "Error: Cargo.toml not found at $CargoToml" -ForegroundColor Red
    exit 1
}

if (-not (Test-Path $SecurityMd)) {
    Write-Host "Error: SECURITY.md not found at $SecurityMd" -ForegroundColor Red
    exit 1
}

# Extract version from Cargo.toml
$CargoContent = Get-Content $CargoToml -Raw
if ($CargoContent -match 'version\s*=\s*"(\d+)\.(\d+)\.(\d+)"') {
    $CurrentYear = $Matches[1]
    $CurrentMonth = $Matches[2]
    $CurrentBuild = $Matches[3]
    $CurrentVersion = "$CurrentYear.$CurrentMonth.$CurrentBuild"
    $CurrentVersionPattern = "$CurrentYear.$CurrentMonth.x"
} else {
    Write-Host "Error: Could not extract version from Cargo.toml" -ForegroundColor Red
    exit 1
}

Write-Host "Current version: $CurrentVersion" -ForegroundColor Green

# Calculate previous months for support tiers
$Year = [int]$CurrentYear
$Month = [int]$CurrentMonth

# Previous month (limited support)
$PrevMonth = $Month - 1
$PrevYear = $Year
if ($PrevMonth -lt 1) {
    $PrevMonth = 12
    $PrevYear = $Year - 1
}
$LimitedSupportPattern = "$PrevYear.$PrevMonth.x"

# Two months ago (no support)
$OldMonth = $Month - 2
$OldYear = $Year
if ($OldMonth -lt 1) {
    $OldMonth = $OldMonth + 12
    $OldYear = $Year - 1
}
if ($OldMonth -lt 1) {
    $OldMonth = $OldMonth + 12
    $OldYear = $OldYear - 1
}
$NoSupportPattern = "$OldYear.$OldMonth.x and older"

# Current date for "Last Updated"
$CurrentDate = Get-Date -Format "MMMM yyyy"

# Current year for copyright
$CopyrightYear = "20$CurrentYear"

Write-Host "Support tiers:" -ForegroundColor Yellow
Write-Host "  Current:  $CurrentVersionPattern" -ForegroundColor Green
Write-Host "  Limited:  $LimitedSupportPattern" -ForegroundColor Yellow
Write-Host "  None:     $NoSupportPattern" -ForegroundColor Red

# Read SECURITY.md
$SecurityContent = Get-Content $SecurityMd -Raw

# Update version table
$VersionTablePattern = '(?s)\| Version Pattern \| Supported\s+\| Notes \|\r?\n\| --------------- \| ------------------ \| ----- \|\r?\n\| [^\|]+\| ✅ Yes\s+\| Active development, security fixes prioritized \|\r?\n\| [^\|]+\| ⚠️ Limited\s+\| Critical security issues only \|\r?\n\| [^\|]+\| ❌ No\s+\| No security updates provided \|'

$NewVersionTable = @"
| Version Pattern | Supported          | Notes |
| --------------- | ------------------ | ----- |
| $CurrentVersionPattern (Current)| ✅ Yes             | Active development, security fixes prioritized |
| $LimitedSupportPattern         | ⚠️ Limited         | Critical security issues only |
| $NoSupportPattern| ❌ No            | No security updates provided |
"@

if ($SecurityContent -match $VersionTablePattern) {
    $SecurityContent = $SecurityContent -replace $VersionTablePattern, $NewVersionTable
    Write-Host "[OK] Updated version support table" -ForegroundColor Green
} else {
    Write-Host "Warning: Could not find version table pattern to update" -ForegroundColor Yellow
}

# Update footer metadata
$FooterPattern = '\*\*Last Updated\*\*: [^\r\n]+\r?\n\*\*Version\*\*: [^\r\n]+\r?\n\r?\n© \d{4} Logbie LLC\.'

$NewFooter = @"
**Last Updated**: $CurrentDate
**Version**: $CurrentVersion

© $CopyrightYear Logbie LLC.
"@

if ($SecurityContent -match $FooterPattern) {
    $SecurityContent = $SecurityContent -replace $FooterPattern, $NewFooter
    Write-Host "[OK] Updated footer metadata" -ForegroundColor Green
} else {
    Write-Host "Warning: Could not find footer pattern to update" -ForegroundColor Yellow
}

# Write updated content
Set-Content -Path $SecurityMd -Value $SecurityContent -NoNewline

Write-Host "`n[SUCCESS] SECURITY.md updated successfully!" -ForegroundColor Green
Write-Host "  Version: $CurrentVersion" -ForegroundColor Cyan
Write-Host "  Date: $CurrentDate" -ForegroundColor Cyan
Write-Host "`nReview the changes and commit if they look correct." -ForegroundColor Yellow
