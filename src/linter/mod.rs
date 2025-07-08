use crate::diagnostics::{DiagnosticReporter, Severity, WflDiagnostic};
use crate::parser::ast::{Program, Statement};
use std::path::Path;

pub trait LintRule {
    fn code(&self) -> &'static str;

    fn description(&self) -> &'static str;

    fn apply(
        &self,
        program: &Program,
        reporter: &mut DiagnosticReporter,
        file_id: usize,
    ) -> Vec<WflDiagnostic>;
}

pub struct Linter {
    rules: Vec<Box<dyn LintRule>>,
    max_line_length: usize,
    max_nesting_depth: usize,
}

impl Linter {
    pub fn new() -> Self {
        let mut linter = Self {
            rules: Vec::new(),
            max_line_length: 100, // Default max line length
            max_nesting_depth: 5, // Default max nesting depth
        };

        linter.add_rule(Box::new(NamingConventionRule));
        linter.add_rule(Box::new(IndentationRule));
        linter.add_rule(Box::new(KeywordCasingRule));
        linter.add_rule(Box::new(TrailingWhitespaceRule));
        linter.add_rule(Box::new(LineLengthRule));
        linter.add_rule(Box::new(NestingDepthRule));

        linter
    }

    pub fn add_rule(&mut self, rule: Box<dyn LintRule>) {
        self.rules.push(rule);
    }

    pub fn set_max_line_length(&mut self, length: usize) {
        self.max_line_length = length;
    }

    pub fn set_max_nesting_depth(&mut self, depth: usize) {
        self.max_nesting_depth = depth;
    }

    pub fn lint(
        &self,
        program: &Program,
        source: &str,
        file_path: &str,
    ) -> (Vec<WflDiagnostic>, bool) {
        println!("Starting linting process...");
        let mut reporter = DiagnosticReporter::new();
        let file_id = reporter.add_file(file_path, source);

        let mut all_diagnostics = Vec::new();

        for rule in &self.rules {
            println!("Applying rule: {}", rule.code());
            let diagnostics = rule.apply(program, &mut reporter, file_id);
            println!("Rule {} found {} issues", rule.code(), diagnostics.len());
            all_diagnostics.extend(diagnostics);
        }

        println!("Linting complete. Found {} issues", all_diagnostics.len());
        (all_diagnostics.clone(), all_diagnostics.is_empty())
    }

    pub fn load_config(&mut self, dir: &Path) {
        let config = crate::config::load_config(dir);
        self.set_max_line_length(config.max_line_length);
        self.set_max_nesting_depth(config.max_nesting_depth);
    }
}

struct NamingConventionRule;

impl LintRule for NamingConventionRule {
    fn code(&self) -> &'static str {
        "LINT-NAME"
    }

    fn description(&self) -> &'static str {
        "Variable and action names should use snake_case"
    }

    fn apply(
        &self,
        program: &Program,
        _reporter: &mut DiagnosticReporter,
        file_id: usize,
    ) -> Vec<WflDiagnostic> {
        println!("  NamingConventionRule: checking variable and action names");
        let mut diagnostics = Vec::new();

        for statement in &program.statements {
            match statement {
                Statement::VariableDeclaration {
                    name, line, column, ..
                }
                | Statement::Assignment {
                    name, line, column, ..
                } => {
                    if !is_snake_case(name) {
                        let snake_case_name = to_snake_case(name);
                        let diagnostic = WflDiagnostic::new(
                            Severity::Warning,
                            format!("Variable name '{name}' should be snake_case"),
                            Some(format!("Rename to '{snake_case_name}'")),
                            "LINT-NAME".to_string(),
                            file_id,
                            *line,
                            *column,
                            None,
                        );
                        diagnostics.push(diagnostic);
                    }
                }
                Statement::ActionDefinition {
                    name, line, column, ..
                } => {
                    if !is_snake_case(name) {
                        let snake_case_name = to_snake_case(name);
                        let diagnostic = WflDiagnostic::new(
                            Severity::Warning,
                            format!("Action name '{name}' should be snake_case"),
                            Some(format!("Rename to '{snake_case_name}'")),
                            "LINT-NAME".to_string(),
                            file_id,
                            *line,
                            *column,
                            None,
                        );
                        diagnostics.push(diagnostic);
                    }
                }
                _ => {}
            }
        }

        diagnostics
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RuleSeverity {
    Allow, // Rule is disabled
    Warn,  // Rule generates warnings but doesn't cause failure
    #[default]
    Deny, // Rule generates errors and causes failure
    Forbid, // Rule generates errors and cannot be overridden
}

struct IndentationRule;

impl LintRule for IndentationRule {
    fn code(&self) -> &'static str {
        "LINT-INDENT"
    }

    fn description(&self) -> &'static str {
        "Code should be indented with 4 spaces"
    }

    fn apply(
        &self,
        _program: &Program,
        reporter: &mut DiagnosticReporter,
        file_id: usize,
    ) -> Vec<WflDiagnostic> {
        let mut diagnostics = Vec::new();

        if let Ok(file) = reporter.files.get(file_id) {
            let source = file.source();
            let lines: Vec<&str> = source.lines().collect();

            let mut expected_indent = 0;

            for (line_idx, line) in lines.iter().enumerate() {
                let line_num = line_idx + 1;
                let trimmed = line.trim_start();

                if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
                    continue;
                }

                let indent_spaces = line.len() - trimmed.len();

                if (trimmed.starts_with("end ")
                    || trimmed == "end action"
                    || trimmed == "otherwise:"
                    || trimmed == "end check")
                    && expected_indent >= 4
                {
                    expected_indent -= 4;
                }

                if indent_spaces != expected_indent {
                    diagnostics.push(WflDiagnostic::new(
                        Severity::Warning,
                        format!(
                            "Line should be indented with {expected_indent} spaces, found {indent_spaces}"
                        ),
                        Some(format!("Adjust indentation to {expected_indent} spaces")),
                        "LINT-INDENT".to_string(),
                        file_id,
                        line_num,
                        1, // Column is always 1 for indentation issues
                        None,
                    ));
                }

                if trimmed.ends_with(':') || trimmed.contains("then:") {
                    expected_indent += 4;
                }
            }
        }

        diagnostics
    }
}

struct KeywordCasingRule;

impl LintRule for KeywordCasingRule {
    fn code(&self) -> &'static str {
        "LINT-KEYWORD"
    }

    fn description(&self) -> &'static str {
        "Keywords should be lowercase"
    }

    fn apply(
        &self,
        _program: &Program,
        reporter: &mut DiagnosticReporter,
        file_id: usize,
    ) -> Vec<WflDiagnostic> {
        let mut diagnostics = Vec::new();

        if let Ok(file) = reporter.files.get(file_id) {
            let source = file.source();

            let keywords = [
                "store",
                "as",
                "create",
                "change",
                "to",
                "define",
                "action",
                "called",
                "give",
                "back",
                "check",
                "if",
                "otherwise",
                "end",
                "count",
                "from",
                "for",
                "each",
                "in",
                "while",
                "display",
                "yes",
                "no",
                "nothing",
                "missing",
                "undefined",
            ];

            for keyword in keywords.iter() {
                let uppercase_keyword = keyword.to_uppercase();
                let mixed_case_keyword = keyword
                    .chars()
                    .enumerate()
                    .map(|(i, c)| {
                        if i == 0 {
                            c.to_uppercase().next().unwrap()
                        } else {
                            c
                        }
                    })
                    .collect::<String>();

                if let Some(pos) = source.find(&uppercase_keyword) {
                    let line_col = line_col_from_pos(source, pos);
                    diagnostics.push(WflDiagnostic::new(
                        Severity::Warning,
                        format!("Keyword '{uppercase_keyword}' should be lowercase"),
                        Some(format!("Change to '{keyword}'")),
                        "LINT-KEYWORD".to_string(),
                        file_id,
                        line_col.0,
                        line_col.1,
                        None,
                    ));
                }

                if let Some(pos) = source.find(&mixed_case_keyword) {
                    let line_col = line_col_from_pos(source, pos);
                    diagnostics.push(WflDiagnostic::new(
                        Severity::Warning,
                        format!("Keyword '{mixed_case_keyword}' should be lowercase"),
                        Some(format!("Change to '{keyword}'")),
                        "LINT-KEYWORD".to_string(),
                        file_id,
                        line_col.0,
                        line_col.1,
                        None,
                    ));
                }
            }
        }

        diagnostics
    }
}

struct TrailingWhitespaceRule;

impl LintRule for TrailingWhitespaceRule {
    fn code(&self) -> &'static str {
        "LINT-WHITESPACE"
    }

    fn description(&self) -> &'static str {
        "Lines should not have trailing whitespace"
    }

    fn apply(
        &self,
        _program: &Program,
        reporter: &mut DiagnosticReporter,
        file_id: usize,
    ) -> Vec<WflDiagnostic> {
        let mut diagnostics = Vec::new();

        if let Ok(file) = reporter.files.get(file_id) {
            let source = file.source();
            let lines: Vec<&str> = source.lines().collect();

            for (line_idx, line) in lines.iter().enumerate() {
                let line_num = line_idx + 1;

                if line.trim_end().len() < line.len() {
                    diagnostics.push(WflDiagnostic::new(
                        Severity::Warning,
                        "Line has trailing whitespace".to_string(),
                        Some("Remove trailing whitespace".to_string()),
                        "LINT-WHITESPACE".to_string(),
                        file_id,
                        line_num,
                        line.trim_end().len() + 1,
                        None,
                    ));
                }
            }
        }

        diagnostics
    }
}

/// Rule for enforcing maximum line length
struct LineLengthRule;

impl LintRule for LineLengthRule {
    fn code(&self) -> &'static str {
        "LINT-LENGTH"
    }

    fn description(&self) -> &'static str {
        "Lines should not exceed the maximum length"
    }

    fn apply(
        &self,
        _program: &Program,
        reporter: &mut DiagnosticReporter,
        file_id: usize,
    ) -> Vec<WflDiagnostic> {
        let mut diagnostics = Vec::new();

        let max_length = 100;

        if let Ok(file) = reporter.files.get(file_id) {
            let source = file.source();
            let lines: Vec<&str> = source.lines().collect();

            for (line_idx, line) in lines.iter().enumerate() {
                let line_num = line_idx + 1;

                if line.len() > max_length {
                    diagnostics.push(WflDiagnostic::new(
                        Severity::Warning,
                        format!("Line exceeds maximum length of {max_length} characters"),
                        Some(format!("Shorten line to {max_length} characters or less")),
                        "LINT-LENGTH".to_string(),
                        file_id,
                        line_num,
                        max_length + 1,
                        None,
                    ));
                }
            }
        }

        diagnostics
    }
}

/// Rule for enforcing maximum nesting depth
struct NestingDepthRule;

impl LintRule for NestingDepthRule {
    fn code(&self) -> &'static str {
        "LINT-COMPLEX"
    }

    fn description(&self) -> &'static str {
        "Nesting depth should not exceed the maximum"
    }

    fn apply(
        &self,
        program: &Program,
        _reporter: &mut DiagnosticReporter,
        file_id: usize,
    ) -> Vec<WflDiagnostic> {
        let mut diagnostics = Vec::new();

        let max_depth = 5;

        for statement in &program.statements {
            check_nesting_depth(statement, 0, max_depth, &mut diagnostics, file_id);
        }

        diagnostics
    }
}

fn check_nesting_depth(
    statement: &Statement,
    current_depth: usize,
    max_depth: usize,
    diagnostics: &mut Vec<WflDiagnostic>,
    file_id: usize,
) {
    if current_depth > max_depth {
        match statement {
            Statement::IfStatement { line, column, .. }
            | Statement::WhileLoop { line, column, .. }
            | Statement::ForEachLoop { line, column, .. }
            | Statement::CountLoop { line, column, .. } => {
                diagnostics.push(WflDiagnostic::new(
                    Severity::Warning,
                    format!("Nesting depth exceeds maximum of {max_depth}"),
                    Some("Refactor to reduce nesting".to_string()),
                    "LINT-COMPLEX".to_string(),
                    file_id,
                    *line,
                    *column,
                    None,
                ));
            }
            _ => {}
        }
    }

    match statement {
        Statement::IfStatement {
            then_block,
            else_block,
            ..
        } => {
            for stmt in then_block {
                check_nesting_depth(stmt, current_depth + 1, max_depth, diagnostics, file_id);
            }
            if let Some(else_stmts) = else_block {
                for stmt in else_stmts {
                    check_nesting_depth(stmt, current_depth + 1, max_depth, diagnostics, file_id);
                }
            }
        }
        Statement::WhileLoop { body, .. }
        | Statement::ForEachLoop { body, .. }
        | Statement::CountLoop { body, .. } => {
            for stmt in body {
                check_nesting_depth(stmt, current_depth + 1, max_depth, diagnostics, file_id);
            }
        }
        Statement::ActionDefinition { body, .. } => {
            for stmt in body {
                check_nesting_depth(stmt, current_depth, max_depth, diagnostics, file_id);
            }
        }
        _ => {}
    }
}

fn is_snake_case(s: &str) -> bool {
    !s.contains(char::is_uppercase) && !s.contains(' ')
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut previous_char_is_lowercase = false;

    for (i, c) in s.char_indices() {
        if c.is_uppercase() {
            if i > 0 && previous_char_is_lowercase {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else if c == ' ' {
            result.push('_');
        } else {
            result.push(c);
        }

        previous_char_is_lowercase = c.is_lowercase();
    }

    result
}

fn line_col_from_pos(source: &str, pos: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;

    for (i, c) in source.char_indices() {
        if i >= pos {
            break;
        }

        if c == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }

    (line, col)
}

#[cfg(test)]
mod tests;

impl Default for Linter {
    fn default() -> Self {
        Self::new()
    }
}
