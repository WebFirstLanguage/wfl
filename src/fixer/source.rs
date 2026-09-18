//! Source-preserving formatting. Edits touch trivia and safe local identifier
//! spellings; every grammar construct and literal keeps its original tokens.

use super::{CodeFixer, FixerSummary};
use crate::lexer::lex_wfl_with_positions;
use crate::lexer::token::Token;
use crate::parser::{Parser, ast::Program};
use logos::Logos;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Read, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};

/// Unlike the recovering compiler lexer, formatting must reject *every*
/// unrecognized byte before it can overwrite a file or claim a clean lint.
pub fn validate_source(source: &str) -> io::Result<()> {
    let budget = crate::exec::budget::ExecutionBudget::current();
    if let Some(budget) = &budget {
        budget
            .check_source_bytes(source.len())
            .map_err(io::Error::other)?;
    }
    let mut lexer = Token::lexer(source);
    let mut count = 0usize;
    while let Some(token) = lexer.next() {
        if count.is_multiple_of(4096)
            && let Some(budget) = &budget
        {
            budget.check_cancelled().map_err(io::Error::other)?;
            budget.check_deadline().map_err(io::Error::other)?;
        }
        count += 1;
        if token.is_err() || matches!(token, Ok(Token::Error)) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Lexing error at byte {}: unexpected input {:?}",
                    lexer.span().start,
                    lexer.slice()
                ),
            ));
        }
    }
    Ok(())
}

/// Publish fully validated output with a same-directory atomic replacement.
/// Failed writes/flushes/renames leave the original intact. Preserve symlinks
/// and file permissions, and refuse read-only files or observed stale source.
/// A sibling marker excludes cooperating WFL writers across the final source
/// check and replacement. Editors that ignore that marker can still race the
/// replacement; this is not a filesystem compare-and-swap operation.
pub fn write_fixed_file(path: &Path, original: &str, fixed: &str) -> io::Result<()> {
    let destination = fs::canonicalize(path)?;
    let metadata = fs::metadata(&destination)?;
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Source must be a regular file",
        ));
    }
    if original == fixed {
        if !source_is_unchanged(&destination, original)? {
            return Err(io::Error::other(
                "Source changed while formatting; no fixes were written",
            ));
        }
        return Ok(());
    }
    if metadata.permissions().readonly() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Source is read-only",
        ));
    }
    // Lock the destination name, not the source inode: atomic replacement
    // changes that inode, and a waiter could otherwise lock the obsolete file.
    // Keep this guard alive through rename and temporary-file cleanup.
    let _writer_lock = FormatterWriteLock::acquire(&destination)?;
    if !source_is_unchanged(&destination, original)? {
        return Err(io::Error::other(
            "Source changed while formatting; no fixes were written",
        ));
    }
    let parent = destination
        .parent()
        .ok_or_else(|| io::Error::other("Source has no parent directory"))?;
    let mut temporary = tempfile::Builder::new()
        .prefix(".wfl-fix-")
        .tempfile_in(parent)?;
    temporary
        .as_file()
        .set_permissions(metadata.permissions())?;
    temporary.write_all(fixed.as_bytes())?;
    temporary.flush()?;
    temporary.as_file().sync_all()?;
    // This catches edits observed during preparation. The marker excludes
    // cooperating writers only; an arbitrary editor may ignore it entirely.
    if !source_is_unchanged(&destination, original)? {
        return Err(io::Error::other(
            "Source changed while formatting; no fixes were written",
        ));
    }
    let temporary_path = temporary.into_temp_path();
    fs::rename(&temporary_path, &destination)?;
    Ok(())
}

/// Ownership is established only by atomic creation. Never open or remove an
/// existing marker: its owner might still be preparing a replacement. Keeping
/// the marker path stable also avoids the orphan-inode race of unlinking a
/// reusable advisory lock file while another process has it open.
struct FormatterWriteLock {
    path: PathBuf,
    file: Option<fs::File>,
}

impl FormatterWriteLock {
    /// Claim the canonical destination's marker without waiting or touching an
    /// existing marker. A failed owner-information write releases this claim.
    fn acquire(destination: &Path) -> io::Result<Self> {
        let name = destination
            .file_name()
            .ok_or_else(|| io::Error::other("Source has no filename"))?;
        let digest = Sha256::digest(name.as_encoded_bytes());
        let path = destination.with_file_name(format!(".wfl-fix-{digest:x}.lock"));
        let file = fs::File::create_new(&path).map_err(|error| {
            if error.kind() == io::ErrorKind::AlreadyExists {
                io::Error::new(
                    io::ErrorKind::WouldBlock,
                    format!(
                        "Formatter lock {} already exists; retry later, or remove the lock only after confirming its owner has stopped",
                        path.display()
                    ),
                )
            } else {
                error
            }
        })?;
        let mut lock = Self {
            path,
            file: Some(file),
        };
        // Build the guard before writing so even an owner-information write
        // failure removes only the marker this process successfully created.
        writeln!(
            lock.file.as_mut().expect("new lock owns its file"),
            "WFL formatter process {}",
            std::process::id()
        )?;
        Ok(lock)
    }
}

impl Drop for FormatterWriteLock {
    /// Close the owned marker and remove it best-effort; cleanup failure leaves
    /// a marker that blocks later writers until explicit recovery.
    fn drop(&mut self) {
        // Close first for Windows. A crash, or a failed cleanup, leaves a
        // fail-closed marker for explicit recovery instead of risking a write.
        drop(self.file.take());
        let _ = fs::remove_file(&self.path);
    }
}

/// Compare source bytes using fixed-size chunks and at most one extra byte.
/// Growth, truncation, or changed content returns false; other I/O errors are
/// propagated. This bounds rereads even when an editor replaces the file.
fn source_is_unchanged(path: &Path, original: &str) -> io::Result<bool> {
    let mut input = fs::File::open(path)?;
    if input.metadata()?.len() != original.len() as u64 {
        return Ok(false);
    }
    let mut buffer = [0; 8192];
    for expected in original.as_bytes().chunks(buffer.len()) {
        let actual = &mut buffer[..expected.len()];
        if let Err(error) = input.read_exact(actual) {
            return if error.kind() == io::ErrorKind::UnexpectedEof {
                Ok(false)
            } else {
                Err(error)
            };
        }
        if actual != expected {
            return Ok(false);
        }
    }
    Ok(input.read(&mut buffer[..1])? == 0)
}

/// Apply trivia and conservative identifier edits, then validate the result.
/// The supplied AST must describe `source`; literals and untouched token spans
/// are copied directly, and overlapping edits or invalid output are rejected.
pub(super) fn fix_source(
    fixer: &CodeFixer,
    program: &Program,
    source: &str,
) -> io::Result<(String, FixerSummary)> {
    validate_source(source)?;
    let layout = crate::linter::layout::SourceLayout::new(source, program);
    let mut edits: Vec<(Range<usize>, String)> = Vec::new();
    let budget = crate::exec::budget::ExecutionBudget::current_or_default();
    let mut expanded_len = source.len();
    for line in &layout.lines {
        if let Some(depth) = line.depth {
            let width = depth.checked_mul(fixer.indent_size).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Configured indentation is too large",
                )
            })?;
            expanded_len = expanded_len
                .checked_add(width.saturating_sub(line.indentation.len()))
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "Formatted source is too large")
                })?;
            budget
                .check_source_bytes(expanded_len)
                .map_err(io::Error::other)?;
            let desired = " ".repeat(width);
            if source[line.indentation.clone()] != desired {
                edits.push((line.indentation.clone(), desired));
            }
        }
    }
    // Split at each physical newline without normalizing its bytes. Whitespace
    // inside a multiline string is data even when it lies at the end of a line.
    let bytes = source.as_bytes();
    let mut start = 0;
    while start < bytes.len() {
        let end = bytes[start..]
            .iter()
            .position(|b| matches!(b, b'\r' | b'\n'))
            .map_or(bytes.len(), |i| start + i);
        let trimmed = start + source[start..end].trim_end().len();
        if !fixer.trailing_whitespace && trimmed < end && !layout.overlaps_string(&(trimmed..end)) {
            edits.push((trimmed..end, String::new()));
        }
        start = end + usize::from(end < bytes.len());
        if end < bytes.len() && bytes[end] == b'\r' && bytes.get(start) == Some(&b'\n') {
            start += 1;
        }
    }
    let tokens = lex_wfl_with_positions(source);
    for token in &tokens {
        if fixer.consistent_keyword_case && matches!(token.token, Token::BooleanLiteral(_)) {
            let text = &source[token.byte_start..token.byte_end];
            let lower = text.to_ascii_lowercase();
            if lower != text {
                edits.push((token.byte_start..token.byte_end, lower));
            }
        }
    }
    let mut summary = FixerSummary::default();
    if fixer.snake_case_variables {
        rename_locals(fixer, program, source, &tokens, &mut edits, &mut summary);
    }
    edits.sort_by_key(|(range, _)| (range.start, range.end));
    let mut fixed = String::with_capacity(source.len());
    let mut cursor = 0;
    for (range, replacement) in edits {
        if range.start < cursor {
            return Err(io::Error::other(
                "Overlapping formatting edits; source was not modified",
            ));
        }
        fixed.push_str(&source[cursor..range.start]);
        fixed.push_str(&replacement);
        cursor = range.end;
    }
    fixed.push_str(&source[cursor..]);
    validate_source(&fixed)?;
    Parser::new(&lex_wfl_with_positions(&fixed))
        .parse()
        .map_err(|errors| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Formatted source did not parse; source was not modified: {errors:?}"),
            )
        })?;
    summary.lines_reformatted = source
        .lines()
        .zip(fixed.lines())
        .filter(|(old, new)| old != new)
        .count()
        + source.lines().count().abs_diff(fixed.lines().count());
    Ok((fixed, summary))
}

/// Schedule local spelling edits while excluding module contracts, public
/// names, naming collisions, and replacements that would become keywords.
fn rename_locals(
    fixer: &CodeFixer,
    program: &Program,
    source: &str,
    tokens: &[crate::lexer::token::TokenWithPosition],
    edits: &mut Vec<(Range<usize>, String)>,
    summary: &mut FixerSummary,
) {
    use crate::parser::ast::Statement;
    // A single-file formatter cannot update other modules' callers/bindings.
    // Leave naming diagnostics actionable without changing module contracts.
    if tokens.iter().any(|token| {
        matches!(
            token.token,
            Token::KeywordInclude | Token::KeywordLoad | Token::KeywordExport
        )
    }) {
        return;
    }
    let mut candidates = HashSet::new();
    let mut protected = HashSet::new();
    for statement in crate::linter::naming_statements(program) {
        match statement {
            Statement::VariableDeclaration {
                name,
                is_constant: false,
                ..
            } => {
                candidates.insert(name.as_str());
            }
            Statement::ActionDefinition {
                name, parameters, ..
            } => {
                candidates.insert(name.as_str());
                protected.extend(parameters.iter().map(|parameter| parameter.name.as_str()));
            }
            Statement::VariableDeclaration {
                name,
                is_constant: true,
                ..
            } => {
                protected.insert(name.as_str());
            }
            Statement::ContainerDefinition {
                name,
                properties,
                static_properties,
                methods,
                static_methods,
                events,
                ..
            } => {
                protected.insert(name.as_str());
                for property in properties.iter().chain(static_properties) {
                    protected.insert(property.name.as_str());
                }
                for method in methods.iter().chain(static_methods) {
                    if let Statement::ActionDefinition {
                        name, parameters, ..
                    } = method
                    {
                        protected.insert(name.as_str());
                        protected
                            .extend(parameters.iter().map(|parameter| parameter.name.as_str()));
                    }
                }
                for event in events {
                    protected.insert(event.name.as_str());
                    protected.extend(
                        event
                            .parameters
                            .iter()
                            .map(|parameter| parameter.name.as_str()),
                    );
                }
            }
            Statement::InterfaceDefinition {
                name,
                required_actions,
                ..
            } => {
                protected.insert(name.as_str());
                for action in required_actions {
                    protected.insert(action.name.as_str());
                    protected.extend(
                        action
                            .parameters
                            .iter()
                            .map(|parameter| parameter.name.as_str()),
                    );
                }
            }
            Statement::ContainerInstantiation {
                container_type,
                property_initializers,
                ..
            } => {
                protected.insert(container_type.as_str());
                for property in property_initializers {
                    protected.insert(property.name.as_str());
                }
            }
            Statement::MapCreation { entries, .. } => {
                protected.extend(entries.iter().map(|(name, _)| name.as_str()));
            }
            Statement::EventDefinition {
                name, parameters, ..
            } => {
                protected.insert(name.as_str());
                protected.extend(parameters.iter().map(|parameter| parameter.name.as_str()));
            }
            Statement::EventTrigger { name, .. } => {
                protected.insert(name.as_str());
            }
            Statement::EventHandler { event_name, .. } => {
                protected.insert(event_name.as_str());
            }
            Statement::PatternDefinition { name, pattern, .. } => {
                protected.insert(name.as_str());
                protect_pattern_names(pattern, &mut protected);
            }
            _ => {}
        }
    }
    // A local spelling can also occur as an external property or method name.
    // Renaming all occurrences would silently change those public APIs.
    for pair in tokens.windows(2) {
        if matches!(pair[0].token, Token::Dot | Token::Colon)
            && let Token::Identifier(name) = &pair[1].token
        {
            protected.insert(name.as_str());
        }
    }
    let existing: HashSet<_> = tokens
        .iter()
        .filter_map(|token| match &token.token {
            Token::Identifier(name) => Some(name.as_str()),
            _ => None,
        })
        .collect();
    let mut targets: HashMap<String, usize> = HashMap::new();
    for name in &existing {
        *targets.entry(fixer.to_snake_case(name)).or_default() += 1;
    }
    let renames: HashMap<_, _> = candidates.into_iter().filter_map(|name| {
        let fixed = fixer.to_snake_case(name);
        if fixed == name || protected.contains(name) || existing.contains(fixed.as_str()) || targets.get(&fixed) != Some(&1) {
            return None;
        }
        // A rename such as Store -> store or True -> true changes token kind.
        let replacement = lex_wfl_with_positions(&fixed);
        if !matches!(replacement.as_slice(), [only] if matches!(&only.token, Token::Identifier(value) if value == &fixed)) {
            return None;
        }
        Some((name, fixed))
    }).collect();
    let mut renamed = HashSet::new();
    for token in tokens {
        if let Token::Identifier(name) = &token.token
            && let Some(fixed) = renames.get(name.as_str())
        {
            // Merged identifier spans may include comments. Only rewrite a
            // plain identifier's original whitespace-separated spelling.
            let raw = &source[token.byte_start..token.byte_end];
            if raw.split_whitespace().collect::<Vec<_>>().join(" ") != *name {
                continue;
            }
            edits.push((token.byte_start..token.byte_end, fixed.clone()));
            renamed.insert(name);
        }
    }
    summary.vars_renamed = renamed.len();
}

/// Protect capture and backreference spellings throughout a pattern without
/// using recursive Rust calls for nested pattern expressions.
fn protect_pattern_names<'a>(
    pattern: &'a crate::parser::ast::PatternExpression,
    protected: &mut HashSet<&'a str>,
) {
    use crate::parser::ast::PatternExpression;
    let mut pending = vec![pattern];
    while let Some(pattern) = pending.pop() {
        match pattern {
            PatternExpression::Capture { name, pattern } => {
                protected.insert(name);
                pending.push(pattern);
            }
            PatternExpression::Backreference(name) => {
                protected.insert(name);
            }
            PatternExpression::Sequence(parts) | PatternExpression::Alternative(parts) => {
                pending.extend(parts)
            }
            PatternExpression::Quantified { pattern, .. }
            | PatternExpression::Lookahead(pattern)
            | PatternExpression::NegativeLookahead(pattern)
            | PatternExpression::Lookbehind(pattern)
            | PatternExpression::NegativeLookbehind(pattern) => pending.push(pattern),
            _ => {}
        }
    }
}
