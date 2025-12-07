// This file contains the complete parser implementation
// Will replace mod.rs when finished

pub mod ast;
#[cfg(test)]
mod tests;

use crate::exec_trace;
use crate::lexer::token::{Token, TokenWithPosition};
use ast::*;
use std::iter::Peekable;
use std::slice::Iter;

pub struct Parser<'a> {
    tokens: Peekable<Iter<'a, TokenWithPosition>>,
    errors: Vec<ParseError>,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [TokenWithPosition]) -> Self {
        Parser {
            tokens: tokens.iter().peekable(),
            errors: Vec::with_capacity(4),
        }
    }

    pub fn parse(&mut self) -> Result<Program, Vec<ParseError>> {
        let mut program = Program::new();
        program.statements.reserve(self.tokens.clone().count() / 5);

        while self.tokens.peek().is_some() {
            let start_len = self.tokens.clone().count();

            // Comprehensive handling of "end" tokens that might be left unconsumed
            // Check first two tokens to avoid borrow checker issues
            let mut tokens_clone = self.tokens.clone();
            if let Some(first_token) = tokens_clone.next() {
                if first_token.token == Token::KeywordEnd {
                    if let Some(second_token) = tokens_clone.next() {
                        match &second_token.token {
                            Token::KeywordAction => {
                                exec_trace!("Consuming orphaned 'end action' at line {}", first_token.line);
                                self.tokens.next(); // Consume "end"
                                self.tokens.next(); // Consume "action"
                                continue;
                            }
                            Token::KeywordCheck => {
                                exec_trace!("Consuming orphaned 'end check' at line {}", first_token.line);
                                self.tokens.next(); // Consume "end"
                                self.tokens.next(); // Consume "check"
                                continue;
                            }
                            Token::KeywordFor => {
                                exec_trace!("Consuming orphaned 'end for' at line {}", first_token.line);
                                self.tokens.next(); // Consume "end"
                                self.tokens.next(); // Consume "for"
                                continue;
                            }
                            Token::KeywordCount => {
                                exec_trace!("Consuming orphaned 'end count' at line {}", first_token.line);
                                self.tokens.next(); // Consume "end"
                                self.tokens.next(); // Consume "count"
                                continue;
                            }
                            Token::KeywordRepeat => {
                                exec_trace!("Consuming orphaned 'end repeat' at line {}", first_token.line);
                                self.tokens.next(); // Consume "end"
                                self.tokens.next(); // Consume "repeat"
                                continue;
                            }
                            Token::KeywordTry => {
                                exec_trace!("Consuming orphaned 'end try' at line {}", first_token.line);
                                self.tokens.next(); // Consume "end"
                                self.tokens.next(); // Consume "try"
                                continue;
                            }
                            Token::KeywordLoop => {
                                exec_trace!("Consuming orphaned 'end loop' at line {}", first_token.line);
                                self.tokens.next(); // Consume "end"
                                self.tokens.next(); // Consume "loop"
                                continue;
                            }
                            Token::KeywordWhile => {
                                exec_trace!("Consuming orphaned 'end while' at line {}", first_token.line);
                                self.tokens.next(); // Consume "end"
                                self.tokens.next(); // Consume "while"
                                continue;
                            }
                            _ => {
                                // Standalone "end" or unexpected pattern - consume and log error
                                exec_trace!("Found unexpected 'end' followed by {:?} at line {}", second_token.token, first_token.line);
                                self.tokens.next(); // Consume "end"
                                self.errors.push(ParseError::new(
                                    format!("Unexpected 'end' followed by {:?}", second_token.token),
                                    first_token.line,
                                    first_token.column,
                                ));
                                continue;
                            }
                        }
                    } else {
                        // "end" at end of file
                        exec_trace!("Found standalone 'end' at end of file, line {}", first_token.line);
                        self.tokens.next();
                        break;
                    }
                }
            }

            match self.parse_statement() {
                Ok(statement) => program.statements.push(statement),
                Err(error) => {
                    self.errors.push(error);
                    self.synchronize(); // Skip to next statement on error
                }
            }

            let end_len = self.tokens.clone().count();

            // Special case for end of file - if we have processed all meaningful tokens,
            // and only trailing tokens remain (if any), just break
            if let Some(token) = self.tokens.peek() {
                if token.token == Token::KeywordEnd && start_len <= 2 {
                    // If we're at the end with just 1-2 tokens left, consume them and break
                    while self.tokens.next().is_some() {}
                    break;
                }
            }

            assert!(
                end_len < start_len,
                "Parser made no progress - token {:?} caused infinite loop",
                self.tokens.peek()
            );
        }

        if self.errors.is_empty() {
            Ok(program)
        } else {
            Err(self.errors.clone())
        }
    }

    fn synchronize(&mut self) {
        while let Some(token) = self.tokens.peek().cloned() {
            match &token.token {
                Token::KeywordStore
                | Token::KeywordCreate
                | Token::KeywordDisplay
                | Token::KeywordCheck
                | Token::KeywordCount
                | Token::KeywordFor
                | Token::KeywordDefine
                | Token::KeywordIf => {
                    break;
                }
                Token::KeywordEnd => {
                    // Handle orphaned "end" tokens during error recovery
                    exec_trace!("Synchronizing: found 'end' token at line {}", token.line);
                    self.tokens.next(); // Consume "end"
                    if let Some(next_token) = self.tokens.peek() {
                        match &next_token.token {
                            Token::KeywordAction
                            | Token::KeywordCheck
                            | Token::KeywordFor
                            | Token::KeywordCount
                            | Token::KeywordRepeat
                            | Token::KeywordTry
                            | Token::KeywordLoop
                            | Token::KeywordWhile => {
                                exec_trace!("Synchronizing: consuming {:?} after 'end'", next_token.token);
                                self.tokens.next(); // Consume the keyword after "end"
                            }
                            _ => {} // Just consumed "end", continue
                        }
                    }
                    break; // After handling orphaned end, continue with recovery
                }
                _ => {
                    self.tokens.next(); // Skip the token
                }
            }
        }
    }

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        if let Some(token) = self.tokens.peek().cloned() {
            match &token.token {
                Token::KeywordStore | Token::KeywordCreate => self.parse_variable_declaration(),
                Token::KeywordDisplay => self.parse_display_statement(),
                Token::KeywordCheck => self.parse_if_statement(),
                Token::KeywordIf => self.parse_single_line_if(),
                Token::KeywordCount => self.parse_count_loop(),
                Token::KeywordFor => self.parse_for_each_loop(),
                Token::KeywordDefine => self.parse_action_definition(),
                Token::KeywordChange => self.parse_assignment(),
                Token::KeywordBreak => {
                    let token_pos = self.tokens.next().unwrap();
                    Ok(Statement::BreakStatement {
                        line: token_pos.line,
                        column: token_pos.column,
                    })
                }
                Token::KeywordContinue | Token::KeywordSkip => {
                    let token_pos = self.tokens.next().unwrap();
                    Ok(Statement::ContinueStatement {
                        line: token_pos.line,
                        column: token_pos.column,
                    })
                }
                Token::KeywordOpen => self.parse_open_file_statement(),
                Token::KeywordClose => {
                    if let Some(next_token) = self.tokens.clone().nth(1) {
                        if next_token.token == Token::KeywordFile {
                            self.parse_close_file_statement()
                        } else {
                            self.parse_expression_statement()
                        }
                    } else {
                        self.parse_expression_statement()
                    }
                }
                Token::KeywordWait => self.parse_wait_for_statement(),
                Token::KeywordGive | Token::KeywordReturn => self.parse_return_statement(),
                Token::KeywordTry => self.parse_try_statement(),
                Token::KeywordRepeat => self.parse_repeat_statement(),
                Token::KeywordExit => self.parse_exit_statement(),
                // Add push statement parsing
                Token::Identifier(id) if id == "push" => self.parse_push_statement(),
                _ => self.parse_expression_statement(),
            }
        } else {
            Err(self.cursor.error("Unexpected end of input".to_string()))
        }
    }

    fn parse_variable_declaration(&mut self) -> Result<Statement, ParseError> {
        let token_pos = self.tokens.next().unwrap();
        let is_store = matches!(token_pos.token, Token::KeywordStore);

        let name = self.parse_variable_name_list()?;

        // Handle special case: "create list as name"
        if !is_store && name == "list" {
            self.expect_token(Token::KeywordAs, "Expected 'as' after 'list'")?;
            
            let list_name = if let Some(token) = self.tokens.peek() {
                if let Token::Identifier(id) = &token.token {
                    self.tokens.next();
                    id.clone()
                } else {
                    return Err(ParseError::new(
                        format!("Expected identifier after 'as', found {:?}", token.token),
                        token.line,
                        token.column,
                    ));
                }
            } else {
                return Err(ParseError::from_token("Expected identifier after 'as'".to_string(), &token_pos));
            };

            // Create an empty list literal
            let empty_list = Expression::Literal(
                Literal::List(Vec::new()),
                token_pos.line,
                token_pos.column,
            );

            return Ok(Statement::VariableDeclaration {
                name: list_name,
                value: empty_list,
                line: token_pos.line,
                column: token_pos.column,
            });
        }

        self.expect_token(Token::KeywordAs, "Expected 'as' after variable name")?;
        let value = self.parse_expression()?;

        Ok(Statement::VariableDeclaration {
            name,
            value,
            line: token_pos.line,
            column: token_pos.column,
        })
    }

    fn parse_push_statement(&mut self) -> Result<Statement, ParseError> {
        let push_token = self.tokens.next().unwrap(); // Consume "push"
        
        self.expect_token(Token::KeywordWith, "Expected 'with' after 'push'")?;
        let list_expr = self.parse_expression()?;
        self.expect_token(Token::KeywordAnd, "Expected 'and' after list expression")?;
        let value_expr = self.parse_expression()?;

        // For now, represent push as an expression statement
        Ok(Statement::ExpressionStatement {
            expression: Expression::FunctionCall {
                function: Box::new(Expression::Variable("push".to_string(), push_token.line, push_token.column)),
                arguments: vec![
                    Argument { name: None, value: list_expr },
                    Argument { name: None, value: value_expr },
                ],
                line: push_token.line,
                column: push_token.column,
            },
            line: push_token.line,
            column: push_token.column,
        })
    }

    fn parse_variable_name_list(&mut self) -> Result<String, ParseError> {
        let mut name_parts = Vec::with_capacity(3);

        if let Some(token) = self.tokens.peek().cloned() {
            match &token.token {
                Token::Identifier(id) => {
                    self.tokens.next();
                    name_parts.push(id.clone());
                }
                _ => {
                    return Err(ParseError::new(
                        format!("Expected identifier for variable name, found {:?}", token.token),
                        token.line,
                        token.column,
                    ));
                }
            }
        } else {
            return Err(ParseError::new(
                "Expected variable name but found end of input".to_string(),
                0,
                0,
            ));
        }

        while let Some(token) = self.tokens.peek().cloned() {
            match &token.token {
                Token::Identifier(id) => {
                    self.tokens.next();
                    name_parts.push(id.clone());
                }
                Token::KeywordAs => break,
                _ => break,
            }
        }

        Ok(name_parts.join(" "))
    }

    fn expect_token(&mut self, expected: Token, error_message: &str) -> Result<(), ParseError> {
        if let Some(token) = self.tokens.peek().cloned() {
            if token.token == expected {
                self.tokens.next();
                Ok(())
            } else {
                Err(ParseError::new(
                    format!("{}: expected {:?}, found {:?}", error_message, expected, token.token),
                    token.line,
                    token.column,
                ))
            }
        } else {
            Err(ParseError::new(
                format!("{}: unexpected end of input", error_message),
                0,
                0,
            ))
        }
    }

    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        self.parse_binary_expression(0)
    }

    fn parse_binary_expression(&mut self, precedence: u8) -> Result<Expression, ParseError> {
        let mut left = self.parse_primary_expression()?;

        while let Some(token_pos) = self.tokens.peek().cloned() {
            let op = match &token_pos.token {
                Token::KeywordPlus => {
                    self.tokens.next();
                    Some((Operator::Plus, 1))
                }
                Token::KeywordMinus => {
                    self.tokens.next();
                    Some((Operator::Minus, 1))
                }
                Token::KeywordTimes => {
                    self.tokens.next();
                    Some((Operator::Multiply, 2))
                }
                Token::KeywordDividedBy => {
                    self.tokens.next();
                    Some((Operator::Divide, 2))
                }
                Token::KeywordIs => {
                    self.tokens.next();
                    if let Some(next_token) = self.tokens.peek().cloned() {
                        match &next_token.token {
                            Token::KeywordEqual => {
                                self.tokens.next();
                                if let Some(to_token) = self.tokens.peek().cloned() {
                                    if matches!(to_token.token, Token::KeywordTo) {
                                        self.tokens.next();
                                    }
                                }
                                Some((Operator::Equals, 0))
                            }
                            Token::KeywordGreater => {
                                self.tokens.next();
                                if let Some(than_token) = self.tokens.peek().cloned() {
                                    if let Token::Identifier(id) = &than_token.token {
                                        if id == "than" {
                                            self.tokens.next();
                                        }
                                    }
                                }
                                Some((Operator::GreaterThan, 0))
                            }
                            Token::KeywordLess => {
                                self.tokens.next();
                                if let Some(than_token) = self.tokens.peek().cloned() {
                                    if let Token::Identifier(id) = &than_token.token {
                                        if id == "than" {
                                            self.tokens.next();
                                        }
                                    }
                                }
                                Some((Operator::LessThan, 0))
                            }
                            _ => Some((Operator::Equals, 0)),
                        }
                    } else {
                        Some((Operator::Equals, 0))
                    }
                }
                Token::KeywordWith => {
                    // With the introduction of 'call' keyword, we still support
                    // legacy syntax for builtin functions: `builtinName with args`
                    if let Expression::Variable(ref name, var_line, var_column) = left {
                        if crate::builtins::is_builtin_function(name) {
                            self.tokens.next();
                            let arguments = self.parse_argument_list()?;
                            left = Expression::ActionCall {
                                name: name.clone(),
                                arguments,
                                line: var_line,
                                column: var_column,
                            };
                            continue;
                        }
                    }
                    self.tokens.next();
                    let right = self.parse_expression()?;
                    left = Expression::Concatenation {
                        left: Box::new(left),
                        right: Box::new(right),
                        line: token_pos.line,
                        column: token_pos.column,
                    };
                    continue;
                }
                Token::KeywordAnd => {
                    self.tokens.next();
                    Some((Operator::And, 0))
                }
                Token::KeywordOr => {
                    self.tokens.next();
                    Some((Operator::Or, 0))
                }
                _ => None,
            };

            if let Some((operator, op_precedence)) = op {
                if op_precedence < precedence {
                    break;
                }
                let right = self.parse_binary_expression(op_precedence + 1)?;
                left = Expression::BinaryOperation {
                    left: Box::new(left),
                    operator,
                    right: Box::new(right),
                    line: token_pos.line,
                    column: token_pos.column,
                };
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_primary_expression(&mut self) -> Result<Expression, ParseError> {
        if let Some(token) = self.tokens.peek().cloned() {
            match &token.token {
                Token::StringLiteral(s) => {
                    let token_pos = self.tokens.next().unwrap();
                    Ok(Expression::Literal(
                        Literal::String(s.to_string()),
                        token_pos.line,
                        token_pos.column,
                    ))
                }
                Token::IntLiteral(n) => {
                    let token_pos = self.tokens.next().unwrap();
                    Ok(Expression::Literal(
                        Literal::Integer(*n),
                        token_pos.line,
                        token_pos.column,
                    ))
                }
                Token::FloatLiteral(f) => {
                    let token_pos = self.tokens.next().unwrap();
                    Ok(Expression::Literal(
                        Literal::Float(*f),
                        token_pos.line,
                        token_pos.column,
                    ))
                }
                Token::BooleanLiteral(b) => {
                    let token_pos = self.tokens.next().unwrap();
                    Ok(Expression::Literal(
                        Literal::Boolean(*b),
                        token_pos.line,
                        token_pos.column,
                    ))
                }
                Token::NothingLiteral => {
                    let token_pos = self.tokens.next().unwrap();
                    Ok(Expression::Literal(
                        Literal::Nothing,
                        token_pos.line,
                        token_pos.column,
                    ))
                }
                Token::Identifier(name) => {
                    let token_pos = self.tokens.next().unwrap();
                    Ok(Expression::Variable(name.clone(), token_pos.line, token_pos.column))
                }
                Token::KeywordCount => {
                    let token_pos = self.tokens.next().unwrap();
                    Ok(Expression::Variable("count".to_string(), token_pos.line, token_pos.column))
                }
                _ => Err(ParseError::new(
                    format!("Unexpected token in expression: {:?}", token.token),
                    token.line,
                    token.column,
                )),
            }
        } else {
            Err(ParseError::new(
                "Unexpected end of input while parsing expression".to_string(),
                0,
                0,
            ))
        }
    }

    // Simplified implementations for remaining methods
    fn parse_display_statement(&mut self) -> Result<Statement, ParseError> {
        let token_pos = self.tokens.next().unwrap(); // Consume "display"
        let expr = self.parse_expression()?;
        Ok(Statement::DisplayStatement {
            value: expr,
            line: token_pos.line,
            column: token_pos.column,
        })
    }

    fn parse_if_statement(&mut self) -> Result<Statement, ParseError> {
        let check_token = self.tokens.next().unwrap(); // Consume "check"
        self.expect_token(Token::KeywordIf, "Expected 'if' after 'check'")?;
        let condition = self.parse_expression()?;
        self.expect_token(Token::Colon, "Expected ':' after if condition")?;

        let mut then_block = Vec::new();
        while let Some(token) = self.tokens.peek().cloned() {
            if matches!(token.token, Token::KeywordOtherwise | Token::KeywordEnd) {
                break;
            }
            then_block.push(self.parse_statement()?);
        }

        let else_block = if let Some(token) = self.tokens.peek() {
            if matches!(token.token, Token::KeywordOtherwise) {
                self.tokens.next(); // Consume "otherwise"
                self.expect_token(Token::Colon, "Expected ':' after 'otherwise'")?;
                let mut else_stmts = Vec::new();
                while let Some(token) = self.tokens.peek().cloned() {
                    if matches!(token.token, Token::KeywordEnd) {
                        break;
                    }
                    else_stmts.push(self.parse_statement()?);
                }
                Some(else_stmts)
            } else {
                None
            }
        } else {
            None
        };

        self.expect_token(Token::KeywordEnd, "Expected 'end' after if block")?;
        self.expect_token(Token::KeywordCheck, "Expected 'check' after 'end'")?;

        Ok(Statement::IfStatement {
            condition,
            then_block,
            else_block,
            line: check_token.line,
            column: check_token.column,
        })
    }

    // Add other methods with simplified implementations
    fn parse_single_line_if(&mut self) -> Result<Statement, ParseError> {
        let if_token = self.tokens.next().unwrap();
        let condition = self.parse_expression()?;
        self.expect_token(Token::KeywordThen, "Expected 'then' after if condition")?;
        let then_stmt = Box::new(self.parse_statement()?);
        let else_stmt = if let Some(token) = self.tokens.peek() {
            if matches!(token.token, Token::KeywordOtherwise) {
                self.tokens.next();
                Some(Box::new(self.parse_statement()?))
            } else {
                None
            }
        } else {
            None
        };

        Ok(Statement::SingleLineIf {
            condition,
            then_stmt,
            else_stmt,
            line: if_token.line,
            column: if_token.column,
        })
    }

    fn parse_count_loop(&mut self) -> Result<Statement, ParseError> {
        let count_token = self.tokens.next().unwrap();
        self.expect_token(Token::KeywordFrom, "Expected 'from' after 'count'")?;
        let start = self.parse_expression()?;
        self.expect_token(Token::KeywordTo, "Expected 'to' after start expression")?;
        let end = self.parse_expression()?;
        self.expect_token(Token::Colon, "Expected ':' after count range")?;

        let mut body = Vec::new();
        while let Some(token) = self.tokens.peek().cloned() {
            if matches!(token.token, Token::KeywordEnd) {
                break;
            }
            body.push(self.parse_statement()?);
        }

        self.expect_token(Token::KeywordEnd, "Expected 'end' after count loop body")?;
        self.expect_token(Token::KeywordCount, "Expected 'count' after 'end'")?;

        Ok(Statement::CountLoop {
            start,
            end,
            step: None,
            downward: false,
            body,
            line: count_token.line,
            column: count_token.column,
        })
    }

    fn parse_for_each_loop(&mut self) -> Result<Statement, ParseError> {
        let for_token = self.tokens.next().unwrap();
        self.expect_token(Token::KeywordEach, "Expected 'each' after 'for'")?;
        
        let item_name = if let Some(token) = self.tokens.peek() {
            if let Token::Identifier(id) = &token.token {
                self.tokens.next();
                id.clone()
            } else {
                return Err(ParseError::new(
                    format!("Expected identifier after 'each', found {:?}", token.token),
                    token.line,
                    token.column,
                ));
            }
        } else {
            return Err(ParseError::new(
                "Unexpected end of input after 'each'".to_string(),
                0,
                0,
            ));
        };

        self.expect_token(Token::KeywordIn, "Expected 'in' after item name")?;
        let collection = self.parse_expression()?;
        self.expect_token(Token::Colon, "Expected ':' after collection")?;

        let mut body = Vec::new();
        while let Some(token) = self.tokens.peek().cloned() {
            if matches!(token.token, Token::KeywordEnd) {
                break;
            }
            body.push(self.parse_statement()?);
        }

        self.expect_token(Token::KeywordEnd, "Expected 'end' after for-each loop body")?;
        self.expect_token(Token::KeywordFor, "Expected 'for' after 'end'")?;

        Ok(Statement::ForEachLoop {
            item_name,
            collection,
            reversed: false,
            body,
            line: for_token.line,
            column: for_token.column,
        })
    }

    fn parse_action_definition(&mut self) -> Result<Statement, ParseError> {
        let define_token = self.tokens.next().unwrap();
        self.expect_token(Token::KeywordAction, "Expected 'action' after 'define'")?;
        self.expect_token(Token::KeywordCalled, "Expected 'called' after 'action'")?;

        let name = if let Some(token) = self.tokens.peek() {
            if let Token::Identifier(id) = &token.token {
                self.tokens.next();
                id.clone()
            } else {
                return Err(ParseError::new(
                    format!("Expected identifier after 'called', found {:?}", token.token),
                    token.line,
                    token.column,
                ));
            }
        } else {
            return Err(ParseError::new(
                "Unexpected end of input after 'called'".to_string(),
                0,
                0,
            ));
        };

        // Simplified parameter parsing
        let mut parameters = Vec::new();
        if let Some(token) = self.tokens.peek().cloned() {
            if matches!(token.token, Token::KeywordNeeds) {
                self.tokens.next(); // Consume "needs"
                while let Some(token) = self.tokens.peek().cloned() {
                    if let Token::Identifier(param_name) = &token.token {
                        self.tokens.next();
                        parameters.push(Parameter {
                            name: param_name.clone(),
                            param_type: None,
                            default_value: None,
                        });
                        if let Some(and_token) = self.tokens.peek().cloned() {
                            if let Token::KeywordAnd = &and_token.token {
                                self.tokens.next(); // Consume "and"
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
        }

        self.expect_token(Token::Colon, "Expected ':' after action definition")?;

        let mut body = Vec::new();
        while let Some(token) = self.tokens.peek().cloned() {
            if matches!(token.token, Token::KeywordEnd) {
                break;
            }
            body.push(self.parse_statement()?);
        }

        self.expect_token(Token::KeywordEnd, "Expected 'end' after action body")?;
        self.expect_token(Token::KeywordAction, "Expected 'action' after 'end'")?;

        Ok(Statement::ActionDefinition {
            name,
            parameters,
            body,
            return_type: None,
            line: define_token.line,
            column: define_token.column,
        })
    }

    fn parse_assignment(&mut self) -> Result<Statement, ParseError> {
        let change_token = self.tokens.next().unwrap();
        
        let name = if let Some(token) = self.tokens.peek() {
            if let Token::Identifier(id) = &token.token {
                self.tokens.next();
                id.clone()
            } else {
                return Err(ParseError::new(
                    format!("Expected identifier after 'change', found {:?}", token.token),
                    token.line,
                    token.column,
                ));
            }
