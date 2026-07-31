# WFL Windows installer (MSI) build entry point.
#
# Moved from root build_msi.ps1 by the repository hygiene migration
# (REPOSITORY_HYGIENE.md); the retired Tools MSI launcher's version-bump
# option lives on here as -BumpVersion.
#
# Usage: ./scripts/build_windows_installer.ps1 [-OutputDir <dir>] [-BumpVersion]
param (
    [string]$OutputDir,
    [switch]$BumpVersion
)

# Always operate from the repository root, wherever the script is invoked from.
Set-Location (Join-Path $PSScriptRoot "..")

# Optionally bump the build number first (previously the MSI launcher's job).
if ($BumpVersion) {
    Write-Host "Bumping version..." -ForegroundColor Cyan
    python scripts/bump_version.py --skip-git
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Version bump failed. Exit code: $LASTEXITCODE" -ForegroundColor Red
        exit 1
    }
}

# Check for required dependencies
Write-Host "Checking for required dependencies..." -ForegroundColor Cyan

# Check for WiX Toolset
$wixInstalled = $false
if (Test-Path "${env:ProgramFiles(x86)}\WiX Toolset*" -ErrorAction SilentlyContinue) {
    $wixInstalled = $true
} elseif (Test-Path "${env:ProgramFiles}\WiX Toolset*" -ErrorAction SilentlyContinue) {
    $wixInstalled = $true
} elseif ($null -ne $env:WIX) {
    $wixInstalled = $true
}

if (-not $wixInstalled) {
    Write-Host "WiX Toolset not found. To install the WiX Toolset:" -ForegroundColor Red
    Write-Host " Method 1 (Recommended) - Using Chocolatey:" -ForegroundColor Yellow
    Write-Host "   1. Install Chocolatey package manager from https://chocolatey.org/" -ForegroundColor Yellow
    Write-Host "   2. Run 'choco install wixtoolset -y' as administrator" -ForegroundColor Yellow
    Write-Host
    Write-Host " Method 2 - Manual Installation:" -ForegroundColor Yellow
    Write-Host "   1. Download WiX Toolset from https://wixtoolset.org/releases/" -ForegroundColor Yellow
    Write-Host "   2. Run the installer and follow the prompts" -ForegroundColor Yellow
    Write-Host "   3. Make sure the WiX Toolset bin directory is added to your PATH" -ForegroundColor Yellow
    Write-Host
    Write-Host "After installing WiX Toolset, run this script again." -ForegroundColor Yellow
    exit 1
}
Write-Host "WiX Toolset found." -ForegroundColor Green

# Install cargo-wix if it's not already installed
Write-Host "Checking for cargo-wix..." -ForegroundColor Cyan
if (-not ((cargo --list 2>$null) | Select-String -Pattern "wix" -Quiet)) {
    Write-Host "Installing cargo-wix..." -ForegroundColor Yellow
    cargo install cargo-wix@0.3.3 --locked
    
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Failed to install cargo-wix. Please check your Rust installation." -ForegroundColor Red
        exit 1
    }
    
    Write-Host "Successfully installed cargo-wix." -ForegroundColor Green
} else {
    Write-Host "cargo-wix already installed." -ForegroundColor Green
}

# Create default config file
Write-Host "Creating configuration files..." -ForegroundColor Cyan
New-Item -ItemType Directory -Force -Path target/x86_64-pc-windows-msvc/release/package | Out-Null
Set-Content -Path "target/x86_64-pc-windows-msvc/release/package/.wflcfg" -Value "timeout_seconds = 60"
Add-Content -Path "target/x86_64-pc-windows-msvc/release/package/.wflcfg" -Value "logging_enabled = false"
Add-Content -Path "target/x86_64-pc-windows-msvc/release/package/.wflcfg" -Value "debug_report_enabled = true"
Add-Content -Path "target/x86_64-pc-windows-msvc/release/package/.wflcfg" -Value "log_level = info"

# Copy config to root for wix.toml
Copy-Item target/x86_64-pc-windows-msvc/release/package/.wflcfg -Destination .wflcfg

# Initialize WiX source files
Write-Host "Initializing WiX source files..." -ForegroundColor Cyan
try {
    # Check if the wix directory exists
    if (-not (Test-Path "wix")) {
        Write-Host "Generating WiX source files..." -ForegroundColor Yellow
        cargo wix init -p wfl
        
        if ($LASTEXITCODE -ne 0) {
            Write-Host "Failed to initialize WiX source files. Exit code: $LASTEXITCODE" -ForegroundColor Red
            exit 1
        }
    } else {
        Write-Host "WiX source files already exist." -ForegroundColor Green
    }
} catch {
    Write-Host "An error occurred while initializing WiX source files:" -ForegroundColor Red
    Write-Host $_.Exception.Message -ForegroundColor Red
    exit 1
}

# Update version information with the version script
Write-Host "Updating version information..." -ForegroundColor Cyan
try {
    if (Test-Path "scripts/bump_version.py") {
        # Use the enhanced version script with --update-wix-only to avoid bumping the version number
        python scripts/bump_version.py --update-wix-only --skip-git
        
        if ($LASTEXITCODE -ne 0) {
            Write-Host "Failed to update version information. Exit code: $LASTEXITCODE" -ForegroundColor Red
            exit 1
        }
        Write-Host "Version information successfully updated." -ForegroundColor Green
    } else {
        Write-Host "Warning: Version script not found at scripts/bump_version.py" -ForegroundColor Yellow
        Write-Host "Falling back to manual version update..." -ForegroundColor Yellow
        
        $wixTomlContent = Get-Content -Path "wix.toml" -Raw
        $updatedWixTomlContent = $wixTomlContent -replace 'version = "0.0.0.0" # Will be overridden by cargo-wix command line', 'version = "2025.4.0.0" # Updated by build_windows_installer.ps1'
        Set-Content -Path "wix.toml" -Value $updatedWixTomlContent
        Write-Host "Version updated in wix.toml." -ForegroundColor Green
    }
} catch {
    Write-Host "An error occurred while updating version information:" -ForegroundColor Red
    Write-Host $_.Exception.Message -ForegroundColor Red
    exit 1
}

# Check if binary exists and build it if necessary
Write-Host "Checking for wfl.exe binary..." -ForegroundColor Cyan
$binaryPath = "target/release/wfl.exe"
if (-not (Test-Path $binaryPath)) {
    Write-Host "Binary not found. Building wfl.exe..." -ForegroundColor Yellow
    try {
        cargo build --release -p wfl
        
        if ($LASTEXITCODE -ne 0) {
            Write-Host "Failed to build wfl.exe. Exit code: $LASTEXITCODE" -ForegroundColor Red
            exit 1
        }
        
        Write-Host "Successfully built wfl.exe." -ForegroundColor Green
    } catch {
        Write-Host "An error occurred while building wfl.exe:" -ForegroundColor Red
        Write-Host $_.Exception.Message -ForegroundColor Red
        exit 1
    }
} else {
    Write-Host "Binary found at $binaryPath." -ForegroundColor Green
}

# Build MSI with corrected cargo-wix command
Write-Host "Building MSI installer..." -ForegroundColor Cyan
try {
    # Get version from wix.toml
    $wixTomlContent = Get-Content -Path "wix.toml" -Raw
    $versionMatch = [regex]::Match($wixTomlContent, 'version\s*=\s*"([^"]*)"')
    $version = "2025.4"
    if ($versionMatch.Success) {
        $version = $versionMatch.Groups[1].Value.Split('.')[0..1] -join '.'
    }
    
    # Determine output path
    $outputPath = "target/x86_64-pc-windows-msvc/release/wfl-$version.msi"
    if ($OutputDir) {
        # Create the output directory if it doesn't exist
        if (-not (Test-Path $OutputDir)) {
            New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
            Write-Host "Created output directory: $OutputDir" -ForegroundColor Green
        }
        $outputPath = Join-Path -Path $OutputDir -ChildPath "wfl-$version.msi"
        Write-Host "Using custom output path: $outputPath" -ForegroundColor Cyan
    }
    
    cargo wix --no-build --nocapture `
      --output $outputPath `
      -p wfl
    
    if ($LASTEXITCODE -eq 0) {
        if (Test-Path $outputPath) {
            Write-Host "MSI successfully created at: $outputPath" -ForegroundColor Green
        } else {
            Write-Host "Build command succeeded but MSI file not found at expected location: $outputPath" -ForegroundColor Yellow
        }
    } else {
        Write-Host "Failed to build MSI installer. Exit code: $LASTEXITCODE" -ForegroundColor Red
    }
} catch {
    Write-Host "An error occurred while building the MSI:" -ForegroundColor Red
    Write-Host $_.Exception.Message -ForegroundColor Red
    exit 1
}
