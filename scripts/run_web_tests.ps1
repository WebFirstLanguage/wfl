# WFL Web Server Integration Test Runner (PowerShell)
# Tests WFL web server functionality by starting servers and sending HTTP requests

param(
    [switch]$Help,
    [int]$Timeout = 10
)

if ($Help) {
    Write-Host "WFL Web Server Test Runner" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Usage: .\run_web_tests.ps1 [options]" -ForegroundColor White
    Write-Host ""
    Write-Host "Options:" -ForegroundColor White
    Write-Host "  -Help          Show this help message" -ForegroundColor Gray
    Write-Host "  -Timeout <sec> Timeout for each test (default: 10)" -ForegroundColor Gray
    Write-Host ""
    Write-Host "This script tests WFL web server functionality by:" -ForegroundColor Gray
    Write-Host "  1. Starting the WFL web server in background" -ForegroundColor Gray
    Write-Host "  2. Sending HTTP requests to verify it works" -ForegroundColor Gray
    Write-Host "  3. Checking responses and cleaning up" -ForegroundColor Gray
    exit 0
}

Write-Host "[INFO] WFL Web Server Test Runner" -ForegroundColor Blue
Write-Host "[INFO] ============================" -ForegroundColor Blue

# Check if we're in the right directory
if (-not (Test-Path "Cargo.toml")) {
    Write-Host "[ERROR] Cargo.toml not found. Please run this script from the WFL project root." -ForegroundColor Red
    exit 1
}

$BinaryPath = "target\release\wfl.exe"

# Check if release binary exists
if (-not (Test-Path $BinaryPath)) {
    Write-Host "[ERROR] Release binary not found. Run 'cargo build --release' first." -ForegroundColor Red
    exit 1
}
Write-Host "[SUCCESS] Binary found: $BinaryPath" -ForegroundColor Green

# Function to test a web server
function Test-WflWebServer {
    param(
        [string]$TestFile,
        [int]$Port,
        [string]$ExpectedResponse,
        [int]$TimeoutSeconds
    )

    $testName = Split-Path $TestFile -Leaf
    Write-Host ""
    Write-Host "[INFO] Testing: $testName on port $Port" -ForegroundColor Blue

    # Start the WFL server in background
    $serverProcess = Start-Process -FilePath ".\$BinaryPath" -ArgumentList $TestFile -NoNewWindow -PassThru -RedirectStandardOutput "NUL" -RedirectStandardError "NUL"

    try {
        # Wait for server to start (with retries)
        $serverReady = $false
        $retries = 0
        $maxRetries = $TimeoutSeconds * 2  # Check every 500ms

        while (-not $serverReady -and $retries -lt $maxRetries) {
            Start-Sleep -Milliseconds 500
            $retries++

            # Try to connect
            try {
                $response = Invoke-WebRequest -Uri "http://localhost:$Port/" -TimeoutSec 2 -UseBasicParsing -ErrorAction Stop
                $serverReady = $true
            } catch {
                # Server not ready yet, continue waiting
            }
        }

        if (-not $serverReady) {
            Write-Host "[ERROR] TIMEOUT: Server did not start within ${TimeoutSeconds}s" -ForegroundColor Red
            return $false
        }

        # Server is ready, check response
        if ($response.Content -like "*$ExpectedResponse*") {
            Write-Host "[SUCCESS] PASS: Got expected response" -ForegroundColor Green
            return $true
        } else {
            Write-Host "[ERROR] FAIL: Unexpected response" -ForegroundColor Red
            Write-Host "  Expected: $ExpectedResponse" -ForegroundColor Gray
            Write-Host "  Got: $($response.Content)" -ForegroundColor Gray
            return $false
        }
    } finally {
        # Clean up - kill the server
        if (-not $serverProcess.HasExited) {
            $serverProcess.Kill()
            Write-Host "[INFO] Server process terminated" -ForegroundColor Gray
        }
    }
}

# Run web server tests
$totalTests = 0
$passedTests = 0

# Test 1: simple_web_test.wfl
if (Test-Path "TestPrograms\simple_web_test.wfl") {
    $totalTests++
    $result = Test-WflWebServer -TestFile "TestPrograms\simple_web_test.wfl" -Port 8095 -ExpectedResponse "Hello from WFL" -TimeoutSeconds $Timeout
    if ($result) { $passedTests++ }
}

# Test 2: web_server_test.wfl (if exists)
if (Test-Path "TestPrograms\web_server_test.wfl") {
    $totalTests++
    # Read the file to find the port
    $content = Get-Content "TestPrograms\web_server_test.wfl" -Raw
    if ($content -match "port\s+(\d+)") {
        $port = [int]$Matches[1]
        $result = Test-WflWebServer -TestFile "TestPrograms\web_server_test.wfl" -Port $port -ExpectedResponse "" -TimeoutSeconds $Timeout
        if ($result) { $passedTests++ }
    } else {
        Write-Host "[SKIP] web_server_test.wfl - could not determine port" -ForegroundColor Yellow
        $totalTests--
    }
}

# Test 3: web_route_params_test.wfl (route parameter extraction)
if (Test-Path "TestPrograms\web_route_params_test.wfl") {
    $totalTests++
    Write-Host ""
    Write-Host "[INFO] Testing: web_route_params_test.wfl on port 8096" -ForegroundColor Blue

    $routeProcess = Start-Process -FilePath ".\$BinaryPath" -ArgumentList "TestPrograms\web_route_params_test.wfl" -NoNewWindow -PassThru -RedirectStandardOutput "NUL" -RedirectStandardError "NUL"

    try {
        $serverReady = $false
        $retries = 0
        $maxRetries = $Timeout * 2

        while (-not $serverReady -and $retries -lt $maxRetries) {
            Start-Sleep -Milliseconds 500
            $retries++
            try {
                $rootResponse = Invoke-WebRequest -Uri "http://localhost:8096/" -TimeoutSec 2 -UseBasicParsing -ErrorAction Stop
                if ($rootResponse.Content -like "*Route server ready*") {
                    $serverReady = $true
                }
            } catch {
                # Intentionally empty - server not ready yet, continue polling
            }
        }

        $routeOk = $true
        if (-not $serverReady) {
            Write-Host "[ERROR] TIMEOUT: Route params server did not start within ${Timeout}s" -ForegroundColor Red
            $routeOk = $false
        } else {
            # Route parameter extraction: /users/:id
            $userResponse = Invoke-WebRequest -Uri "http://localhost:8096/users/42" -TimeoutSec 2 -UseBasicParsing
            if ($userResponse.Content -like "*User 42*") {
                Write-Host "[SUCCESS] PASS: /users/42 -> '$($userResponse.Content)'" -ForegroundColor Green
            } else {
                Write-Host "[ERROR] FAIL: /users/42 returned '$($userResponse.Content)'" -ForegroundColor Red
                $routeOk = $false
            }

            # Non-matching route returns 404
            try {
                Invoke-WebRequest -Uri "http://localhost:8096/missing" -TimeoutSec 2 -UseBasicParsing -ErrorAction Stop | Out-Null
                Write-Host "[ERROR] FAIL: unknown route did not return 404" -ForegroundColor Red
                $routeOk = $false
            } catch {
                if ($_.Exception.Response -and [int]$_.Exception.Response.StatusCode -eq 404) {
                    Write-Host "[SUCCESS] PASS: unknown route returns 404" -ForegroundColor Green
                } else {
                    Write-Host "[ERROR] FAIL: unknown route error was not a 404" -ForegroundColor Red
                    $routeOk = $false
                }
            }

            # Header access regression
            $agentResponse = Invoke-WebRequest -Uri "http://localhost:8096/agent" -TimeoutSec 2 -UseBasicParsing -UserAgent "wfl-route-test"
            if ($agentResponse.Content -like "*wfl-route-test*") {
                Write-Host "[SUCCESS] PASS: header access echoes User-Agent" -ForegroundColor Green
            } else {
                Write-Host "[ERROR] FAIL: /agent returned '$($agentResponse.Content)'" -ForegroundColor Red
                $routeOk = $false
            }
        }

        if ($routeOk) { $passedTests++ }
    } finally {
        if (-not $routeProcess.HasExited) {
            $routeProcess.Kill()
            Write-Host "[INFO] Server process terminated" -ForegroundColor Gray
        }
    }
}

# Test 4: web_server_tls.wfl (HTTPS + HTTP->HTTPS redirect)
if (Test-Path "TestPrograms\web_server_tls.wfl") {
    $openssl = Get-Command openssl -ErrorAction SilentlyContinue
    if (-not $openssl) {
        Write-Host "[SKIP] web_server_tls.wfl - openssl not available to generate a test certificate" -ForegroundColor Yellow
    } else {
        $totalTests++
        Write-Host ""
        Write-Host "[INFO] Testing: web_server_tls.wfl on ports 8443 (https) and 8090 (redirect)" -ForegroundColor Blue

        $tlsDir = Join-Path ([System.IO.Path]::GetTempPath()) ([System.IO.Path]::GetRandomFileName())
        New-Item -ItemType Directory -Path $tlsDir | Out-Null
        & openssl req -x509 -newkey rsa:2048 -nodes -keyout "$tlsDir\key.pem" -out "$tlsDir\cert.pem" -days 1 -subj "/CN=localhost" 2>&1 | Out-Null

        # The test program uses relative cert paths, so run it from the temp dir
        $absBinary = Join-Path (Get-Location) $BinaryPath
        $absTest = Join-Path (Get-Location) "TestPrograms\web_server_tls.wfl"
        $tlsProcess = Start-Process -FilePath $absBinary -ArgumentList $absTest -WorkingDirectory $tlsDir -NoNewWindow -PassThru -RedirectStandardOutput "NUL" -RedirectStandardError "NUL"

        try {
            # Probe readiness via the redirect port: it answers natively and does
            # not consume the program's single `wait for request`
            $serverReady = $false
            $retries = 0
            $maxRetries = $Timeout * 2
            while (-not $serverReady -and $retries -lt $maxRetries) {
                Start-Sleep -Milliseconds 500
                $retries++
                try {
                    Invoke-WebRequest -Uri "http://localhost:8090/" -TimeoutSec 2 -UseBasicParsing -MaximumRedirection 0 -ErrorAction Stop | Out-Null
                    $serverReady = $true
                } catch {
                    if ($_.Exception.Response -and [int]$_.Exception.Response.StatusCode -eq 301) {
                        $serverReady = $true
                    }
                }
            }

            $tlsOk = $true
            if (-not $serverReady) {
                Write-Host "[ERROR] TIMEOUT: TLS server did not start within ${Timeout}s" -ForegroundColor Red
                $tlsOk = $false
            } else {
                # Redirect server: 301 with Location preserving path/query on the HTTPS port
                $location = $null
                try {
                    $redirectResponse = Invoke-WebRequest -Uri "http://localhost:8090/some/path?x=1" -TimeoutSec 2 -UseBasicParsing -MaximumRedirection 0 -ErrorAction Stop
                    $location = $redirectResponse.Headers["Location"]
                } catch {
                    if ($_.Exception.Response) {
                        $location = $_.Exception.Response.Headers["Location"]
                    }
                }
                if ($location -eq "https://localhost:8443/some/path?x=1") {
                    Write-Host "[SUCCESS] PASS: redirect returns 301 to $location" -ForegroundColor Green
                } else {
                    Write-Host "[ERROR] FAIL: redirect Location was '$location'" -ForegroundColor Red
                    $tlsOk = $false
                }

                # HTTPS request with certificate validation disabled (self-signed)
                try {
                    $httpsResponse = Invoke-WebRequest -Uri "https://localhost:8443/" -TimeoutSec 3 -UseBasicParsing -SkipCertificateCheck -ErrorAction Stop
                    if ($httpsResponse.Content -like "*Hello over HTTPS!*") {
                        Write-Host "[SUCCESS] PASS: HTTPS response '$($httpsResponse.Content)'" -ForegroundColor Green
                    } else {
                        Write-Host "[ERROR] FAIL: HTTPS request returned '$($httpsResponse.Content)'" -ForegroundColor Red
                        $tlsOk = $false
                    }
                } catch {
                    Write-Host "[ERROR] FAIL: HTTPS request failed: $_" -ForegroundColor Red
                    Write-Host "[INFO] Note: -SkipCertificateCheck requires PowerShell 6+" -ForegroundColor Gray
                    $tlsOk = $false
                }
            }

            if ($tlsOk) { $passedTests++ }
        } finally {
            if (-not $tlsProcess.HasExited) {
                $tlsProcess.Kill()
                Write-Host "[INFO] Server process terminated" -ForegroundColor Gray
            }
            Remove-Item -Recurse -Force $tlsDir -ErrorAction SilentlyContinue
        }
    }
}

# Summary
Write-Host ""
Write-Host "[INFO] ============================" -ForegroundColor Blue
Write-Host "[INFO] Results: $passedTests/$totalTests tests passed" -ForegroundColor Blue

if ($passedTests -eq $totalTests) {
    Write-Host "[SUCCESS] All web server tests passed!" -ForegroundColor Green
    exit 0
} else {
    Write-Host "[ERROR] Some web server tests failed" -ForegroundColor Red
    exit 1
}
