#!/usr/bin/env pwsh
# Generate self-signed certificates for testing HTTPS functionality

$CERT_DIR = "TestPrograms\.ssl"

Write-Host "Generating test certificates for HTTPS testing..." -ForegroundColor Cyan

# Create .ssl directory if it doesn't exist
New-Item -ItemType Directory -Force -Path $CERT_DIR | Out-Null

# Check if OpenSSL is available
$openssl = Get-Command openssl -ErrorAction SilentlyContinue

if ($null -eq $openssl) {
    Write-Host "✗ OpenSSL not found. Please install OpenSSL to generate certificates." -ForegroundColor Red
    Write-Host ""
    Write-Host "Installation options:" -ForegroundColor Yellow
    Write-Host "  - Windows: Download from https://slproweb.com/products/Win32OpenSSL.html" -ForegroundColor Yellow
    Write-Host "  - Or use Git Bash which includes OpenSSL" -ForegroundColor Yellow
    exit 1
}

# Generate self-signed certificate valid for 365 days
$ErrorActionPreference = 'Continue'
& openssl req -x509 -newkey rsa:2048 -nodes `
    -keyout "$CERT_DIR\key.pem" `
    -out "$CERT_DIR\cert.pem" `
    -days 365 `
    -subj "/CN=localhost/O=WFL Test/C=US" `
    2>$null

if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ Test certificates generated successfully in $CERT_DIR\" -ForegroundColor Green
    Write-Host "  - cert.pem: Certificate" -ForegroundColor Gray
    Write-Host "  - key.pem: Private key" -ForegroundColor Gray
    Write-Host ""
    Write-Host "These certificates are for testing only and should not be used in production." -ForegroundColor Yellow
} else {
    Write-Host "✗ Failed to generate certificates." -ForegroundColor Red
    exit 1
}
