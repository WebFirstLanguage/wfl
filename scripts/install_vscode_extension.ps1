# PowerShell script to install VS Code extension
param (
    [string]$InstallDir = $env:ProgramFiles + "\WFL"
)

$ErrorActionPreference = "Continue"

Write-Host "Installing WFL VS Code Extension..." -ForegroundColor Green

$extensionPath = Join-Path $InstallDir "vscode-extension"

# Check if extension files exist
if (-not (Test-Path $extensionPath)) {
    Write-Host "VS Code extension files not found at $extensionPath" -ForegroundColor Yellow
    Write-Host "VS Code extension installation skipped." -ForegroundColor Yellow
    exit 0
}

# Check if VS Code is installed
$vscodePaths = @(
    "${env:ProgramFiles}\Microsoft VS Code\bin\code.cmd",
    "${env:ProgramFiles(x86)}\Microsoft VS Code\bin\code.cmd",
    "${env:LOCALAPPDATA}\Programs\Microsoft VS Code\bin\code.cmd"
)

$vscodeCmd = $null
foreach ($path in $vscodePaths) {
    if (Test-Path $path) {
        $vscodeCmd = $path
        Write-Host "Found VS Code at: $path" -ForegroundColor Green
        break
    }
}

if ($vscodeCmd) {
    try {
        Write-Host "Installing VS Code extension from $extensionPath" -ForegroundColor Cyan

        # Look for VSIX file first
        $vsixFile = Join-Path $extensionPath "vscode-wfl-0.1.0.vsix"
        if (Test-Path $vsixFile) {
            Write-Host "Installing from VSIX file: $vsixFile" -ForegroundColor Cyan
            & $vscodeCmd --install-extension $vsixFile --force

            if ($LASTEXITCODE -eq 0) {
                Write-Host "VS Code extension installed successfully from VSIX" -ForegroundColor Green
            } else {
                Write-Host "VSIX installation failed, trying directory installation..." -ForegroundColor Yellow

                # Fallback to directory installation
                & $vscodeCmd --install-extension $extensionPath --force

                if ($LASTEXITCODE -eq 0) {
                    Write-Host "VS Code extension installed successfully from directory" -ForegroundColor Green
                } else {
                    Write-Host "VS Code extension installation failed" -ForegroundColor Red
                }
            }
        } else {
            Write-Host "VSIX file not found, installing from directory: $extensionPath" -ForegroundColor Cyan
            & $vscodeCmd --install-extension $extensionPath --force

            if ($LASTEXITCODE -eq 0) {
                Write-Host "VS Code extension installed successfully from directory" -ForegroundColor Green
            } else {
                Write-Host "VS Code extension installation failed" -ForegroundColor Red
            }
        }

        # Create registry entry for successful installation
        try {
            $registryPath = "HKLM:\SOFTWARE\WFL"
            if (-not (Test-Path $registryPath)) {
                New-Item -Path $registryPath -Force | Out-Null
            }
            Set-ItemProperty -Path $registryPath -Name "VSCodeExtensionInstalled" -Value "true"
            Write-Host "VS Code extension registry entry created" -ForegroundColor Green
        } catch {
            Write-Host "Could not create registry entry: $($_.Exception.Message)" -ForegroundColor Yellow
        }

    } catch {
        Write-Host "Error during VS Code extension installation: $($_.Exception.Message)" -ForegroundColor Red
    }
} else {
    Write-Host "VS Code not found in standard locations:" -ForegroundColor Yellow
    foreach ($path in $vscodePaths) {
        Write-Host "  - $path" -ForegroundColor Yellow
    }
    Write-Host "VS Code extension installation skipped." -ForegroundColor Yellow
    Write-Host "You can manually install the extension later from: $extensionPath" -ForegroundColor Cyan
}

Write-Host "VS Code extension installation process completed." -ForegroundColor Green
