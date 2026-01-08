pub mod ast;
mod cursor;
mod expr;
mod helpers;
mod import_processor;
mod stmt;
#[cfg(test)]
mod tests;

use crate::exec_trace;
use crate::lexer::token::{Token, TokenWithPosition};
use ast::*;
pub use cursor::Cursor; // Re-export Cursor publicly for doctests
use expr::ExprParser;
use stmt::{
    ActionParser, CollectionParser, ContainerParser, ControlFlowParser, ErrorHandlingParser,
    IoParser, ModuleParser, PatternParser, ProcessParser, StmtParser, VariableParser, WebParser,
};

pub struct Parser<'a> {
    /// Cursor for efficient token navigation
    cursor: Cursor<'a>,
    /// Parse errors accumulated during parsing
    errors: Vec<ParseError>,
    /// Base path for resolving relative imports
    base_path: std::path::PathBuf,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [TokenWithPosition]) -> Self {
        Parser {
            cursor: Cursor::new(tokens),
            errors: Vec::with_capacity(4),
            base_path: std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
        }
    }

    /// Set the base path for resolving relative imports
    pub fn set_base_path(&mut self, path: std::path::PathBuf) {
        self.base_path = path;
    }

    /// Consume token from cursor and advance position.
    ///
    /// This is a convenience wrapper around `cursor.bump()` that maintains
    /// consistent naming with the rest of the parser.
    #[inline]
    fn bump_sync(&mut self) -> Option<&'a TokenWithPosition> {
        self.cursor.bump()
    }

    /// Parse without processing imports (used internally for recursive imports)
    pub(crate) fn parse_without_imports(&mut self) -> Result<Program, Vec<ParseError>> {
        let mut program = Program::new();
        program.statements.reserve(self.cursor.remaining() / 5);

        while self.cursor.peek().is_some() {
            let start_pos = self.cursor.pos();

            // Skip any leading Eol tokens
            if let Some(token) = self.cursor.peek()
                && matches!(token.token, Token::Eol)
            {
                self.bump_sync();
                continue;
            }

            // Comprehensive handling of "end" tokens that might be left unconsumed
            // Check first two tokens without cloning
            if let Some(first_token) = self.cursor.peek()
                && first_token.token == Token::KeywordEnd
            {
                if let Some(second_token) = self.cursor.peek_next() {
                    match &second_token.token {
                        Token::KeywordAction => {
                            exec_trace!(
                                "Consuming orphaned 'end action' at line {}",
                                first_token.line
                            );
                            self.bump_sync(); // Consume "end"
                            self.bump_sync(); // Consume "action"
                            continue;
                        }
                        Token::KeywordCheck => {
                            exec_trace!(
                                "Consuming orphaned 'end check' at line {}",
                                first_token.line
                            );
                            self.bump_sync(); // Consume "end"
                            self.bump_sync(); // Consume "check"
                            continue;
                        }
                        Token::KeywordFor => {
                            exec_trace!(
                                "Consuming orphaned 'end for' at line {}",
                                first_token.line
                            );
                            self.bump_sync(); // Consume "end"
                            self.bump_sync(); // Consume "for"
                            continue;
                        }
                        Token::KeywordCount => {
                            exec_trace!(
                                "Consuming orphaned 'end count' at line {}",
                                first_token.line
                            );
                            self.bump_sync(); // Consume "end"
                            self.bump_sync(); // Consume "count"
                            continue;
                        }
                        Token::KeywordRepeat => {
                            exec_trace!(
                                "Consuming orphaned 'end repeat' at line {}",
                                first_token.line
                            );
                            self.bump_sync(); // Consume "end"
                            self.bump_sync(); // Consume "repeat"
                            continue;
                        }
                        Token::KeywordTry => {
                            exec_trace!(
                                "Consuming orphaned 'end try' at line {}",
                                first_token.line
                            );
                            self.bump_sync(); // Consume "end"
                            self.bump_sync(); // Consume "try"
                            continue;
                        }
                        Token::KeywordLoop => {
                            exec_trace!(
                                "Consuming orphaned 'end loop' at line {}",
                                first_token.line
                            );
                            self.bump_sync(); // Consume "end"
                            self.bump_sync(); // Consume "loop"
                            continue;
                        }
                        Token::KeywordMap => {
                            exec_trace!(
                                "Consuming orphaned 'end map' at line {}",
                                first_token.line
                            );
                            self.bump_sync(); // Consume "end"
                            self.bump_sync(); // Consume "map"
                            continue;
                        }
                        Token::KeywordWhile => {
                            exec_trace!(
                                "Consuming orphaned 'end while' at line {}",
                                first_token.line
                            );
                            self.bump_sync(); // Consume "end"
                            self.bump_sync(); // Consume "while"
                            continue;
                        }
                        Token::KeywordPattern => {
                            exec_trace!(
                                "Consuming orphaned 'end pattern' at line {}",
                                first_token.line
                            );
                            self.bump_sync(); // Consume "end"
                            self.bump_sync(); // Consume "pattern"
                            continue;
                        }
                        Token::KeywordList => {
                            exec_trace!(
                                "Consuming orphaned 'end list' at line {}",
                                first_token.line
                            );
                            self.bump_sync(); // Consume "end"
                            self.bump_sync(); // Consume "list"
                            continue;
                        }
                        Token::KeywordContainer => {
                            exec_trace!(
                                "Consuming orphaned 'end container' at line {}",
                                first_token.line
                            );
                            self.bump_sync(); // Consume "end"
                            self.bump_sync(); // Consume "container"
                            continue;
                        }
                        Token::Eol => {
                            // Standalone "end" on its own line - consume it
                            exec_trace!("Found standalone 'end' at line {}", first_token.line);
                            self.bump_sync(); // Consume "end"
                            continue;
                        }
                        _ => {
                            // Standalone "end" or unexpected pattern - consume and log error
                            exec_trace!(
                                "Found unexpected 'end' followed by {:?} at line {}",
                                second_token.token,
                                first_token.line
                            );
                            self.bump_sync(); // Consume "end"
                            self.errors.push(ParseError::from_token(
                                format!("Unexpected 'end' followed by {:?}", second_token.token),
                                first_token,
                            ));
                            continue;
                        }
                    }
                } else {
                    // "end" at end of file
                    exec_trace!(
                        "Found standalone 'end' at end of file, line {}",
                        first_token.line
                    );
                    self.bump_sync();
                    break;
                }
            }

            match self.parse_statement() {
                Ok(statement) => {
                    program.statements.push(statement);

                    // Consume trailing Eol tokens (blank lines between statements)
                    while let Some(token) = self.cursor.peek() {
                        if matches!(token.token, Token::Eol) {
                            self.bump_sync();
                        } else {
                            break;
                        }
                    }
                }
                Err(error) => {
                    self.errors.push(error);

                    // Skip tokens until we reach Eol or statement starter
                    while let Some(token) = self.cursor.peek() {
                        if matches!(token.token, Token::Eol)
                            || Parser::is_statement_starter(&token.token)
                        {
                            break;
                        }
                        self.bump_sync(); // Skip token
                    }

                    // Consume trailing Eol tokens if any
                    while let Some(token) = self.cursor.peek() {
                        if matches!(token.token, Token::Eol) {
                            self.bump_sync();
                        } else {
                            break;
                        }
                    }
                }
            }

            // Special case for end of file - if we have processed all meaningful tokens,
            // and only trailing tokens remain (if any), just break
            if let Some(token) = self.cursor.peek()
                && token.token == Token::KeywordEnd
                && self.cursor.remaining() <= 2
            {
                // If we're at the end with just 1-2 tokens left, consume them and break
                while self.bump_sync().is_some() {}
                break;
            }

            assert!(
                self.cursor.pos() > start_pos,
                "Parser made no progress at line {} (stuck at position {}) - token {:?} caused infinite loop",
                self.cursor.current_line(),
                start_pos,
                self.cursor.peek()
            );
        }

        if self.errors.is_empty() {
            Ok(program)
        } else {
            Err(self.errors.clone())
        }
    }

    /// Parse a WFL program and process all imports
    pub fn parse(&mut self) -> Result<Program, Vec<ParseError>> {
        let program = self.parse_without_imports()?;
        // Process imports to inline imported files
        import_processor::process_imports(program, &self.base_path)
    }

    // Container-related parsing methods

    /// Helper method to parse a variable name that can consist of multiple identifiers.
    /// Used by variable declarations and other statement parsers.
    fn parse_variable_name_list(&mut self) -> Result<String, ParseError> {
        let mut name_parts = Vec::with_capacity(3);

        if let Some(token) = self.cursor.peek().cloned() {
            match &token.token {
                Token::Identifier(id) => {
                    self.bump_sync(); // Consume the identifier
                    name_parts.push(id.clone());
                }
                Token::IntLiteral(_) | Token::FloatLiteral(_) => {
                    return Err(ParseError::from_token(
                        format!("Cannot use a number as a variable name: {:?}", token.token),
                        &token,
                    ));
                }
                Token::KeywordAs => {
                    return Err(ParseError::from_token(
                        "Expected a variable name before 'as'".to_string(),
                        &token,
                    ));
                }
                _ if token.token.is_structural_keyword() => {
                    return Err(ParseError::from_token(
                        format!(
                            "Cannot use reserved keyword '{:?}' as a variable name",
                            token.token
                        ),
                        &token,
                    ));
                }
                _ if token.token.is_contextual_keyword() => {
                    // Contextual keywords can be used as variable names
                    let name = self.get_token_text(&token.token);
                    self.bump_sync(); // Consume the contextual keyword
                    name_parts.push(name);
                }
                _ => {
                    return Err(ParseError::from_token(
                        format!(
                            "Expected identifier for variable name, found {:?}",
                            token.token
                        ),
                        &token,
                    ));
                }
            }
        } else {
            return Err(ParseError::from_span(
                "Expected variable name but found end of input".to_string(),
                crate::diagnostics::Span { start: 0, end: 0 },
                0,
                0,
            ));
        }

        while let Some(token) = self.cursor.peek().cloned() {
            match &token.token {
                Token::Identifier(id) => {
                    self.bump_sync(); // Consume the identifier
                    name_parts.push(id.clone());
                }
                Token::KeywordAs => {
                    break;
                }
                Token::IntLiteral(_) | Token::FloatLiteral(_) => {
                    return Err(ParseError::from_token(
                        format!(
                            "Expected 'as' after variable name, but found number: {:?}",
                            token.token
                        ),
                        &token,
                    ));
                }
                _ => {
                    return Err(ParseError::from_token(
                        format!(
                            "Expected 'as' after variable name, but found {:?}",
                            token.token
                        ),
                        &token,
                    ));
                }
            }
        }

        Ok(name_parts.join(" "))
    }

    /// Helper method to parse a simple variable name (space-separated identifiers).
    /// Used by assignments, arithmetic operations, and other statement parsers.
    fn parse_variable_name_simple(&mut self) -> Result<String, ParseError> {
        let mut name = String::new();
        let mut has_identifier = false;

        while let Some(token) = self.cursor.peek().cloned() {
            if let Token::Identifier(id) = &token.token {
                has_identifier = true;
                if !name.is_empty() {
                    name.push(' ');
                }
                name.push_str(id);
                self.bump_sync();
            } else {
                break;
            }
        }

        if !has_identifier {
            if let Some(token) = self.cursor.peek() {
                return Err(ParseError::from_token(
                    "Expected variable name".to_string(),
                    token,
                ));
            } else {
                return Err(ParseError::from_span(
                    "Expected variable name".to_string(),
                    crate::diagnostics::Span { start: 0, end: 0 },
                    0,
                    0,
                ));
            }
        }

        Ok(name)
    }

    // Subprocess parsing functions
    // Web server parsing methods
}

// Implementation of StmtParser trait
impl<'a> StmtParser<'a> for Parser<'a> {
    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        if let Some(token) = self.cursor.peek().cloned() {
            match &token.token {
                Token::KeywordStore => self.parse_variable_declaration(),
                Token::KeywordCreate => {
                    // Check what follows "create" keyword
                    if let Some(next_token) = self.cursor.peek_next() {
                        match &next_token.token {
                            Token::KeywordContainer => self.parse_container_definition(),
                            Token::KeywordInterface => self.parse_interface_definition(),
                            Token::KeywordNew => self.parse_container_instantiation(),
                            Token::KeywordPattern => self.parse_create_pattern_statement(),
                            Token::KeywordDirectory => self.parse_create_directory_statement(),
                            Token::KeywordFile => self.parse_create_file_statement(),
                            Token::KeywordList => self.parse_create_list_statement(),
                            Token::KeywordMap => self.parse_map_creation(),
                            Token::KeywordDate => self.parse_create_date_statement(),
                            Token::KeywordTime => self.parse_create_time_statement(),
                            _ => self.parse_variable_declaration(),
                        }
                    } else {
                        self.parse_variable_declaration()
                    }
                }
                Token::KeywordDisplay => self.parse_display_statement(),
                Token::KeywordCheck => self.parse_if_statement(),
                Token::KeywordIf => self.parse_single_line_if(),
                Token::KeywordCount => self.parse_count_loop(),
                Token::KeywordFor => self.parse_for_each_loop(),
                Token::KeywordDefine => self.parse_action_definition(),
                Token::KeywordChange => self.parse_assignment(),
                Token::KeywordAdd => {
                    // Peek ahead to determine if this is arithmetic or list operation
                    // For arithmetic: "add 5 to variable" (number comes first)
                    // For list: "add "item" to list" (any value to a list)
                    // We'll try to parse as list operation first, fall back to arithmetic
                    self.parse_add_operation()
                }
                Token::KeywordSubtract => self.parse_arithmetic_operation(),
                Token::KeywordMultiply => self.parse_arithmetic_operation(),
                Token::KeywordDivide => self.parse_arithmetic_operation(),
                Token::KeywordRemove => self.parse_remove_from_list_statement(),
                Token::KeywordClear => self.parse_clear_list_statement(),
                Token::KeywordTry => self.parse_try_statement(),
                Token::KeywordRepeat => self.parse_repeat_statement(),
                Token::KeywordExit => self.parse_exit_statement(),
                Token::KeywordPush => self.parse_push_statement(),
                Token::KeywordEvent => self.parse_event_definition(),
                Token::KeywordTrigger => self.parse_event_trigger(),
                Token::KeywordOn => self.parse_event_handler(),
                Token::KeywordParent => self.parse_parent_method_call(),
                Token::KeywordBreak => {
                    let token_pos = self.bump_sync().unwrap();
                    Ok(Statement::BreakStatement {
                        line: token_pos.line,
                        column: token_pos.column,
                    })
                }
                Token::KeywordContinue | Token::KeywordSkip => {
                    let token_pos = self.bump_sync().unwrap();
                    Ok(Statement::ContinueStatement {
                        line: token_pos.line,
                        column: token_pos.column,
                    })
                }
                Token::KeywordOpen => {
                    // Parse open file statement (handles both regular and "read content" variants)
                    self.parse_open_file_statement()
                }
                Token::KeywordLoad => {
                    // Parse load module statement
                    self.parse_load_statement()
                }
                Token::KeywordExecute => self.parse_execute_command_statement(),
                Token::KeywordSpawn => self.parse_spawn_process_statement(),
                Token::KeywordKill => self.parse_kill_process_statement(),
                Token::KeywordRead => {
                    // Look ahead to distinguish "read output from process" from other read variants
                    if let Some(next_token) = self.cursor.peek_next() {
                        if matches!(next_token.token, Token::KeywordOutput) {
                            // It's "read output from process"
                            self.parse_read_process_output_statement()
                        } else {
                            // "read" by itself is not a valid statement - treat as expression
                            let token_pos = self.cursor.peek().unwrap();
                            Err(ParseError::from_token(
                                "Unexpected 'read' - did you mean 'read output from process'?"
                                    .to_string(),
                                token_pos,
                            ))
                        }
                    } else {
                        let token_pos = self.cursor.peek().unwrap();
                        Err(ParseError::from_token(
                            "Unexpected 'read' at end of input".to_string(),
                            token_pos,
                        ))
                    }
                }
                Token::KeywordClose => {
                    // Check if it's "close server" or regular "close file"
                    if let Some(next_token) = self.cursor.peek_next() {
                        if matches!(next_token.token, Token::KeywordServer) {
                            self.parse_close_server_statement()
                        } else {
                            self.parse_close_file_statement()
                        }
                    } else {
                        self.parse_close_file_statement()
                    }
                }
                Token::KeywordDelete => self.parse_delete_statement(),
                Token::KeywordWrite => self.parse_write_to_statement(),
                Token::KeywordWait => self.parse_wait_for_statement(),
                Token::KeywordListen => self.parse_listen_statement(),
                Token::KeywordRespond => self.parse_respond_statement(),
                Token::KeywordRegister => self.parse_register_signal_handler_statement(),
                Token::KeywordStop => self.parse_stop_accepting_connections_statement(),
                Token::KeywordGive | Token::KeywordReturn => self.parse_return_statement(),
                Token::Identifier(id) if id == "main" => {
                    // Check if next token is "loop"
                    if let Some(next_token) = self.cursor.peek_next() {
                        if matches!(next_token.token, Token::KeywordLoop) {
                            self.parse_main_loop()
                        } else {
                            self.parse_expression_statement()
                        }
                    } else {
                        self.parse_expression_statement()
                    }
                }
                _ => self.parse_expression_statement(),
            }
        } else {
            Err(self.cursor.error("Unexpected end of input".to_string()))
        }
    }

    fn parse_expression_statement(&mut self) -> Result<Statement, ParseError> {
        let expr = self.parse_expression()?;

        let default_token = TokenWithPosition {
            token: Token::Identifier("expression".to_string()),
            line: 0,
            column: 0,
            length: 0,
            byte_start: 0,
            byte_end: 0,
        };
        let token_pos = self.cursor.peek().map_or(&default_token, |v| v);
        Ok(Statement::ExpressionStatement {
            expression: expr,
            line: token_pos.line,
            column: token_pos.column,
        })
    }
}
