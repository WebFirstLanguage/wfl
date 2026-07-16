use std::path::{Path, PathBuf};

use ignore::Match;
use ignore::gitignore::{Gitignore, GitignoreBuilder};

use crate::error::PackageError;

const MAX_IGNORE_FILE_BYTES: u64 = 1024 * 1024;
const MAX_TOTAL_IGNORE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_TOTAL_IGNORE_RULES: usize = 4096;
const MAX_IGNORE_FILES: usize = 1024;

/// Git-ignore matchers active for the directory currently being traversed.
///
/// Each `.gitignore` is parsed by the same mature matcher used by ripgrep's
/// ignore walker. Nested matchers are evaluated after their parents so they
/// have Git's expected precedence. Callers must pop the number returned by
/// `push_for_dir` when they leave a directory.
pub(crate) struct IgnoreStack {
    files: Vec<IgnoreFile>,
    total_bytes: u64,
    total_rules: usize,
    ignore_files: usize,
}

struct IgnoreFile {
    base: PathBuf,
    matcher: Gitignore,
}

impl IgnoreStack {
    pub(crate) fn new(project_dir: &Path) -> Result<Self, PackageError> {
        let mut stack = Self {
            files: Vec::new(),
            total_bytes: 0,
            total_rules: 0,
            ignore_files: 0,
        };
        stack.push_for_dir(project_dir, Path::new(""))?;
        Ok(stack)
    }

    /// Load the `.gitignore` in `dir`, scoped to `relative_dir`.
    pub(crate) fn push_for_dir(
        &mut self,
        dir: &Path,
        relative_dir: &Path,
    ) -> Result<usize, PackageError> {
        let ignore_path = dir.join(".gitignore");
        match std::fs::symlink_metadata(&ignore_path) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
                return Err(PackageError::General(format!(
                    ".gitignore must be a regular, non-symlink file: {}",
                    ignore_path.display()
                )));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(error) => return Err(PackageError::Io(error)),
        }

        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NOFOLLOW);
        }
        let file = match options.open(&ignore_path) {
            Ok(file) => file,
            Err(error) => return Err(PackageError::Io(error)),
        };

        let metadata = file.metadata()?;
        if !metadata.is_file() {
            return Ok(0);
        }
        if metadata.len() > MAX_IGNORE_FILE_BYTES {
            return Err(PackageError::General(format!(
                ".gitignore is too large to process safely: {}",
                ignore_path.display()
            )));
        }

        use std::io::Read;
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        file.take(MAX_IGNORE_FILE_BYTES + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_IGNORE_FILE_BYTES {
            return Err(PackageError::General(format!(
                ".gitignore grew too large while it was being read: {}",
                ignore_path.display()
            )));
        }
        let content = String::from_utf8(bytes).map_err(|_| {
            PackageError::General(format!(
                ".gitignore is not valid UTF-8: {}",
                ignore_path.display()
            ))
        })?;

        let mut builder = GitignoreBuilder::new("");
        builder
            .case_insensitive(cfg!(windows))
            .map_err(ignore_pattern_error)?;
        for line in content.lines() {
            reject_unsupported_git_classes(line)?;
            builder
                .add_line(Some(ignore_path.clone()), line)
                .map_err(ignore_pattern_error)?;
        }
        let matcher = builder.build().map_err(ignore_pattern_error)?;
        let rule_count = (matcher.num_ignores() + matcher.num_whitelists()) as usize;

        let next_bytes = self
            .total_bytes
            .checked_add(content.len() as u64)
            .ok_or_else(ignore_budget_error)?;
        let next_files = self
            .ignore_files
            .checked_add(1)
            .ok_or_else(ignore_budget_error)?;
        let next_rules = self
            .total_rules
            .checked_add(rule_count)
            .ok_or_else(ignore_budget_error)?;
        if next_bytes > MAX_TOTAL_IGNORE_BYTES
            || next_files > MAX_IGNORE_FILES
            || next_rules > MAX_TOTAL_IGNORE_RULES
        {
            return Err(ignore_budget_error());
        }

        self.total_bytes = next_bytes;
        self.ignore_files = next_files;
        self.total_rules = next_rules;
        if rule_count == 0 {
            return Ok(0);
        }
        self.files.push(IgnoreFile {
            base: relative_dir.to_path_buf(),
            matcher,
        });
        Ok(1)
    }

    pub(crate) fn pop(&mut self, count: usize) {
        self.files.truncate(self.files.len().saturating_sub(count));
    }

    pub(crate) fn is_ignored(
        &self,
        relative_path: &Path,
        is_dir: bool,
    ) -> Result<bool, PackageError> {
        // Fail closed instead of publishing a path that cannot be represented
        // consistently in a portable package checksum.
        if relative_path.to_str().is_none() {
            return Err(PackageError::General(format!(
                "Package path is not valid Unicode: {}",
                relative_path.display()
            )));
        }

        let mut ignored = false;
        for file in &self.files {
            let Ok(scoped_path) = relative_path.strip_prefix(&file.base) else {
                continue;
            };
            if scoped_path.as_os_str().is_empty() {
                continue;
            }
            match file.matcher.matched(scoped_path, is_dir) {
                Match::Ignore(_) => ignored = true,
                Match::Whitelist(_) => ignored = false,
                Match::None => {}
            }
        }
        Ok(ignored)
    }
}

fn reject_unsupported_git_classes(line: &str) -> Result<(), PackageError> {
    // Git wildmatch supports POSIX bracket classes, while the matcher used by
    // `ignore` currently accepts but does not match them. Refuse these forms
    // instead of silently publishing an intended secret.
    if line.contains("[[:") || line.contains("[[.") || line.contains("[[=") {
        return Err(PackageError::General(
            "A .gitignore uses a POSIX bracket class that cannot be evaluated safely; refusing to publish."
                .to_string(),
        ));
    }
    Ok(())
}

fn ignore_pattern_error(error: ignore::Error) -> PackageError {
    PackageError::General(format!(
        "A .gitignore contains a pattern that cannot be evaluated safely; refusing to publish: {}",
        error
    ))
}

fn ignore_budget_error() -> PackageError {
    PackageError::General(
        "The project's .gitignore rules exceed the safe processing limit; refusing to publish."
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_rules_ignore_logs_and_allow_negation() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join(".gitignore"), "*.log\n!important.log\n").unwrap();
        let stack = IgnoreStack::new(temp.path()).unwrap();

        assert!(stack.is_ignored(Path::new("debug.log"), false).unwrap());
        assert!(
            stack
                .is_ignored(Path::new("nested/debug.log"), false)
                .unwrap()
        );
        assert!(!stack.is_ignored(Path::new("important.log"), false).unwrap());
    }

    #[test]
    fn directory_rules_exclude_descendants_by_pruning() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join(".gitignore"), "secrets/\n").unwrap();
        let stack = IgnoreStack::new(temp.path()).unwrap();

        assert!(stack.is_ignored(Path::new("secrets"), true).unwrap());
        assert!(stack.is_ignored(Path::new("nested/secrets"), true).unwrap());
    }

    #[test]
    fn negated_parent_does_not_clear_leaf_ignore() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join(".gitignore"), "*.log\n!a/b\n").unwrap();
        let stack = IgnoreStack::new(temp.path()).unwrap();

        assert!(stack.is_ignored(Path::new("a/b/x.log"), false).unwrap());
    }

    #[test]
    fn anchored_rules_only_match_from_their_base() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join(".gitignore"), "/.env\n").unwrap();
        let stack = IgnoreStack::new(temp.path()).unwrap();

        assert!(stack.is_ignored(Path::new(".env"), false).unwrap());
        assert!(!stack.is_ignored(Path::new("nested/.env"), false).unwrap());
    }

    #[test]
    fn escaped_metacharacters_and_star_runs_follow_git_syntax() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(
            temp.path().join(".gitignore"),
            "secret\\?.txt\nliteral\\*.env\nname\\ \nfoo**bar\n",
        )
        .unwrap();
        let stack = IgnoreStack::new(temp.path()).unwrap();

        assert!(stack.is_ignored(Path::new("secret?.txt"), false).unwrap());
        assert!(!stack.is_ignored(Path::new("secret1.txt"), false).unwrap());
        assert!(stack.is_ignored(Path::new("literal*.env"), false).unwrap());
        assert!(
            !stack
                .is_ignored(Path::new("literal-secret.env"), false)
                .unwrap()
        );
        assert!(stack.is_ignored(Path::new("name "), false).unwrap());
        assert!(
            stack
                .is_ignored(Path::new("foo-anything-bar"), false)
                .unwrap()
        );
    }

    #[test]
    fn posix_character_class_fails_closed() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join(".gitignore"), "file[[:digit:]].env\n").unwrap();
        assert!(IgnoreStack::new(temp.path()).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_gitignore_fails_closed() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("rules"), ".env\n").unwrap();
        symlink("rules", temp.path().join(".gitignore")).unwrap();
        assert!(IgnoreStack::new(temp.path()).is_err());
    }
}
