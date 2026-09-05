#!/bin/bash
# WFL Web Server Integration Test Runner
# Tests WFL web server functionality by starting servers and sending HTTP requests

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
GRAY='\033[0;90m'
NC='\033[0m' # No Color

# Default timeout
TIMEOUT=10

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --help|-h)
            echo "WFL Web Server Test Runner"
            echo ""
            echo "Usage: $0 [options]"
            echo ""
            echo "Options:"
            echo "  --help, -h       Show this help message"
            echo "  --timeout <sec>  Timeout for each test (default: 10)"
            echo ""
            echo "This script tests WFL web server functionality by:"
            echo "  1. Starting the WFL web server in background"
            echo "  2. Sending HTTP requests to verify it works"
            echo "  3. Checking responses and cleaning up"
            exit 0
            ;;
        --timeout)
            TIMEOUT="$2"
            shift 2
            ;;
        *)
            echo -e "${RED}[ERROR]${NC} Unknown option: $1"
            exit 1
            ;;
    esac
done

echo -e "${BLUE}[INFO]${NC} WFL Web Server Test Runner"
echo -e "${BLUE}[INFO]${NC} ============================"

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}[ERROR]${NC} Cargo.toml not found. Please run this script from the WFL project root."
    exit 1
fi

# Determine binary path
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
    BINARY_PATH="target/release/wfl.exe"
else
    BINARY_PATH="target/release/wfl"
fi

# Check if release binary exists
if [ ! -f "$BINARY_PATH" ]; then
    echo -e "${RED}[ERROR]${NC} Release binary not found. Run 'cargo build --release' first."
    exit 1
fi
echo -e "${GREEN}[SUCCESS]${NC} Binary found: $BINARY_PATH"

# Function to test a web server
test_wfl_webserver() {
    local test_file="$1"
    local port="$2"
    local expected_response="$3"
    local timeout_seconds="$4"

    local test_name=$(basename "$test_file")
    echo ""
    echo -e "${BLUE}[INFO]${NC} Testing: $test_name on port $port"

    # Start the WFL server in background
    "./$BINARY_PATH" "$test_file" > /dev/null 2>&1 &
    local server_pid=$!

    # Cleanup function
    cleanup() {
        if kill -0 $server_pid 2>/dev/null; then
            kill $server_pid 2>/dev/null || true
            echo -e "${GRAY}[INFO] Server process terminated${NC}"
        fi
    }
    trap cleanup EXIT

    # Wait for server to start (with retries)
    local server_ready=false
    local retries=0
    local max_retries=$((timeout_seconds * 2))  # Check every 500ms

    while [ "$server_ready" = false ] && [ $retries -lt $max_retries ]; do
        sleep 0.5
        retries=$((retries + 1))

        # Try to connect using curl
        if response=$(curl -s --max-time 2 "http://localhost:$port/" 2>/dev/null); then
            server_ready=true
        fi
    done

    if [ "$server_ready" = false ]; then
        echo -e "${RED}[ERROR]${NC} TIMEOUT: Server did not start within ${timeout_seconds}s"
        cleanup
        return 1
    fi

    # Server is ready, check response
    if [[ "$response" == *"$expected_response"* ]]; then
        echo -e "${GREEN}[SUCCESS]${NC} PASS: Got expected response"
        cleanup
        return 0
    else
        echo -e "${RED}[ERROR]${NC} FAIL: Unexpected response"
        echo -e "${GRAY}  Expected: $expected_response${NC}"
        echo -e "${GRAY}  Got: $response${NC}"
        cleanup
        return 1
    fi
}

# Run web server tests
total_tests=0
passed_tests=0

# Test 1: simple_web_test.wfl
if [ -f "TestPrograms/simple_web_test.wfl" ]; then
    total_tests=$((total_tests + 1))
    if test_wfl_webserver "TestPrograms/simple_web_test.wfl" 8095 "Hello from WFL" "$TIMEOUT"; then
        passed_tests=$((passed_tests + 1))
    fi
fi

# Test 2: web_server_test.wfl (if exists)
if [ -f "TestPrograms/web_server_test.wfl" ]; then
    total_tests=$((total_tests + 1))
    # Read the file to find the port
    port=$(grep -oP 'port\s+\K\d+' "TestPrograms/web_server_test.wfl" 2>/dev/null || echo "")
    if [ -n "$port" ]; then
        if test_wfl_webserver "TestPrograms/web_server_test.wfl" "$port" "" "$TIMEOUT"; then
            passed_tests=$((passed_tests + 1))
        fi
    else
        echo -e "${YELLOW}[SKIP]${NC} web_server_test.wfl - could not determine port"
        total_tests=$((total_tests - 1))
    fi
fi

# Test 3: web_route_params_test.wfl (route parameter extraction)
if [ -f "TestPrograms/web_route_params_test.wfl" ]; then
    total_tests=$((total_tests + 1))
    echo ""
    echo -e "${BLUE}[INFO]${NC} Testing: web_route_params_test.wfl on port 8096"

    "./$BINARY_PATH" "TestPrograms/web_route_params_test.wfl" > /dev/null 2>&1 &
    route_pid=$!

    route_ready=false
    retries=0
    max_retries=$((TIMEOUT * 2))
    while [ "$route_ready" = false ] && [ $retries -lt $max_retries ]; do
        sleep 0.5
        retries=$((retries + 1))
        if curl -s --max-time 2 "http://localhost:8096/" 2>/dev/null | grep -q "Route server ready"; then
            route_ready=true
        fi
    done

    route_ok=true
    if [ "$route_ready" = false ]; then
        echo -e "${RED}[ERROR]${NC} TIMEOUT: Route params server did not start within ${TIMEOUT}s"
        route_ok=false
    else
        # Route parameter extraction: /users/:id
        user_resp=$(curl -s --max-time 2 "http://localhost:8096/users/42")
        if [[ "$user_resp" == *"User 42"* ]]; then
            echo -e "${GREEN}[SUCCESS]${NC} PASS: /users/42 -> '$user_resp'"
        else
            echo -e "${RED}[ERROR]${NC} FAIL: /users/42 returned '$user_resp' (expected 'User 42')"
            route_ok=false
        fi

        # Percent-decoded captures
        enc_resp=$(curl -s --max-time 2 "http://localhost:8096/users/John%20Doe")
        if [[ "$enc_resp" == *"User John Doe"* ]]; then
            echo -e "${GREEN}[SUCCESS]${NC} PASS: percent-decoded capture"
        else
            echo -e "${RED}[ERROR]${NC} FAIL: /users/John%20Doe returned '$enc_resp'"
            route_ok=false
        fi

        # Non-matching route returns 404
        notfound_code=$(curl -s -o /dev/null -w '%{http_code}' --max-time 2 "http://localhost:8096/missing")
        if [ "$notfound_code" = "404" ]; then
            echo -e "${GREEN}[SUCCESS]${NC} PASS: unknown route returns 404"
        else
            echo -e "${RED}[ERROR]${NC} FAIL: unknown route returned HTTP $notfound_code (expected 404)"
            route_ok=false
        fi

        # Header access regression (FRAMEWORK_FINAL_REPORT)
        agent_resp=$(curl -s --max-time 2 -A "wfl-route-test" "http://localhost:8096/agent")
        if [[ "$agent_resp" == *"wfl-route-test"* ]]; then
            echo -e "${GREEN}[SUCCESS]${NC} PASS: header access echoes User-Agent"
        else
            echo -e "${RED}[ERROR]${NC} FAIL: /agent returned '$agent_resp'"
            route_ok=false
        fi

        # Request counter increments across requests (property mutation regression)
        counter_resp=$(curl -s --max-time 2 "http://localhost:8096/")
        if [[ "$counter_resp" == *"Route server ready - request"* ]]; then
            echo -e "${GREEN}[SUCCESS]${NC} PASS: request counter response '$counter_resp'"
        else
            echo -e "${RED}[ERROR]${NC} FAIL: counter route returned '$counter_resp'"
            route_ok=false
        fi
    fi

    if kill -0 $route_pid 2>/dev/null; then
        kill $route_pid 2>/dev/null || true
    fi

    if [ "$route_ok" = true ]; then
        passed_tests=$((passed_tests + 1))
    fi
fi

# Test 4: web_server_tls.wfl (HTTPS + HTTP->HTTPS redirect)
if [ -f "TestPrograms/web_server_tls.wfl" ]; then
    if ! command -v openssl >/dev/null 2>&1; then
        echo -e "${YELLOW}[SKIP]${NC} web_server_tls.wfl - openssl not available to generate a test certificate"
    else
        total_tests=$((total_tests + 1))
        echo ""
        echo -e "${BLUE}[INFO]${NC} Testing: web_server_tls.wfl on ports 8443 (https) and 8090 (redirect)"

        tls_dir=$(mktemp -d)
        openssl req -x509 -newkey rsa:2048 -nodes -keyout "$tls_dir/key.pem" \
            -out "$tls_dir/cert.pem" -days 1 -subj "/CN=localhost" >/dev/null 2>&1

        # The test program uses relative cert paths, so run it from the temp dir
        abs_binary="$(pwd)/$BINARY_PATH"
        abs_test="$(pwd)/TestPrograms/web_server_tls.wfl"
        (cd "$tls_dir" && "$abs_binary" "$abs_test" > /dev/null 2>&1) &
        tls_pid=$!

        # Probe readiness via the redirect port: it answers natively and does
        # not consume the program's single `wait for request`
        tls_ready=false
        retries=0
        max_retries=$((TIMEOUT * 2))
        while [ "$tls_ready" = false ] && [ $retries -lt $max_retries ]; do
            sleep 0.5
            retries=$((retries + 1))
            if curl -ks --max-time 2 -o /dev/null "http://localhost:8090/" 2>/dev/null; then
                tls_ready=true
            fi
        done

        tls_ok=true
        if [ "$tls_ready" = false ]; then
            echo -e "${RED}[ERROR]${NC} TIMEOUT: TLS server did not start within ${TIMEOUT}s"
            tls_ok=false
        else
            # Redirect server: 301 with Location preserving path/query on the HTTPS port
            redirect_out=$(curl -ks -o /dev/null -w '%{http_code} %{redirect_url}' --max-time 2 "http://localhost:8090/some/path?x=1")
            if [[ "$redirect_out" == "301 https://localhost:8443/some/path?x=1" ]]; then
                echo -e "${GREEN}[SUCCESS]${NC} PASS: redirect returns 301 to https://localhost:8443/some/path?x=1"
            else
                echo -e "${RED}[ERROR]${NC} FAIL: redirect returned '$redirect_out'"
                tls_ok=false
            fi

            # HTTPS request (consumes the program's single request, then it exits)
            https_resp=$(curl -ks --max-time 3 "https://localhost:8443/")
            if [[ "$https_resp" == *"Hello over HTTPS!"* ]]; then
                echo -e "${GREEN}[SUCCESS]${NC} PASS: HTTPS response '$https_resp'"
            else
                echo -e "${RED}[ERROR]${NC} FAIL: HTTPS request returned '$https_resp'"
                tls_ok=false
            fi
        fi

        if kill -0 $tls_pid 2>/dev/null; then
            kill $tls_pid 2>/dev/null || true
        fi
        rm -rf "$tls_dir"

        if [ "$tls_ok" = true ]; then
            passed_tests=$((passed_tests + 1))
        fi
    fi
fi

# Test 5: web_server_session_test.wfl (cookies, CSRF, storage KV)
if [ -f "TestPrograms/web_server_session_test.wfl" ]; then
    total_tests=$((total_tests + 1))
    echo ""
    echo -e "${BLUE}[INFO]${NC} Testing: web_server_session_test.wfl on port 8097"

    session_log=$(mktemp)
    "./$BINARY_PATH" "TestPrograms/web_server_session_test.wfl" > "$session_log" 2>&1 &
    session_pid=$!
    cookie_jar=$(mktemp)

    session_ok=true
    session_ready=false
    retries=0
    max_retries=$((TIMEOUT * 2))
    while [ "$session_ready" = false ] && [ $retries -lt $max_retries ]; do
        sleep 0.5
        retries=$((retries + 1))
        if curl -s --max-time 2 "http://127.0.0.1:8097/" 2>/dev/null | grep -q "Session server ready"; then
            session_ready=true
        fi
    done

    if [ "$session_ready" = false ]; then
        echo -e "${RED}[ERROR]${NC} TIMEOUT: Session server did not start within ${TIMEOUT}s"
        echo -e "${GRAY}  server log:${NC}"
        cat "$session_log" || true
        session_ok=false
    else
        login_headers=$(mktemp)
        login_body=$(curl -s --max-time 2 -D "$login_headers" -c "$cookie_jar" "http://127.0.0.1:8097/login")
        session_id=$(echo "$login_body" | awk '{print $1}')
        csrf_token=$(echo "$login_body" | awk '{print $2}')
        if [[ "$login_body" == *" "* ]] && grep -qi "Set-Cookie: wfl_sid=" "$login_headers" && grep -qi "HttpOnly" "$login_headers"; then
            echo -e "${GREEN}[SUCCESS]${NC} PASS: /login sets HttpOnly wfl_sid cookie"
        else
            echo -e "${RED}[ERROR]${NC} FAIL: /login body='$login_body' headers:"
            cat "$login_headers" || true
            session_ok=false
        fi

        profile_body=$(curl -s --max-time 2 -b "$cookie_jar" "http://127.0.0.1:8097/profile")
        if [ "$profile_body" = "user123" ]; then
            echo -e "${GREEN}[SUCCESS]${NC} PASS: /profile returns user_id from cookie"
        else
            echo -e "${RED}[ERROR]${NC} FAIL: /profile returned '$profile_body' (expected user123)"
            session_ok=false
        fi

        anon_code=$(curl -s -o /dev/null -w '%{http_code}' --max-time 2 "http://127.0.0.1:8097/profile")
        if [ "$anon_code" = "401" ]; then
            echo -e "${GREEN}[SUCCESS]${NC} PASS: /profile without cookie returns 401"
        else
            echo -e "${RED}[ERROR]${NC} FAIL: anonymous /profile returned HTTP $anon_code (expected 401)"
            session_ok=false
        fi

        csrf_miss=$(curl -s -o /dev/null -w '%{http_code}' --max-time 2 -b "$cookie_jar" "http://127.0.0.1:8097/secure")
        if [ "$csrf_miss" = "403" ]; then
            echo -e "${GREEN}[SUCCESS]${NC} PASS: /secure without CSRF header returns 403"
        else
            echo -e "${RED}[ERROR]${NC} FAIL: /secure without CSRF returned HTTP $csrf_miss (expected 403)"
            session_ok=false
        fi

        csrf_ok=$(curl -s --max-time 2 -b "$cookie_jar" -H "X-CSRF-Token: $csrf_token" "http://127.0.0.1:8097/secure")
        if [ "$csrf_ok" = "Access granted" ]; then
            echo -e "${GREEN}[SUCCESS]${NC} PASS: /secure accepts matching CSRF token"
        else
            echo -e "${RED}[ERROR]${NC} FAIL: /secure with CSRF returned '$csrf_ok'"
            session_ok=false
        fi

        admin_code=$(curl -s -o /dev/null -w '%{http_code}' --max-time 2 -b "$cookie_jar" "http://127.0.0.1:8097/admin")
        if [ "$admin_code" = "403" ]; then
            echo -e "${GREEN}[SUCCESS]${NC} PASS: /admin denies missing admin permission"
        else
            echo -e "${RED}[ERROR]${NC} FAIL: /admin returned HTTP $admin_code (expected 403)"
            session_ok=false
        fi

        stats_body=$(curl -s --max-time 2 "http://127.0.0.1:8097/stats")
        if [[ "$stats_body" == memory\ * ]]; then
            echo -e "${GREEN}[SUCCESS]${NC} PASS: /stats reports memory backend ($stats_body)"
        else
            echo -e "${RED}[ERROR]${NC} FAIL: /stats returned '$stats_body'"
            session_ok=false
        fi

        storage_body=$(curl -s --max-time 2 "http://127.0.0.1:8097/storage")
        if [ "$storage_body" = "stored test_value" ]; then
            echo -e "${GREEN}[SUCCESS]${NC} PASS: storage KV put/load/delete"
        else
            echo -e "${RED}[ERROR]${NC} FAIL: /storage returned '$storage_body'"
            session_ok=false
        fi

        logout_headers=$(mktemp)
        logout_body=$(curl -s --max-time 2 -D "$logout_headers" -b "$cookie_jar" -c "$cookie_jar" "http://127.0.0.1:8097/logout")
        if [ "$logout_body" = "logged_out" ] && grep -qi "Max-Age=0" "$logout_headers"; then
            echo -e "${GREEN}[SUCCESS]${NC} PASS: /logout clears session cookie"
        else
            echo -e "${RED}[ERROR]${NC} FAIL: /logout body='$logout_body' headers:"
            cat "$logout_headers" || true
            session_ok=false
        fi

        after_logout=$(curl -s -o /dev/null -w '%{http_code}' --max-time 2 -b "$cookie_jar" "http://127.0.0.1:8097/profile")
        if [ "$after_logout" = "401" ]; then
            echo -e "${GREEN}[SUCCESS]${NC} PASS: /profile after logout returns 401"
        else
            echo -e "${RED}[ERROR]${NC} FAIL: /profile after logout returned HTTP $after_logout"
            session_ok=false
        fi

        rm -f "$login_headers" "$logout_headers"
    fi

    if kill -0 $session_pid 2>/dev/null; then
        kill $session_pid 2>/dev/null || true
    fi
    rm -f "$cookie_jar" "$session_log"

    if [ "$session_ok" = true ]; then
        passed_tests=$((passed_tests + 1))
    fi
fi

# Summary
echo ""
echo -e "${BLUE}[INFO]${NC} ============================"
echo -e "${BLUE}[INFO]${NC} Results: $passed_tests/$total_tests tests passed"

if [ "$passed_tests" -eq "$total_tests" ]; then
    echo -e "${GREEN}[SUCCESS]${NC} All web server tests passed!"
    exit 0
else
    echo -e "${RED}[ERROR]${NC} Some web server tests failed"
    exit 1
fi
