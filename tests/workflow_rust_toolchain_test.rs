//! Guard: every GitHub Actions job that runs Cargo must install a Rust toolchain
//! *before* the first Cargo invocation.
//!
//! Background (the defect this reproduces): the `bump-version` job in `ci.yml`
//! ran `python scripts/bump_version.py --update-all` — which shells out to
//! `cargo update` and `cargo check --locked --manifest-path fuzz/Cargo.toml` —
//! without ever installing a toolchain. It silently depended on whatever `rustc`
//! the runner image happened to preinstall. When CI moved to Blacksmith runners
//! that image shipped rustc 1.92.0, below this crate's `rust-version = "1.94"`
//! (raised by `sqlx` 0.9), so the locked fuzz check failed and every push to
//! `main` went red.
//!
//! The Cargo dependency was invisible to a reader because it hid behind a Python
//! script, so this guard resolves that indirection explicitly via
//! `CARGO_INVOKING_SCRIPTS` — and `cargo_invoking_scripts_list_is_complete`
//! keeps that list honest as scripts change.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

/// Scripts that shell out to Cargo. A job that runs one of these needs a
/// toolchain just as much as a job that types `cargo` directly.
const CARGO_INVOKING_SCRIPTS: &[&str] = &["bump_version.py"];

/// Markers for a step that installs/selects a Rust toolchain.
const TOOLCHAIN_MARKERS: &[&str] = &["rust-toolchain", "rustup "];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workflow_files() -> Vec<PathBuf> {
    let dir = repo_root().join(".github").join("workflows");
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .map(|entry| entry.expect("cannot read workflow dir entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|ext| ext == "yml" || ext == "yaml")
        })
        .collect();
    files.sort();
    assert!(
        !files.is_empty(),
        "no workflow files found under {}",
        dir.display()
    );
    files
}

/// Split a workflow into `(job_id, job_body)` pairs.
///
/// Deliberately a line scanner rather than a YAML parse: the repo has no YAML
/// dependency, and job ids are the only two-space-indented keys inside `jobs:`.
/// Full-line comments are dropped so prose mentioning `cargo` cannot be mistaken
/// for a real invocation.
fn jobs_of(source: &str) -> Vec<(String, String)> {
    let mut jobs: Vec<(String, Vec<&str>)> = Vec::new();
    let mut in_jobs = false;

    for line in source.lines() {
        if !in_jobs {
            in_jobs = line.trim_end() == "jobs:";
            continue;
        }
        // A new top-level key ends the `jobs:` mapping.
        if !line.is_empty() && !line.starts_with(char::is_whitespace) && !line.starts_with('#') {
            in_jobs = false;
            continue;
        }
        if line.trim_start().starts_with('#') {
            continue;
        }
        if let Some(job_id) = job_header(line) {
            jobs.push((job_id, Vec::new()));
            continue;
        }
        if let Some((_, body)) = jobs.last_mut() {
            body.push(line);
        }
    }

    jobs.into_iter()
        .map(|(id, body)| (id, body.join("\n")))
        .collect()
}

/// `  job-id:` — exactly two spaces of indent, nothing after the colon.
fn job_header(line: &str) -> Option<String> {
    let rest = line.strip_prefix("  ")?;
    if rest.starts_with(' ') {
        return None;
    }
    let id = rest.strip_suffix(':')?;
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return None;
    }
    Some(id.to_string())
}

fn first_match(haystack: &str, needles: &[&str]) -> Option<usize> {
    needles.iter().filter_map(|n| haystack.find(n)).min()
}

fn first_cargo_use(body: &str) -> Option<usize> {
    let mut needles: Vec<&str> = vec!["cargo "];
    needles.extend_from_slice(CARGO_INVOKING_SCRIPTS);
    first_match(body, &needles)
}

#[test]
fn cargo_jobs_install_a_rust_toolchain_first() {
    let mut missing = Vec::new();
    let mut out_of_order = Vec::new();

    for path in workflow_files() {
        let source = fs::read_to_string(&path).expect("cannot read workflow file");
        let name = path.file_name().unwrap().to_string_lossy().to_string();

        for (job, body) in jobs_of(&source) {
            let Some(cargo_at) = first_cargo_use(&body) else {
                continue;
            };
            match first_match(&body, TOOLCHAIN_MARKERS) {
                None => missing.push(format!("{name}:{job}")),
                Some(toolchain_at) if toolchain_at > cargo_at => {
                    out_of_order.push(format!("{name}:{job}"))
                }
                Some(_) => {}
            }
        }
    }

    assert!(
        missing.is_empty() && out_of_order.is_empty(),
        "GitHub Actions jobs run Cargo without a pinned Rust toolchain, so they \
         inherit whatever rustc the runner image ships — which can be older than \
         this crate's MSRV (rust-version = \"1.94\").\n\
         \n  no toolchain step: {missing:?}\
         \n  toolchain installed after the first Cargo use: {out_of_order:?}\n\
         \nAdd `- uses: dtolnay/rust-toolchain@stable` before the Cargo step.\n\
         Jobs invoking Cargo indirectly through {CARGO_INVOKING_SCRIPTS:?} count too.",
    );
}

/// The indirection list is load-bearing: if a new script starts shelling out to
/// Cargo and nobody adds it here, the guard above silently stops guarding.
#[test]
fn cargo_invoking_scripts_list_is_complete() {
    let scripts_dir = repo_root().join("scripts");
    let declared: BTreeSet<&str> = CARGO_INVOKING_SCRIPTS.iter().copied().collect();
    let mut undeclared = Vec::new();

    for entry in fs::read_dir(&scripts_dir).expect("cannot read scripts dir") {
        let path = entry.expect("cannot read scripts dir entry").path();
        if path.extension().is_none_or(|ext| ext != "py") {
            continue;
        }
        let source = fs::read_to_string(&path).expect("cannot read script");
        // Matches a subprocess argv literal whose program is cargo, e.g.
        // `subprocess.run(["cargo", "update", ...])`.
        if !source.contains("[\"cargo\"") && !source.contains("['cargo'") {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if !declared.contains(name.as_str()) {
            undeclared.push(name);
        }
    }

    assert!(
        undeclared.is_empty(),
        "these scripts invoke cargo but are missing from CARGO_INVOKING_SCRIPTS, \
         so workflow jobs running them would not be checked for a toolchain: {undeclared:?}",
    );
}

/// Guards the scanner itself: a body that mentions cargo only in a comment must
/// not be treated as a Cargo user, and a real job must be found.
#[test]
fn scanner_ignores_comments_and_finds_jobs() {
    let workflow = "\
name: Example
jobs:
  commented:
    runs-on: ubuntu-latest
    steps:
      # this job used to run cargo build
      - run: echo hi
  real:
    runs-on: ubuntu-latest
    steps:
      - run: cargo build
";
    let jobs = jobs_of(workflow);
    let ids: Vec<&str> = jobs.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, ["commented", "real"]);

    assert_eq!(
        first_cargo_use(&jobs[0].1),
        None,
        "comment counted as cargo"
    );
    assert!(
        first_cargo_use(&jobs[1].1).is_some(),
        "missed a real cargo run"
    );
}
