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

# Dump a server's captured stdout/stderr on failure. Without this a genuine
# server error (a panic, a bind failure, a bad response) is invisible behind the
# runner's generic TIMEOUT/assertion message, since the process output is
# redirected to files. Call on every failure path before returning.
function Show-ServerLogs {
    param(
        [string]$OutLog,
        [string]$ErrLog,
        $Process
    )
    if ($Process) {
        if ($Process.HasExited) {
            Write-Host "[LOG] server process exited with code $($Process.ExitCode)" -ForegroundColor Gray
        } else {
            Write-Host "[LOG] server process was still running at failure time" -ForegroundColor Gray
        }
    }
    foreach ($pair in @(@("stdout", $OutLog), @("stderr", $ErrLog))) {
        $label = $pair[0]
        $path = $pair[1]
        if ($path -and (Test-Path $path)) {
            $content = (Get-Content -Raw -ErrorAction SilentlyContinue $path)
            if ([string]::IsNullOrWhiteSpace($content)) {
                Write-Host "[LOG] server $label ($path): <empty>" -ForegroundColor Gray
            } else {
                Write-Host "[LOG] server $label ($path):" -ForegroundColor Gray
                Write-Host $content -ForegroundColor Gray
            }
        }
    }
}

# Kill a background server process and wait for it to actually exit, so any temp
# files/handles it holds are released before the caller removes them (avoids
# Windows cleanup races on the TLS temp dir).
function Stop-ServerProcess {
    param($Process)
    if ($Process -and -not $Process.HasExited) {
        # Kill() itself can throw (e.g. access denied, or the process exiting
        # concurrently), so guard it too rather than leaving it outside the try.
        try {
            $Process.Kill()
        } catch {
            Write-Host "[WARN] Kill() on server process failed: $_" -ForegroundColor Yellow
        }
        # WaitForExit(ms) returns $true only if the process actually exited in
        # time; report honestly rather than always claiming success (a process
        # still alive can race temp-file/cert cleanup that follows). Callers that
        # then delete temp files re-check HasExited before doing so.
        $exited = $false
        try { $exited = $Process.WaitForExit(5000) } catch { }
        if ($exited) {
            Write-Host "[INFO] Server process terminated" -ForegroundColor Gray
        } else {
            Write-Host "[WARN] Server process did not exit within 5s of Kill()" -ForegroundColor Yellow
        }
    }
}

# Extract the Location header from a redirect response across PowerShell/response
# shapes. With -MaximumRedirection 0, pwsh 7 throws and $_.Exception.Response is
# an HttpResponseMessage whose Headers has NO string indexer, so ["Location"]
# silently returns $null; its Location is the strongly-typed .Headers.Location
# (a Uri). Invoke-WebRequest's own response object (and Windows PowerShell 5.1's
# HttpWebResponse) use a string indexer instead, whose value may be a string[].
function Get-LocationHeader {
    param($Response)
    if ($null -eq $Response) { return $null }
    # Match the HttpResponseMessage shape by type NAME, not the type literal
    # [System.Net.Http.HttpResponseMessage]: a clean Windows PowerShell 5.1
    # process may not have System.Net.Http loaded, so resolving the literal would
    # throw before the HttpWebResponse string-indexer fallback below.
    if ($Response.GetType().FullName -eq 'System.Net.Http.HttpResponseMessage') {
        if ($Response.Headers.Location) { return $Response.Headers.Location.AbsoluteUri }
        return $null
    }
    try {
        $loc = $Response.Headers["Location"]
        if ($loc -is [array]) { $loc = $loc[0] }
        if ($loc) { return [string]$loc }
    } catch { }
    return $null
}

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

    # Start the WFL server in background. Start-Process rejects the same target
    # for both redirects (the "NUL"/"NUL" collision errored on PowerShell 7), so
    # discard stdout and stderr to two distinct, port-keyed temp files.
    $outLog = Join-Path ([System.IO.Path]::GetTempPath()) "wfl_web_$Port.out.log"
    $errLog = Join-Path ([System.IO.Path]::GetTempPath()) "wfl_web_$Port.err.log"
    $serverProcess = Start-Process -FilePath ".\$BinaryPath" -ArgumentList $TestFile -NoNewWindow -PassThru -RedirectStandardOutput $outLog -RedirectStandardError $errLog

    try {
        # Wait for the server to start, bounded by a real wall-clock deadline.
        # A fixed retry count misleads: each failed 2s request stacks on top of
        # the 500ms sleep, so "20 tries" of a 10s wait could actually run ~50s.
        # A Stopwatch caps total wait at TimeoutSeconds (plus one in-flight probe).
        $serverReady = $false
        $deadline = [System.Diagnostics.Stopwatch]::StartNew()
        while (-not $serverReady -and $deadline.Elapsed.TotalSeconds -lt $TimeoutSeconds) {
            try {
                $response = Invoke-WebRequest -Uri "http://127.0.0.1:$Port/" -TimeoutSec 2 -UseBasicParsing -ErrorAction Stop
                $serverReady = $true
            } catch {
                # Server not ready yet, wait briefly and retry until the deadline.
                Start-Sleep -Milliseconds 500
            }
        }

        if (-not $serverReady) {
            Write-Host "[ERROR] TIMEOUT: Server did not start within ${TimeoutSeconds}s" -ForegroundColor Red
            Show-ServerLogs -OutLog $outLog -ErrLog $errLog -Process $serverProcess
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
            Show-ServerLogs -OutLog $outLog -ErrLog $errLog -Process $serverProcess
            return $false
        }
    } finally {
        # Clean up - kill the server and wait for exit
        if (-not $serverProcess.HasExited) {
            Stop-ServerProcess -Process $serverProcess
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

    # Distinct redirect targets (see the note in Test-WflWebServer): the same
    # path for both streams errors on PowerShell 7.
    $routeOutLog = Join-Path ([System.IO.Path]::GetTempPath()) "wfl_web_route.out.log"
    $routeErrLog = Join-Path ([System.IO.Path]::GetTempPath()) "wfl_web_route.err.log"
    $routeProcess = Start-Process -FilePath ".\$BinaryPath" -ArgumentList "TestPrograms\web_route_params_test.wfl" -NoNewWindow -PassThru -RedirectStandardOutput $routeOutLog -RedirectStandardError $routeErrLog

    try {
        # Wall-clock deadline (see Test-WflWebServer) instead of a retry count.
        $serverReady = $false
        $deadline = [System.Diagnostics.Stopwatch]::StartNew()
        while (-not $serverReady -and $deadline.Elapsed.TotalSeconds -lt $Timeout) {
            try {
                $rootResponse = Invoke-WebRequest -Uri "http://127.0.0.1:8096/" -TimeoutSec 2 -UseBasicParsing -ErrorAction Stop
                if ($rootResponse.Content -like "*Route server ready*") {
                    $serverReady = $true
                }
            } catch {
                # Intentionally empty - server not ready yet, continue polling
            }
            if (-not $serverReady) { Start-Sleep -Milliseconds 500 }
        }

        $routeOk = $true
        if (-not $serverReady) {
            Write-Host "[ERROR] TIMEOUT: Route params server did not start within ${Timeout}s" -ForegroundColor Red
            $routeOk = $false
        } else {
            # Route parameter extraction: /users/:id. Wrapped so a request
            # failure marks the test failed (and dumps server logs below) instead
            # of throwing out of the script and skipping the summary/other tests.
            try {
                $userResponse = Invoke-WebRequest -Uri "http://127.0.0.1:8096/users/42" -TimeoutSec 2 -UseBasicParsing -ErrorAction Stop
                if ($userResponse.Content -like "*User 42*") {
                    Write-Host "[SUCCESS] PASS: /users/42 -> '$($userResponse.Content)'" -ForegroundColor Green
                } else {
                    Write-Host "[ERROR] FAIL: /users/42 returned '$($userResponse.Content)'" -ForegroundColor Red
                    $routeOk = $false
                }
            } catch {
                Write-Host "[ERROR] FAIL: /users/42 request failed: $_" -ForegroundColor Red
                $routeOk = $false
            }

            # Non-matching route returns 404
            try {
                Invoke-WebRequest -Uri "http://127.0.0.1:8096/missing" -TimeoutSec 2 -UseBasicParsing -ErrorAction Stop | Out-Null
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
            try {
                $agentResponse = Invoke-WebRequest -Uri "http://127.0.0.1:8096/agent" -TimeoutSec 2 -UseBasicParsing -UserAgent "wfl-route-test" -ErrorAction Stop
                if ($agentResponse.Content -like "*wfl-route-test*") {
                    Write-Host "[SUCCESS] PASS: header access echoes User-Agent" -ForegroundColor Green
                } else {
                    Write-Host "[ERROR] FAIL: /agent returned '$($agentResponse.Content)'" -ForegroundColor Red
                    $routeOk = $false
                }
            } catch {
                Write-Host "[ERROR] FAIL: /agent request failed: $_" -ForegroundColor Red
                $routeOk = $false
            }
        }

        if ($routeOk) {
            $passedTests++
        } else {
            Show-ServerLogs -OutLog $routeOutLog -ErrLog $routeErrLog -Process $routeProcess
        }
    } finally {
        Stop-ServerProcess -Process $routeProcess
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
        # Distinct redirect targets under the (auto-cleaned) temp dir; the same
        # path for both streams errors on PowerShell 7.
        $tlsProcess = Start-Process -FilePath $absBinary -ArgumentList $absTest -WorkingDirectory $tlsDir -NoNewWindow -PassThru -RedirectStandardOutput (Join-Path $tlsDir "server.out.log") -RedirectStandardError (Join-Path $tlsDir "server.err.log")

        try {
            # Probe readiness via the redirect port: it answers natively and does
            # not consume the program's single `wait for request`
            # Wall-clock deadline (see Test-WflWebServer) instead of a retry count.
            $serverReady = $false
            $deadline = [System.Diagnostics.Stopwatch]::StartNew()
            while (-not $serverReady -and $deadline.Elapsed.TotalSeconds -lt $Timeout) {
                try {
                    Invoke-WebRequest -Uri "http://127.0.0.1:8090/" -TimeoutSec 2 -UseBasicParsing -MaximumRedirection 0 -ErrorAction Stop | Out-Null
                    $serverReady = $true
                } catch {
                    if ($_.Exception.Response -and [int]$_.Exception.Response.StatusCode -eq 301) {
                        $serverReady = $true
                    }
                }
                if (-not $serverReady) { Start-Sleep -Milliseconds 500 }
            }

            $tlsOk = $true
            if (-not $serverReady) {
                Write-Host "[ERROR] TIMEOUT: TLS server did not start within ${Timeout}s" -ForegroundColor Red
                $tlsOk = $false
            } else {
                # Redirect server: 301 with Location preserving path/query on the HTTPS port
                $location = $null
                try {
                    $redirectResponse = Invoke-WebRequest -Uri "http://127.0.0.1:8090/some/path?x=1" -TimeoutSec 2 -UseBasicParsing -MaximumRedirection 0 -ErrorAction Stop
                    $location = Get-LocationHeader -Response $redirectResponse
                } catch {
                    if ($_.Exception.Response) {
                        $location = Get-LocationHeader -Response $_.Exception.Response
                    }
                }
                if ($location -eq "https://127.0.0.1:8443/some/path?x=1") {
                    Write-Host "[SUCCESS] PASS: redirect returns 301 to $location" -ForegroundColor Green
                } else {
                    Write-Host "[ERROR] FAIL: redirect Location was '$location'" -ForegroundColor Red
                    $tlsOk = $false
                }

                # HTTPS request with certificate validation disabled (self-signed).
                # -SkipCertificateCheck only exists in PowerShell 6+; on Windows
                # PowerShell 5.1 skip this check gracefully instead of failing.
                if ($PSVersionTable.PSVersion.Major -ge 6) {
                    try {
                        $httpsResponse = Invoke-WebRequest -Uri "https://127.0.0.1:8443/" -TimeoutSec 3 -UseBasicParsing -SkipCertificateCheck -ErrorAction Stop
                        if ($httpsResponse.Content -like "*Hello over HTTPS!*") {
                            Write-Host "[SUCCESS] PASS: HTTPS response '$($httpsResponse.Content)'" -ForegroundColor Green
                        } else {
                            Write-Host "[ERROR] FAIL: HTTPS request returned '$($httpsResponse.Content)'" -ForegroundColor Red
                            $tlsOk = $false
                        }
                    } catch {
                        Write-Host "[ERROR] FAIL: HTTPS request failed: $_" -ForegroundColor Red
                        $tlsOk = $false
                    }
                } else {
                    Write-Host "[SKIP] HTTPS request check requires PowerShell 6+ (-SkipCertificateCheck); redirect check still ran" -ForegroundColor Yellow
                }
            }

            if ($tlsOk) {
                $passedTests++
            } else {
                Show-ServerLogs -OutLog (Join-Path $tlsDir "server.out.log") -ErrLog (Join-Path $tlsDir "server.err.log") -Process $tlsProcess
            }
        } finally {
            # Kill AND wait for exit before removing the temp dir, so the server
            # has released its cert/log file handles (avoids a Windows cleanup
            # race that would leave the dir or fail the Remove-Item).
            Stop-ServerProcess -Process $tlsProcess
            if ($tlsProcess -and -not $tlsProcess.HasExited) {
                # Still alive after Kill()+WaitForExit — deleting now can race the
                # process's open cert/log handles. Surface it instead of hiding a
                # failed cleanup behind SilentlyContinue.
                Write-Host "[WARN] TLS server still running; temp dir cleanup may be incomplete" -ForegroundColor Yellow
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
