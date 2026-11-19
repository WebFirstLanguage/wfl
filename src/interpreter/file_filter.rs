//! Filters for `list files ... with extension(s) / pattern ...` (issue #578).

use std::path::Path;

use crate::parser::ast::FileFilterKind;

/// A `list files` filter, compiled once per listing and then checked against
/// every file found.
#[derive(Debug)]
pub(crate) enum FileFilter {
    /// `with extension(s)`: the file's extension, with its dot, must equal one
    /// of these values.
    Extensions(Vec<String>),
    /// `with pattern`: each glob is matched against the file name.
    Patterns(Vec<FilePattern>),
}

#[derive(Debug)]
pub(crate) struct FilePattern {
    glob: glob::Pattern,
    /// The pattern as written. Before #578 was fixed, `with pattern` compared
    /// this text with the file's extension, so a value such as `".txt"` still
    /// matches by extension to keep those programs working.
    source: String,
}

impl FileFilter {
    /// Compile `values` as `kind` filters. A malformed glob is reported as an
    /// error message that names the pattern.
    pub(crate) fn new(kind: FileFilterKind, values: Vec<String>) -> Result<Self, String> {
        match kind {
            FileFilterKind::Extension => Ok(Self::Extensions(values)),
            FileFilterKind::Pattern => values
                .into_iter()
                .map(|source| match glob::Pattern::new(&source) {
                    Ok(glob) => Ok(FilePattern { glob, source }),
                    Err(e) => Err(format!(
                        "Invalid file pattern \"{source}\": {e}. Use * for any characters, \
                         ? for exactly one character, and [abc] for one of a set of characters"
                    )),
                })
                .collect::<Result<_, _>>()
                .map(Self::Patterns),
        }
    }

    /// Whether the file at `path` passes the filter.
    pub(crate) fn matches(&self, path: &Path) -> bool {
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| format!(".{ext}"));
        match self {
            Self::Extensions(extensions) => extension.is_some_and(|ext| extensions.contains(&ext)),
            Self::Patterns(patterns) => {
                let name = path.file_name().and_then(|name| name.to_str());
                patterns.iter().any(|pattern| {
                    name.is_some_and(|name| pattern.glob.matches(name))
                        || extension.as_deref() == Some(pattern.source.as_str())
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compile(kind: FileFilterKind, values: &[&str]) -> FileFilter {
        FileFilter::new(kind, values.iter().map(|v| v.to_string()).collect()).expect("valid filter")
    }

    fn matching<'a>(filter: &FileFilter, names: &[&'a str]) -> Vec<&'a str> {
        names
            .iter()
            .copied()
            .filter(|name| filter.matches(Path::new("dir").join(name).as_path()))
            .collect()
    }

    const NAMES: &[&str] = &[
        "a.txt",
        "b.txt",
        "report.txt",
        "c.log",
        ".hidden.txt",
        "README",
    ];

    #[test]
    fn pattern_globs_match_the_file_name() {
        let cases: &[(&str, &[&str])] = &[
            ("*.txt", &["a.txt", "b.txt", "report.txt", ".hidden.txt"]),
            ("report.txt", &["report.txt"]),
            ("?.txt", &["a.txt", "b.txt"]),
            ("[ab].*", &["a.txt", "b.txt"]),
            ("[!ab].*", &["c.log"]),
            ("[a-b].txt", &["a.txt", "b.txt"]),
            // Matching is case-sensitive, like the `glob` function.
            ("*.TXT", &[]),
            ("*", NAMES),
            ("*.md", &[]),
        ];
        for (pattern, expected) in cases {
            let filter = compile(FileFilterKind::Pattern, &[pattern]);
            assert_eq!(matching(&filter, NAMES), *expected, "pattern {pattern:?}");
        }
    }

    #[test]
    fn pattern_cannot_name_a_file_outside_the_listed_directory() {
        // Only names inside the listed directory are compared, and a name
        // never contains a separator, so these can never match.
        for pattern in ["../*", "../a.txt", "dir/*", "*/a.txt"] {
            let filter = compile(FileFilterKind::Pattern, &[pattern]);
            assert!(
                matching(&filter, NAMES).is_empty(),
                "pattern {pattern:?} must not match"
            );
        }
    }

    #[test]
    fn many_wildcards_against_a_long_name_still_finish() {
        // A backtracking matcher can take exponential time on this shape.
        let pattern = format!("{}b", "*a".repeat(40));
        let long_name = "a".repeat(255);
        let filter = compile(FileFilterKind::Pattern, &[pattern.as_str()]);
        assert!(!filter.matches(Path::new(&long_name)));
        assert!(filter.matches(Path::new(&format!("{}b", "a".repeat(254)))));
    }

    #[test]
    fn any_of_several_patterns_may_match() {
        let filter = compile(FileFilterKind::Pattern, &["*.log", "README"]);
        assert_eq!(matching(&filter, NAMES), ["c.log", "README"]);
    }

    #[test]
    fn bare_extension_pattern_keeps_matching_by_extension() {
        let filter = compile(FileFilterKind::Pattern, &[".txt"]);
        assert_eq!(
            matching(&filter, NAMES),
            ["a.txt", "b.txt", "report.txt", ".hidden.txt"]
        );
    }

    #[test]
    fn extension_filter_compares_extensions_exactly() {
        let filter = compile(FileFilterKind::Extension, &[".txt", ".log"]);
        assert_eq!(
            matching(&filter, NAMES),
            ["a.txt", "b.txt", "report.txt", "c.log", ".hidden.txt"]
        );
        // Extension filters are not globs.
        let filter = compile(FileFilterKind::Extension, &["*.txt"]);
        assert!(matching(&filter, NAMES).is_empty());
    }

    #[test]
    fn malformed_pattern_is_rejected_with_its_text() {
        let error = FileFilter::new(FileFilterKind::Pattern, vec!["[abc".to_string()])
            .expect_err("unterminated set must be rejected");
        assert!(error.contains("Invalid file pattern \"[abc\""), "{error}");
        // Extension values are never parsed as globs, so they cannot fail.
        assert!(FileFilter::new(FileFilterKind::Extension, vec!["[abc".to_string()]).is_ok());
    }
}
