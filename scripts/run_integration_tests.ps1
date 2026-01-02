# WFL Integration Test Runner (PowerShell)
# Ensures release binary is built before running integration tests

param(
    [switch]$Help,
    [switch]$BuildOnly,
    [switch]$TestOnly
)

if ($Help) {
    Write-Host "WFL Integration Test Runner (PowerShell)" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Usage: .\run_integration_tests.ps1 [options]" -ForegroundColor White
    Write-Host ""
    Write-Host "Options:" -ForegroundColor White
    Write-Host "  -Help          Show this help message" -ForegroundColor Gray
    Write-Host "  -BuildOnly     Only build release binary, don't run tests" -ForegroundColor Gray
    Write-Host "  -TestOnly      Only run tests, assume binary exists" -ForegroundColor Gray
    Write-Host ""
    Write-Host "This script ensures the WFL release binary is built before running" -ForegroundColor Gray
    Write-Host "integration tests that depend on it." -ForegroundColor Gray
    exit 0
}

Write-Host "[INFO] WFL Integration Test Runner" -ForegroundColor Blue
Write-Host "[INFO] ==========================" -ForegroundColor Blue

# Check if we're in the right directory
if (-not (Test-Path "Cargo.toml")) {
    Write-Host "[ERROR] Cargo.toml not found. Please run this script from the WFL project root." -ForegroundColor Red
    exit 1
}

$BinaryPath = "target\release\wfl.exe"

# Build-only mode
if ($BuildOnly) {
    Write-Host "[INFO] Build-only mode" -ForegroundColor Blue
    Write-Host "[INFO] Building release binary..." -ForegroundColor Blue
    & cargo build --release --verbose
    if ($LASTEXITCODE -eq 0) {
        Write-Host "[SUCCESS] Release build completed" -ForegroundColor Green
        if (Test-Path $BinaryPath) {
            Write-Host "[SUCCESS] Binary found: $BinaryPath" -ForegroundColor Green
            exit 0
        } else {
            Write-Host "[ERROR] Binary not found: $BinaryPath" -ForegroundColor Red
            exit 1
        }
    } else {
        Write-Host "[ERROR] Release build failed" -ForegroundColor Red
        exit 1
    }
}

# Test-only mode
if ($TestOnly) {
    Write-Host "[INFO] Test-only mode" -ForegroundColor Blue
    if (-not (Test-Path $BinaryPath)) {
        Write-Host "[ERROR] Release binary not found. Run without -TestOnly to build it first." -ForegroundColor Red
        exit 1
    }
    Write-Host "[SUCCESS] Binary found: $BinaryPath" -ForegroundColor Green
} else {
    # Check if release binary exists, build if not
    if (-not (Test-Path $BinaryPath)) {
        Write-Host "[INFO] Release binary not found, building..." -ForegroundColor Blue
        & cargo build --release --verbose
        if ($LASTEXITCODE -ne 0) {
            Write-Host "[ERROR] Failed to build release binary" -ForegroundColor Red
            exit 1
        }
        
        # Verify binary was created
        if (-not (Test-Path $BinaryPath)) {
            Write-Host "[ERROR] Release binary still not found after build" -ForegroundColor Red
            exit 1
        }
        Write-Host "[SUCCESS] Release build completed" -ForegroundColor Green
    }
    Write-Host "[SUCCESS] Binary found: $BinaryPath" -ForegroundColor Green
}

# Run integration tests
Write-Host "[INFO] Running integration tests..." -ForegroundColor Blue

Write-Host "[INFO] Running split functionality tests..." -ForegroundColor Blue
& cargo test --test split_functionality --verbose
if ($LASTEXITCODE -ne 0) {
    Write-Host "[ERROR] Split functionality tests failed" -ForegroundColor Red
    exit 1
}
Write-Host "[SUCCESS] Split functionality tests passed" -ForegroundColor Green

Write-Host "[INFO] Running all integration tests..." -ForegroundColor Blue
& cargo test --test '*' --verbose
if ($LASTEXITCODE -ne 0) {
    Write-Host "[ERROR] Some integration tests failed" -ForegroundColor Red
    exit 1
}
Write-Host "[SUCCESS] All integration tests passed" -ForegroundColor Green

# Run WFL test programs
Write-Host "[INFO] Running WFL test programs..." -ForegroundColor Blue

# Tests that require special handling (web servers, interactive tests)
# These are tested separately with dedicated scripts
$SkipTests = @(
    "simple_web_test.wfl",      # Web server - needs HTTP client
    "web_server_test.wfl",      # Web server - needs HTTP client
    "websocket_test.wfl"        # WebSocket - needs WS client
)

# Timeout for each test (seconds)
$TestTimeout = 30

if (-not (Test-Path "TestPrograms")) {
    Write-Host "[WARNING] TestPrograms directory not found, skipping WFL program tests" -ForegroundColor Yellow
} else {
    $wflFiles = Get-ChildItem -Path "TestPrograms" -Filter "*.wfl" -ErrorAction SilentlyContinue
    if ($wflFiles.Count -eq 0) {
        Write-Host "[WARNING] No WFL test programs found in TestPrograms/" -ForegroundColor Yellow
    } else {
        Write-Host "[INFO] Found $($wflFiles.Count) WFL test programs" -ForegroundColor Blue

        $failedPrograms = 0
        $skippedPrograms = 0
        foreach ($wflFile in $wflFiles) {
            # Check if this test should be skipped
            if ($SkipTests -contains $wflFile.Name) {
                Write-Host "[SKIP] $($wflFile.Name) (requires special handling)" -ForegroundColor Yellow
                $skippedPrograms++
                continue
            }

            Write-Host "[INFO] Testing: $($wflFile.Name)" -ForegroundColor Blue

            # Run with timeout using background job
            $job = Start-Job -ScriptBlock {
                param($binaryPath, $testFile)
                $output = & $binaryPath $testFile 2>&1
                $exitCode = $LASTEXITCODE
                # Return only the exit code as a structured object
                return @{ ExitCode = $exitCode }
            } -ArgumentList (Resolve-Path $BinaryPath).Path, $wflFile.FullName

            # Wait for completion with timeout
            $completed = Wait-Job -Job $job -Timeout $TestTimeout

            if ($null -eq $completed) {
                # Test timed out
                Stop-Job -Job $job
                Remove-Job -Job $job -Force
                Write-Host "[ERROR] TIMEOUT $($wflFile.Name) (exceeded ${TestTimeout}s)" -ForegroundColor Red
                $failedPrograms++
            } else {
                # Get exit code from job result
                $result = Receive-Job -Job $job
                $exitCode = if ($result -and $result.ExitCode -ne $null) { $result.ExitCode } else { 0 }
                Remove-Job -Job $job -Force

                if ($exitCode -eq 0) {
                    Write-Host "[SUCCESS] PASS $($wflFile.Name)" -ForegroundColor Green
                } else {
                    Write-Host "[ERROR] FAIL $($wflFile.Name) (exit code: $exitCode)" -ForegroundColor Red
                    $failedPrograms++
                }
            }
        }

        Write-Host ""
        Write-Host "[INFO] Results: $($wflFiles.Count - $skippedPrograms - $failedPrograms) passed, $failedPrograms failed, $skippedPrograms skipped" -ForegroundColor Blue

        if ($failedPrograms -eq 0) {
            Write-Host "[SUCCESS] All WFL test programs passed" -ForegroundColor Green
        } else {
            Write-Host "[ERROR] $failedPrograms WFL test programs failed" -ForegroundColor Red
            exit 1
        }
    }
}

Write-Host "[SUCCESS] All tests completed successfully!" -ForegroundColor Green
Write-Host "[INFO] Integration test runner finished" -ForegroundColor Blue
