# Installing WFL

Get WFL running on your system in just a few minutes. Choose the installation method that works best for you.

## Installation Methods

- **[Windows MSI Installer](#windows-msi-installer)** - Easiest for Windows users
- **[Linux x86_64 Tarball](#linux-x86_64-tarball)** - Prebuilt, no dependencies to install
- **[From Source](#from-source)** - Cross-platform, latest features
- **[Verify Installation](#verify-installation)** - Make sure it works

## Windows MSI Installer

**Recommended for Windows users.** The MSI installer provides the easiest setup with optional components.

### Step 1: Download the Installer

Download the latest WFL MSI installer from GitHub Releases:

**[Download WFL MSI →](https://github.com/WebFirstLanguage/wfl/releases/latest)**

Look for the file named `wfl-<version>.msi` (e.g., `wfl-26.1.17.msi`)

### Step 2: Run the Installer

1. **Double-click** the downloaded `.msi` file
2. **Accept** the license agreement (Apache 2.0)
3. **Select components** you want to install:
   - **WFL Core** (required) - The WFL compiler and runtime
   - **LSP Server** (optional) - Language Server for editor integration
   - **VS Code Extension** (optional) - Syntax highlighting and IDE features

4. **Choose installation directory** (default: `C:\Program Files\WFL\`)
5. **Click Install**

### Step 3: Automatic PATH Setup

The installer automatically adds WFL to your PATH. This means you can run `wfl` from any command prompt.

**No manual configuration needed!**

### Step 4: Verify Installation

Open a **new** Command Prompt or PowerShell window:

```powershell
wfl --version
```

**Expected output:**
```
WebFirst Language (WFL) version 26.1.17
```

If you see this, congratulations! WFL is installed.

### What Gets Installed

- **WFL Compiler**: `C:\Program Files\WFL\bin\wfl.exe`
- **LSP Server** (if selected): `C:\Program Files\WFL\bin\wfl-lsp.exe`
- **VS Code Extension** (if selected): Automatically installed to VS Code
- **Documentation**: `C:\Program Files\WFL\docs\`

### Updating WFL

To update to a newer version:
1. Download the latest MSI
2. Run the installer
3. Choose "Upgrade" when prompted

Your existing WFL code will continue to work (backward compatibility guarantee).

---

## Linux x86_64 Tarball

**Recommended for Linux users on x86_64.** The tarball ships prebuilt `wfl` and
`wfl-lsp` binaries, so there is nothing to compile and no runtime to install.

The binaries are statically linked against musl, which means they have **no glibc
requirement** and run on any x86_64 Linux distribution, including older ones like
Debian 12 and Ubuntu 22.04. Every nightly build verifies this by running the
binaries inside a `debian:12-slim` container before publishing them.

### Step 1: Download and Verify

The canonical, CDN-backed download location is
<https://wfl.nyc3.cdn.digitaloceanspaces.com/releases/>. The GitHub Release is a
mirror of the same files.

```bash
BASE=https://wfl.nyc3.cdn.digitaloceanspaces.com/releases
curl -fLO "$BASE/wfl-latest-linux-x86_64.tar.gz"
curl -fLO "$BASE/SHA256SUMS"
```

`wfl-latest-linux-x86_64.tar.gz` always points at the most recent build. To pin a
specific build instead, download the versioned name shown in `SHA256SUMS`
(`wfl-<version>-linux-x86_64-<short-sha>.tar.gz`); those objects are immutable.

Check the download against the published checksums:

```bash
# The rolling "latest" tarball is byte-identical to the versioned one it mirrors,
# so compare its hash against the Linux entry in SHA256SUMS.
sha256sum wfl-latest-linux-x86_64.tar.gz
grep linux-x86_64 SHA256SUMS
```

The two hashes must match. If they do not, do not install the archive.

#### Verifying a pinned version

**`SHA256SUMS` describes only the most recent publish.** It is rewritten by every
nightly, so it is the right file to check `latest` against and the wrong file to
check a pinned version against — the entry you pinned disappears from it as soon
as a newer build lands, even though your tarball is immutable and still served.

Every versioned artifact therefore also has its own checksum file, published once
alongside it and never rewritten:

```bash
BASE=https://wfl.nyc3.cdn.digitaloceanspaces.com/releases
TARBALL=wfl-26.7.59-linux-x86_64-579eb80.tar.gz   # the build you pinned

curl -fLO "$BASE/$TARBALL"
curl -fLO "$BASE/$TARBALL.sha256"
sha256sum -c "$TARBALL.sha256"
```

`sha256sum -c` prints `OK` and exits 0 on a match, so this drops straight into a
deployment script. The same `<artifact>.sha256` naming applies to the MSI and the
VS Code extension (`wfl-<version>.msi.sha256`,
`vscode-wfl-<version>.vsix.sha256`).

Both files are served from the same host as the artifact, so they prove the
download arrived intact — not that it came from us. Signed installers are a
tracked follow-up; until then, treat the checksum as an integrity check, and
record the expected hash yourself if you need to detect a change at the source.

### Step 2: Extract

```bash
tar xzf wfl-latest-linux-x86_64.tar.gz
```

This creates a `wfl-<version>-linux-x86_64/` directory containing:

- `wfl` - the WFL compiler and runtime
- `wfl-lsp` - the Language Server, for editor integration
- `README.md`, `LICENSE`
- `BUILD_INFO` - version, commit, build time, and target triple

### Step 3: Install

Install both binaries somewhere on your `PATH`:

```bash
sudo install -m 755 wfl-*-linux-x86_64/wfl     /usr/local/bin/wfl
sudo install -m 755 wfl-*-linux-x86_64/wfl-lsp /usr/local/bin/wfl-lsp
```

Prefer a per-user install? Use `~/.local/bin` instead of `/usr/local/bin` and
drop the `sudo` (make sure `~/.local/bin` is on your `PATH`).

### Step 4: Verify Installation

```bash
wfl --version
wfl-lsp --version
```

### Updating WFL

Repeat Steps 1-3 with the current tarball; `install` overwrites the previous
binaries in place. Your existing WFL code will continue to work (backward
compatibility guarantee).

> **Other platforms and architectures:** Linux on `aarch64`, macOS, and non-x86_64
> musl targets have no prebuilt artifact yet - build [from
> source](#from-source). See
> [`supported-platforms.md`](../reference/supported-platforms.md) for the current
> support tiers.

---

## From Source

**Cross-platform installation.** Works on Windows, Linux, and macOS.

### Prerequisites

You'll need:
- **Rust** 1.75 or later ([Install Rust](https://rustup.rs/))
- **Git** ([Install Git](https://git-scm.com/downloads))
- **Cargo** (comes with Rust)

Check if you have Rust installed:

```bash
rustc --version
```

You should see version 1.75 or higher.

### Step 1: Clone the Repository

```bash
git clone https://github.com/WebFirstLanguage/wfl.git
cd wfl
```

### Step 2: Build WFL

Build the release version (optimized):

```bash
cargo build --release
```

**This will take a few minutes** (5-10 minutes depending on your machine). Rust is compiling WFL and all its dependencies.

**Expected output:**
```
   Compiling wfl v26.1.17
   ...
   Finished release [optimized] target(s) in 8m 32s
```

### Step 3: Locate the Binary

The WFL binary is now at:

- **Windows**: `target\release\wfl.exe`
- **Linux/macOS**: `target/release/wfl`

### Step 4: Add to PATH (Optional but Recommended)

To run `wfl` from anywhere, add it to your PATH:

#### Windows (PowerShell)
```powershell
$env:Path += ";$(Get-Location)\target\release"
# Make it permanent:
[Environment]::SetEnvironmentVariable("Path", $env:Path, [EnvironmentVariableTarget]::User)
```

#### Linux/macOS (Bash)
```bash
export PATH="$PATH:$(pwd)/target/release"
# Make it permanent (add to ~/.bashrc or ~/.zshrc):
echo 'export PATH="$PATH:'$(pwd)'/target/release"' >> ~/.bashrc
source ~/.bashrc
```

### Step 5: Verify Installation

```bash
wfl --version
```

**Expected output:**
```
WebFirst Language (WFL) version 26.1.17
```

Success! You're ready to code.

### Building the LSP Server (Optional)

For editor integration, also build the LSP server:

```bash
cargo build --release -p wfl-lsp
```

The LSP server will be at:
- **Windows**: `target\release\wfl-lsp.exe`
- **Linux/macOS**: `target/release/wfl-lsp`

### Updating WFL (From Source)

To get the latest version:

```bash
cd wfl
git pull origin main
cargo build --release
```

Your existing WFL code will continue to work.

---

## Verify Installation

Let's make sure everything is working correctly.

### Check Version

```bash
wfl --version
```

You should see version information like:
```
WebFirst Language (WFL) version 26.1.17
```

### Test with Hello World

Create a file called `test.wfl`:

```wfl
display "WFL is installed and working!"
```

Run it:

```bash
wfl test.wfl
```

**Expected output:**
```
WFL is installed and working!
```

**Congratulations!** 🎉 WFL is successfully installed.

### Check Available Commands

See what WFL can do:

```bash
wfl --help
```

**Common commands:**
- `wfl <file>` - Run a WFL program
- `wfl` - Start interactive REPL
- `wfl --lint <file>` - Check code style
- `wfl --analyze <file>` - Static analysis
- `wfl --parse <file>` - Check syntax
- `wfl --version` - Show version

---

## Troubleshooting

### "Command not found" or "wfl is not recognized"

**Problem:** Your shell can't find the `wfl` command.

**Solution (Windows):**
1. Close and reopen your terminal
2. Check if WFL is in PATH: `echo %PATH%`
3. Look for WFL's directory in the output
4. If not there, add it manually or reinstall

**Solution (Linux/macOS):**
1. Check PATH: `echo $PATH`
2. Make sure you added WFL to PATH (see Step 4 above)
3. Run `source ~/.bashrc` (or `~/.zshrc`) to reload

**Alternative:** Run WFL with full path:
```bash
# Windows
C:\path\to\wfl\target\release\wfl.exe test.wfl

# Linux/macOS
/path/to/wfl/target/release/wfl test.wfl
```

### Build Fails: "cargo: command not found"

**Problem:** Rust/Cargo is not installed.

**Solution:** Install Rust from [https://rustup.rs/](https://rustup.rs/)

Then restart your terminal and try again.

### Build Fails: Compilation Errors

**Problem:** Rust version is too old.

**Solution:** Update Rust:
```bash
rustup update
```

You need Rust 1.75 or later.

### MSI Installer: "Windows protected your PC"

**Problem:** Windows SmartScreen warning on unsigned software.

**Solution:**
1. Click "More info"
2. Click "Run anyway"

(WFL is safe, but we don't yet have a code signing certificate for the MSI.)

### Permission Denied (Linux/macOS)

**Problem:** Can't execute the binary.

**Solution:** Make it executable:
```bash
chmod +x target/release/wfl
```

### Slow Build Times

**Problem:** Building from source takes a long time.

**This is normal.** Rust compiles everything from scratch. First build is slowest (8-15 minutes). Subsequent builds are much faster (1-2 minutes).

**Tips:**
- Use `cargo build --release` (faster runtime, slower build)
- Use `cargo build` for development (faster build, slower runtime)
- Be patient—it's worth it!

---

## Next Steps

Now that WFL is installed, let's write your first program!

**[Write "Hello, World!" →](hello-world.md)**

Or explore other options:
- **[Your First Program](your-first-program.md)** - Interactive tutorial
- **[REPL Guide](repl-guide.md)** - Experiment with WFL interactively
- **[Editor Setup](editor-setup.md)** - Get VS Code integration working

---

## Installation Summary

**Windows Users:**
- ✅ Download MSI installer
- ✅ Run installer, select components
- ✅ Automatic PATH setup
- ✅ Ready to code

**All Platforms (From Source):**
- ✅ Install Rust
- ✅ Clone repository
- ✅ `cargo build --release`
- ✅ Add to PATH
- ✅ Ready to code

**Verification:**
```bash
wfl --version
wfl test.wfl
```

Welcome to WFL! 🎉

---

**Previous:** [← Getting Started](index.md) | **Next:** [Hello World →](hello-world.md)
