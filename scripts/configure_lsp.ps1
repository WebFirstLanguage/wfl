# PowerShell script to configure LSP server after installation
param (
    [string]$InstallDir = $env:ProgramFiles + "\WFL"
)

$ErrorActionPreference = "Continue"

Write-Host "Configuring WFL LSP Server..." -ForegroundColor Green

# LSP server binary path
$lspServerPath = Join-Path $InstallDir "bin\wfl-lsp.exe"

# Check if LSP server exists
if (-not (Test-Path $lspServerPath)) {
    Write-Host "LSP server not found at $lspServerPath" -ForegroundColor Yellow
    Write-Host "LSP server configuration skipped." -ForegroundColor Yellow
    exit 0
}

Write-Host "LSP server found at: $lspServerPath" -ForegroundColor Green

# Validate LSP server
try {
    $versionOutput = & $lspServerPath --version 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "LSP server validation successful: $versionOutput" -ForegroundColor Green
    } else {
        Write-Host "LSP server validation failed" -ForegroundColor Yellow
    }
} catch {
    Write-Host "Could not validate LSP server: $($_.Exception.Message)" -ForegroundColor Yellow
}

# Create registry entries for LSP server
try {
    $registryPath = "HKLM:\SOFTWARE\WFL"
    if (-not (Test-Path $registryPath)) {
        New-Item -Path $registryPath -Force | Out-Null
    }
    
    Set-ItemProperty -Path $registryPath -Name "LSPServerPath" -Value $lspServerPath
    Set-ItemProperty -Path $registryPath -Name "LSPServerVersion" -Value "1.0.0"
    
    Write-Host "LSP server registry entries created successfully" -ForegroundColor Green
} catch {
    Write-Host "Could not create registry entries: $($_.Exception.Message)" -ForegroundColor Yellow
}

# Create VS Code settings template (if VS Code is detected)
$vscodeSettingsTemplate = @{
    "wfl.serverPath" = $lspServerPath
    "wfl.serverArgs" = @("--stdio")
    "wfl.versionMode" = "warn"
}

$vscodeUserDir = "$env:APPDATA\Code\User"
if (Test-Path $vscodeUserDir) {
    try {
        $settingsFile = Join-Path $vscodeUserDir "settings.json"
        $templateFile = Join-Path $InstallDir "config\vscode-settings-template.json"
        
        # Create config directory if it doesn't exist
        $configDir = Join-Path $InstallDir "config"
        if (-not (Test-Path $configDir)) {
            New-Item -ItemType Directory -Path $configDir -Force | Out-Null
        }
        
        # Save template
        $vscodeSettingsTemplate | ConvertTo-Json -Depth 3 | Out-File -FilePath $templateFile -Encoding UTF8
        
        Write-Host "VS Code settings template created at: $templateFile" -ForegroundColor Green
        Write-Host "You can merge these settings with your existing VS Code settings." -ForegroundColor Cyan
    } catch {
        Write-Host "Could not create VS Code settings template: $($_.Exception.Message)" -ForegroundColor Yellow
    }
}

Write-Host "LSP server configuration completed." -ForegroundColor Green
