//! Import processing for WFL
//!
//! This module handles loading and inlining imported files during parsing.

use super::ast::{Expression, Literal, ParseError, Program, Statement};
use super::Parser;
use crate::diagnostics::Span;
use crate::lexer::lex_wfl_with_positions;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Process all import statements in a program and inline the imported code
pub fn process_imports(
    program: Program,
    base_path: &Path,
) -> Result<Program, Vec<ParseError>> {
    let mut processor = ImportProcessor::new(base_path);
    processor.process_program(program)
}

struct ImportProcessor {
    base_path: PathBuf,
    imported_files: HashSet<PathBuf>,
    import_stack: Vec<PathBuf>,
}

impl ImportProcessor {
    fn new(base_path: &Path) -> Self {
        Self {
            base_path: base_path.to_path_buf(),
            imported_files: HashSet::new(),
            import_stack: Vec::new(),
        }
    }

    fn process_program(&mut self, program: Program) -> Result<Program, Vec<ParseError>> {
        let mut new_statements = Vec::new();
        let mut errors = Vec::new();

        for statement in program.statements {
            match statement {
                Statement::ImportStatement { path, line, column } => {
                    match self.process_import(&path, line, column) {
                        Ok(imported_stmts) => {
                            new_statements.extend(imported_stmts);
                        }
                        Err(e) => {
                            errors.push(e);
                        }
                    }
                }
                other => {
                    new_statements.push(other);
                }
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(Program {
            statements: new_statements,
        })
    }

    fn process_import(
        &mut self,
        path_expr: &Expression,
        line: usize,
        column: usize,
    ) -> Result<Vec<Statement>, ParseError> {
        // Extract path string from expression
        let path_str = match path_expr {
            Expression::Literal(Literal::String(s), _, _) => s.clone(),
            _ => {
                return Err(make_error(
                    "Import path must be a string literal",
                    line,
                    column,
                ));
            }
        };

        // Resolve path relative to base_path
        let resolved_path = self.resolve_path(&path_str, line, column)?;

        // Check for circular dependency
        if self.import_stack.contains(&resolved_path) {
            let cycle: Vec<String> = self
                .import_stack
                .iter()
                .map(|p| p.display().to_string())
                .collect();
            return Err(make_error(
                &format!(
                    "Circular dependency detected: {} -> {}",
                    cycle.join(" -> "),
                    resolved_path.display()
                ),
                line,
                column,
            ));
        }

        // Check if already imported (skip if yes)
        if self.imported_files.contains(&resolved_path) {
            return Ok(vec![]); // Already imported, return empty
        }

        // Mark as imported
        self.imported_files.insert(resolved_path.clone());
        self.import_stack.push(resolved_path.clone());

        // Load and parse the file
        let imported_program = self.load_and_parse(&resolved_path, line, column)?;

        // Recursively process imports in the imported file (while still on the stack)
        let processed = self.process_program(imported_program).map_err(|errors| {
            // Just return the first error for simplicity
            errors.into_iter().next().unwrap()
        })?;

        // Remove from stack after all recursive imports are processed
        self.import_stack.pop();

        Ok(processed.statements)
    }

    fn resolve_path(&self, path: &str, line: usize, column: usize) -> Result<PathBuf, ParseError> {
        // Try relative to base_path first
        let relative_path = self.base_path.join(path);

        if relative_path.exists() {
            return relative_path.canonicalize().map_err(|e| {
                make_error(
                    &format!("Failed to resolve path '{}': {}", path, e),
                    line,
                    column,
                )
            });
        }

        // Try relative to current working directory
        let cwd_path = PathBuf::from(path);
        if cwd_path.exists() {
            return cwd_path.canonicalize().map_err(|e| {
                make_error(
                    &format!("Failed to resolve path '{}': {}", path, e),
                    line,
                    column,
                )
            });
        }

        // File not found
        Err(make_error(
            &format!(
                "Cannot find module '{}'. Searched:\n  - {}\n  - {}",
                path,
                relative_path.display(),
                cwd_path.display()
            ),
            line,
            column,
        ))
    }

    fn load_and_parse(
        &self,
        path: &PathBuf,
        line: usize,
        column: usize,
    ) -> Result<Program, ParseError> {
        // Read file
        let source = fs::read_to_string(path).map_err(|e| {
            make_error(
                &format!("Failed to read file '{}': {}", path.display(), e),
                line,
                column,
            )
        })?;

        // Lex
        let tokens = lex_wfl_with_positions(&source);

        // Parse without processing imports (we'll handle that recursively)
        let parent_dir = path.parent().unwrap_or(Path::new("."));
        let mut parser = Parser::new(&tokens);
        parser.set_base_path(parent_dir.to_path_buf());

        let program = parser.parse_without_imports().map_err(|errors| {
            // Return the first error with context
            let first_error = &errors[0];
            make_error(
                &format!(
                    "Error parsing imported file '{}': {}",
                    path.display(),
                    first_error.message
                ),
                line,
                column,
            )
        })?;

        Ok(program)
    }
}

/// Helper function to create a ParseError with dummy span
fn make_error(message: &str, line: usize, column: usize) -> ParseError {
    ParseError::from_span(
        message.to_string(),
        Span { start: 0, end: 1 },
        line,
        column,
    )
}
