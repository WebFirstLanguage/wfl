# Automated Versioning System for WFL (Year.Build Scheme)

## Overview

WFL is a Rust-based language, and we want an **automated versioning system** integrated into its development pipeline. The version format will use a *Year.Build* scheme (e.g., **2025.1**, **2025.2**, etc.), where the **build number increments with each official build** and **resets at the start of a new year**. We will achieve this with a Python version-bump script and a GitHub Actions workflow:

* **Python Script:** Manages the version number by reading/updating a metadata file and modifying the Rust source with the new version string.
* **GitHub Actions Workflow:** Runs the script on each build (e.g., on each push to the main branch or on release), then commits and tags the new version in a safe manner (avoiding infinite CI loops).
* **Rust Integration:** The version is stored as a constant in `wfl/src/version.rs` so that the WFL runtime or CLI can easily access and display it.

This plan ensures that each official build of WFL automatically gets a unique version and that the version is readily available in the code for debugging or user queries.

## 1. Version Metadata File (`.build_meta.json`)

To keep track of the build count across runs, use a JSON file in the repository (for example, named **`.build_meta.json`** in the project root). This file will persist the last recorded year and build number. On each build, the Python script will read and update this file.

**Example `.build_meta.json`:**

```json
{
  "year": 2024,
  "build": 7
}
```

* **Purpose:** Stores the last build’s year and sequence number.
* **Reset Mechanism:** If the current year differs from the stored `year`, the script will reset `build` to 1 and update `year` to the new year. Otherwise, it will increment the `build` number.
* **Version Derivation:** The version string is then derived as `"year.build"`, e.g., `"2025.1"` for the first build of 2025.

By keeping this file under version control (and updating it with each release), the build count persists across CI runs. It also allows developers to manually adjust or inspect version history if needed.

## 2. Python Version-Bump Script

Create a Python script (for example, `scripts/bump_version.py`) to automate the version update. This script will perform the following steps:

1. **Read the metadata:** Open `.build_meta.json` and load the JSON data to get the last recorded year and build number.
2. **Compute new version:** Determine the current year (e.g., using `datetime.now().year`) and decide whether to reset or increment the build number.

   * If the stored year != current year, set `build = 1` and update `year = current_year`.
   * If the stored year == current year, increment the `build` by 1.
3. **Update version string:** Format the new version as `f"{year}.{build}"` (e.g., `"2025.13"`).
4. **Write back metadata:** Update the `.build_meta.json` file with the new year and build number (so future runs have the updated baseline).
5. **Update Rust source:** Open the Rust version file (`wfl/src/version.rs`) and replace the version constant with the new version string.
6. **Commit changes (optional within script):** Use Git to commit the modified files and push. This can be done within the script using `subprocess` calls or left for the CI workflow to handle. If committing here, configure a dummy user (e.g., “GitHub Actions”) and include “\[skip ci]” in the commit message to prevent triggering a new CI run from this auto-commit. (GitHub Actions will skip runs triggered by commits containing `[skip ci]` in the message.)

Below is a **simplified Python script** illustrating these steps:

```python
import json, datetime, subprocess

# Step 1: Read existing version metadata
with open(".build_meta.json", "r") as f:
    meta = json.load(f)

current_year = datetime.datetime.now().year
build_num = meta.get("build", 0)
last_year = meta.get("year", current_year)

# Step 2: Compute new build number with annual reset
if current_year != last_year:
    # New year: reset build count
    build_num = 1
    meta["year"] = current_year
else:
    # Same year: increment build count
    build_num += 1
meta["build"] = build_num

# Step 3: Form the new version string (Year.Build)
new_version = f"{current_year}.{build_num}"
print(f"New version: {new_version}")

# Step 4: Update the metadata file
with open(".build_meta.json", "w") as f:
    json.dump(meta, f, indent=2)

# Step 5: Update the Rust version constant
version_file = "wfl/src/version.rs"
# Replace the version line in the Rust file (assuming the file contains only the version or it's easy to find)
new_contents = f'pub const VERSION: &str = "{new_version}";\n'
with open(version_file, "w") as vf:
    vf.write(new_contents)

# Step 6: Commit and push the changes (if running inside CI with proper permissions)
subprocess.run(["git", "config", "user.name", "github-actions"])
subprocess.run(["git", "config", "user.email", "github-actions@github.com"])
subprocess.run(["git", "add", ".build_meta.json", version_file])
commit_msg = f"Bump version to {new_version} [skip ci]"
subprocess.run(["git", "commit", "-m", commit_msg])
subprocess.run(["git", "push"])
```

**Notes on the script:**

* We configure Git with a bot username/email and commit the two files (.build\_meta.json and version.rs). The commit message includes **\[skip ci]** to avoid recursively triggering the workflow on this commit. This is a common practice for auto-commits made by CI to prevent infinite loops.
* The script prints the new version (useful for log output in the workflow).
* Ensure the script is executable or invoke it with `python3 scripts/bump_version.py` in the workflow.
* Place the script in your repository (e.g., under a `scripts/` directory) and check it into version control.

## 3. Rust Version Constant (`wfl/src/version.rs`)

Inside the Rust project, maintain a source file (e.g., **`wfl/src/version.rs`**) that houses the version constant. The Python script will update this file on each build. For example, after running the script, `version.rs` might look like:

```rust
// wfl/src/version.rs
pub const VERSION: &str = "2025.1";
```

This provides a single source of truth for the version within the Rust code. You can then **use this constant in the WFL runtime or CLI** to display the current version. For instance, if using a CLI argument parser (such as Clap or structopt), you could supply this `VERSION` constant to the parser so that running `wfl --version` prints the correct version. If not using a CLI library, simply printing `VERSION` in a `--version` flag handler or at startup is advisable. The key is that after each official build, any user running the WFL binary can check the version and see something like “WFL version 2025.7” (for example).

*Implementation detail:* If `version.rs` contains additional content beyond the version constant, the Python script should carefully replace only the line with `VERSION`. For simplicity, one can structure `version.rs` to contain just the version constant, or parse the file to find the `pub const VERSION` line and update it. Since we control this file, keeping it minimal (just the constant) makes updates straightforward.

## 4. GitHub Actions Workflow Configuration

Next, set up a GitHub Actions workflow to run the version-bump script and handle committing the changes. Create a workflow file (e.g., **`.github/workflows/versioning.yml`**) with contents similar to:

```yaml
name: Auto Version Bump

on:
  push:
    branches: [ main ]   # Run only on pushes to main (official builds)
  # You can add other triggers as needed, e.g., workflow_dispatch or release events.

jobs:
  bump-version:
    runs-on: ubuntu-latest
    steps:
      - name: Check out code
        uses: actions/checkout@v3
        with:
          persist-credentials: true   # Ensure the checkout preserves the token for pushing

      - name: Set up Python
        uses: actions/setup-python@v4
        with:
          python-version: '3.x'      # Use Python 3

      - name: Bump version
        run: python scripts/bump_version.py

      - name: Push changes
        if: ${{ success() }}
        run: |
          # Push the commit created by the bump script
          git push origin HEAD:${{ github.ref_name }}
      
      - name: Tag the new version (optional)
        if: ${{ success() }}
        run: |
          VERSION=$(grep -oP '(?<=VERSION: &str = \")[0-9]+\.[0-9]+' wfl/src/version.rs)
          git tag -a "v$VERSION" -m "Release $VERSION"
          git push origin --tags
```

**Explanation:**

* The workflow is triggered on pushes to the **main** branch (you might restrict this to avoid bumping on every branch or PR; typically, you’d run this after merging PRs or only for official release flows). You can also trigger manually or on a schedule if desired.
* **Checkout:** We use `actions/checkout@v3` to pull the repo. By default, `persist-credentials` is true, meaning the `GITHUB_TOKEN` is retained for subsequent `git` commands. This token will be used to push the commit.
* **Setup Python:** Ensures a Python environment is available to run our script.
* **Run bump script:** Executes the Python script which increments the version, updates files, and commits the changes. At this point, the repository has a new commit locally (with `[skip ci]` in the message as set by the script).
* **Push changes:** If the script ran successfully, push the new commit back to the GitHub repository. We explicitly push to the same branch (e.g., `main`). The `[skip ci]` in the commit message prevents this push from triggering a new workflow run.
* **Tagging (optional):** We create a Git tag for the new version (prefixed with `v`, e.g., `v2025.1`). This step finds the version string from the Rust file (using a grep regex to extract `2025.1` from the line) and tags that commit. Pushing the tags publishes them to the repo. Tagging is useful for marking releases or fetching artifacts later. *(This step can be skipped or replaced with creating a GitHub Release, depending on your release strategy.)*

**Safe Commit Practices:** The key part is that the commit made by the script is done with a special message to avoid an infinite loop of CI triggers. The use of **“\[skip ci]”** in commit message is a widely used convention to signal CI systems to ignore that commit. GitHub Actions honors this convention, so the workflow will not re-run for the skip-ci commit. Additionally, by restricting the trigger to `push` on `main` (and not on every commit or tag), we ensure that only the intended events run the bump process.

You should also consider adding an [`if` condition](https://docs.github.com/en/actions/learn-github-actions/expressions#operators) so that the job **does not run on the auto-commit** made by the bot. For example, one could check `if: ${{ github.actor != 'github-actions' }}` at the job level, or simply rely on the skip-ci as done above.

## 5. Ensuring Version Availability in WFL

With this setup, every official build updates the version constant. To make the version visible to users or developers:

* **Command-line Interface:** If WFL has a CLI, include a `--version` flag or similar that prints the `VERSION` constant. For instance, if using Clap for argument parsing, you can set the app version to this constant (rather than a hard-coded string). This way running `wfl --version` will show “WFL 2025.1” (or the current version).
* **Logging/Metadata:** Alternatively, the language runtime can log the version at startup or include it in a `help` message. The important part is that the `VERSION` constant from `version.rs` is referenced wherever needed, ensuring the program’s reported version always matches the build.

By doing this, any binary or artifact produced by the CI pipeline will carry the correct version string internally.

## 6. Maintenance and Best Practices

Setting up automated versioning requires some care to keep it reliable:

* **Branch Strategy:** Run the version bump workflow only on the primary branch (e.g., `main` or `master`) or during release processes. Avoid running on feature branches or Pull Requests to prevent multiple conflicting version bumps. Only **official builds** (like after merging code into main or a designated release trigger) should increment the version.
* **Avoiding Conflicts:** Because the version is auto-committed, if multiple builds run in parallel or close in time (e.g., two merges in quick succession), you might encounter a merge conflict on `.build_meta.json` or `version.rs`. In practice, sequential merges will trigger the workflow one at a time. However, if a race condition occurs (two CI runs with the same base metadata), one push might be rejected – you’d then need to manually resolve or rerun the failed workflow. Keeping the process to one branch and using skip-ci commits mitigates most of this risk.
* **Annual Rollover:** The first build in a new calendar year will reset the build number to 1. This is handled automatically by the script’s logic (comparing stored year to current year). To ensure this works, the CI runners’ clock should be correct (usually true by default). As a safeguard, review the first release of each year to verify the version string reset (e.g., the first build in 2026 should produce **2026.1**).
* **GitHub Actions Permissions:** Make sure the GitHub Actions workflow has permission to push to the repository. By default, the `GITHUB_TOKEN` in Actions can push to the repo, but if you have locked down permissions, you may need to adjust the workflow YAML with `permissions: contents: write`. In our snippet, we relied on default permissions with `actions/checkout@v3` which usually suffice.
* **Commit Message Conventions:** The commit message from the script includes the new version for clarity (e.g., *"Bump version to 2025.7 \[skip ci]"*). This makes it easy to trace changes in git history. The `[skip ci]` tag prevents the automated commit from triggering another CI run. Ensure developers don’t use “\[skip ci]” in normal commits to avoid skipping intentional builds.
* **Tagging and Releases:** Tagging each version is optional but recommended for traceability. Tags like `v2025.7` allow you to fetch the exact code for that version or download packaged binaries from that tag. You could enhance the workflow to automatically create a GitHub Release with release notes if desired, but in early development, a simple tag is sufficient.
* **Cargo.toml (Optional):** If this project will be published as a crate or you want to keep Cargo’s package version in sync, you might also update the `version` in `Cargo.toml`. Cargo’s version isn’t automatically used by our setup, since we’re using a custom scheme. But for consistency, consider aligning Cargo.toml’s version with the year.build or at least updating it periodically. Some projects use `env!("CARGO_PKG_VERSION")` in Rust to get the version, but here we maintain a separate version constant for custom formatting. Just be mindful if crate publishing becomes a goal.
* **Testing the Setup:** It’s wise to test the version bump script and workflow on a non-critical branch or repository. Ensure that when a push happens, the workflow runs, updates the files, commits, and doesn’t trigger again. You can simulate a year change by tweaking the JSON and system date (or temporarily altering the script logic) to verify the reset logic. Monitoring the GitHub Actions logs will show the printed new version and the success of git push.

By following this plan, the WFL project will have a **durable, automated versioning system**. Every build’s version is uniquely identified, the version constant in Rust ensures the running program knows its version, and the Git history/tags provide a record of each build. This reduces manual effort and potential errors in bumping versions, allowing the team to focus on development while the infrastructure handles version tracking.
*Implemented on 2025-05-19 - The automated versioning system has been implemented as described in this document.*
