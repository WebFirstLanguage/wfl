use super::Parser;
use super::ast::*;
use super::error::ParseError;
use crate::exec_trace;
use crate::lexer::token::Token;

impl<'a> Parser<'a> {
    pub(crate) fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        if let Some(token) = self.tokens.peek().cloned() {
            exec_trace!("parse_statement dispatch on token: {:?}", token.token);
            if matches!(token.token, Token::KeywordEnd) {
                return Err(ParseError::new(
                    "Unexpected 'end' token".to_string(),
                    token.line,
                    token.column,
                ));
            }
            match &token.token {
                Token::KeywordStore => self.parse_variable_declaration(),
                Token::KeywordCreate => {
                    let mut tokens_clone = self.tokens.clone();
                    tokens_clone.next();
                    if let Some(next_token) = tokens_clone.next() {
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
                            Token::KeywordConstant => self.parse_variable_declaration(),
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
                Token::KeywordAdd => self.parse_add_operation(),
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
                Token::KeywordOpen => {
                    let mut tokens_clone = self.tokens.clone();
                    let mut has_read_pattern = false;

                    tokens_clone.next();

                    if let Some(token) = tokens_clone.next()
                        && token.token == Token::KeywordFile
                        && let Some(token) = tokens_clone.next()
                        && token.token == Token::KeywordAt
                        && let Some(token) = tokens_clone.next()
                        && matches!(token.token, Token::StringLiteral(_) | Token::Identifier(_))
                        && let Some(token) = tokens_clone.next()
                        && token.token == Token::KeywordAnd
                        && let Some(token) = tokens_clone.next()
                        && token.token == Token::KeywordRead
                        && let Some(token) = tokens_clone.next()
                        && token.token == Token::KeywordContent
                        && let Some(token) = tokens_clone.next()
                        && token.token == Token::KeywordAs
                        && let Some(token) = tokens_clone.next()
                        && matches!(token.token, Token::Identifier(_) | Token::KeywordContent)
                    {
                        has_read_pattern = true;
                    }

                    if has_read_pattern {
                        self.parse_open_file_read_statement()
                    } else {
                        self.parse_open_file_statement()
                    }
                }
                Token::KeywordClose => self.parse_close_file_statement(),
                Token::KeywordDelete => self.parse_delete_statement(),
                Token::KeywordWrite => self.parse_write_to_statement(),
                Token::KeywordWait => self.parse_wait_for_statement(),
                Token::KeywordGive | Token::KeywordReturn => self.parse_return_statement(),
                Token::Identifier(id) if id == "main" => {
                    let mut tokens_clone = self.tokens.clone();
                    tokens_clone.next();
                    if let Some(next_token) = tokens_clone.peek() {
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
            Err(ParseError::new("Unexpected end of input".to_string(), 0, 0))
        }
    }

    pub(crate) fn parse_variable_declaration(&mut self) -> Result<Statement, ParseError> {
        let first = self.tokens.next().unwrap();
        let mut is_constant = false;

        if matches!(first.token, Token::KeywordCreate)
            && let Some(peek) = self.tokens.peek()
            && matches!(peek.token, Token::KeywordConstant)
        {
            self.tokens.next();
            is_constant = true;
        }

        let name = self.parse_variable_name_list()?;
        self.expect_token(Token::KeywordAs, "Expected 'as' after variable name")?;
        let value = self.parse_expression()?;

        Ok(Statement::VariableDeclaration {
            name,
            value,
            is_constant,
            line: first.line,
            column: first.column,
        })
    }

    pub(crate) fn parse_variable_name_list(&mut self) -> Result<String, ParseError> {
        let mut parts = Vec::with_capacity(3);

        if let Some(token) = self.tokens.peek().cloned() {
            match &token.token {
                Token::Identifier(id) => {
                    self.tokens.next();
                    parts.push(id.clone());
                }
                Token::IntLiteral(_) => {
                    return Err(ParseError::new(
                        "Cannot use a number as a variable name".to_string(),
                        token.line,
                        token.column,
                    ));
                }
                _ => {
                    return Err(ParseError::new(
                        "Cannot use keyword as a variable name".to_string(),
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
                    parts.push(id.clone());
                }
                Token::KeywordAs => break,
                _ => break,
            }
        }

        Ok(parts.join(" "))
    }

    pub(crate) fn parse_create_list_statement(&mut self) -> Result<Statement, ParseError> {
        let create_token = self.tokens.next().unwrap();
        self.expect_token(Token::KeywordList, "Expected 'list' after 'create'")?;

        let name = if let Some(token) = self.tokens.peek() {
            match &token.token {
                Token::Identifier(n) => {
                    let name = n.clone();
                    self.tokens.next();
                    name
                }
                _ => {
                    return Err(ParseError::new(
                        format!("Expected identifier for list name, found {:?}", token.token),
                        token.line,
                        token.column,
                    ));
                }
            }
        } else {
            return Err(ParseError::new(
                "Expected list name after 'create list'".to_string(),
                create_token.line,
                create_token.column,
            ));
        };

        self.expect_token(Token::Colon, "Expected ':' after list name")?;

        let mut initial_values = Vec::new();

        while let Some(token) = self.tokens.peek().cloned() {
            match token.token {
                Token::KeywordEnd => {
                    self.tokens.next();
                    self.expect_token(Token::KeywordList, "Expected 'list' after 'end'")?;
                    break;
                }
                Token::KeywordAdd => {
                    self.tokens.next();
                    let value = self.parse_expression()?;
                    initial_values.push(value);
                }
                _ => {
                    return Err(ParseError::new(
                        format!(
                            "Expected 'add' or 'end list' in list creation, found {:?}",
                            token.token
                        ),
                        token.line,
                        token.column,
                    ));
                }
            }
        }

        Ok(Statement::CreateListStatement {
            name,
            initial_values,
            line: create_token.line,
            column: create_token.column,
        })
    }

    pub(crate) fn parse_create_date_statement(&mut self) -> Result<Statement, ParseError> {
        let create_token = self.tokens.next().unwrap();
        self.expect_token(Token::KeywordDate, "Expected 'date' after 'create'")?;

        let name = self.parse_variable_name_simple()?;

        let value = if let Some(token) = self.tokens.peek() {
            if token.token == Token::KeywordAs {
                self.tokens.next();
                Some(self.parse_expression()?)
            } else {
                None
            }
        } else {
            None
        };

        Ok(Statement::CreateDateStatement {
            name,
            value,
            line: create_token.line,
            column: create_token.column,
        })
    }

    pub(crate) fn parse_create_time_statement(&mut self) -> Result<Statement, ParseError> {
        let create_token = self.tokens.next().unwrap();
        self.expect_token(Token::KeywordTime, "Expected 'time' after 'create'")?;

        let name = self.parse_variable_name_simple()?;

        let value = if let Some(token) = self.tokens.peek() {
            if token.token == Token::KeywordAs {
                self.tokens.next();
                Some(self.parse_expression()?)
            } else {
                None
            }
        } else {
            None
        };

        Ok(Statement::CreateTimeStatement {
            name,
            value,
            line: create_token.line,
            column: create_token.column,
        })
    }

    pub(crate) fn parse_map_creation(&mut self) -> Result<Statement, ParseError> {
        let create_token = self.tokens.next().unwrap();
        self.expect_token(Token::KeywordMap, "Expected 'map' after 'create'")?;

        let name = if let Some(token) = self.tokens.peek() {
            match &token.token {
                Token::Identifier(n) => {
                    let name = n.clone();
                    self.tokens.next();
                    name
                }
                _ => {
                    return Err(ParseError::new(
                        format!("Expected identifier for map name, found {:?}", token.token),
                        token.line,
                        token.column,
                    ));
                }
            }
        } else {
            return Err(ParseError::new(
                "Expected map name after 'create map'".to_string(),
                create_token.line,
                create_token.column,
            ));
        };

        self.expect_token(Token::Colon, "Expected ':' after map name")?;

        let mut entries = Vec::new();

        while let Some(token) = self.tokens.peek().cloned() {
            match &token.token {
                Token::KeywordEnd => {
                    self.tokens.next();
                    self.expect_token(Token::KeywordMap, "Expected 'map' after 'end'")?;
                    break;
                }
                Token::Identifier(key) => {
                    let key = key.clone();
                    self.tokens.next();
                    self.expect_token(Token::KeywordIs, "Expected 'is' after map key")?;
                    let value = self.parse_expression()?;
                    entries.push((key, value));
                }
                _ => {
                    return Err(ParseError::new(
                        format!(
                            "Expected map key (identifier) or 'end map', found {:?}",
                            token.token
                        ),
                        token.line,
                        token.column,
                    ));
                }
            }
        }

        Ok(Statement::MapCreation {
            name,
            entries,
            line: create_token.line,
            column: create_token.column,
        })
    }

    pub(crate) fn parse_add_operation(&mut self) -> Result<Statement, ParseError> {
        let _saved_position = self.tokens.clone();
        let add_token = self.tokens.next().unwrap();

        let value = self.parse_expression()?;

        if let Some(token) = self.tokens.peek() {
            if token.token == Token::KeywordTo {
                self.tokens.next();

                let target_name = self.parse_variable_name_simple()?;

                match &value {
                    Expression::Literal(Literal::Integer(_), _, _)
                    | Expression::Literal(Literal::Float(_), _, _) => {
                        let operator = Operator::Plus;
                        Ok(Statement::Assignment {
                            name: target_name.clone(),
                            value: Expression::BinaryOperation {
                                left: Box::new(Expression::Variable(
                                    target_name,
                                    add_token.line,
                                    add_token.column,
                                )),
                                operator,
                                right: Box::new(value),
                                line: add_token.line,
                                column: add_token.column,
                            },
                            line: add_token.line,
                            column: add_token.column,
                        })
                    }
                    _ => Ok(Statement::AddToListStatement {
                        value,
                        list_name: target_name,
                        line: add_token.line,
                        column: add_token.column,
                    }),
                }
            } else {
                Err(ParseError::new(
                    "Expected 'to' after value in add statement".to_string(),
                    add_token.line,
                    add_token.column,
                ))
            }
        } else {
            Err(ParseError::new(
                "Unexpected end of input after add value".to_string(),
                add_token.line,
                add_token.column,
            ))
        }
    }

    pub(crate) fn parse_remove_from_list_statement(&mut self) -> Result<Statement, ParseError> {
        let remove_token = self.tokens.next().unwrap();

        let value = self.parse_expression()?;
        self.expect_token(Token::KeywordFrom, "Expected 'from' after value in remove")?;
        let list_name = self.parse_variable_name_simple()?;

        Ok(Statement::RemoveFromListStatement {
            value,
            list_name,
            line: remove_token.line,
            column: remove_token.column,
        })
    }

    pub(crate) fn parse_clear_list_statement(&mut self) -> Result<Statement, ParseError> {
        let clear_token = self.tokens.next().unwrap();
        let list_name = self.parse_variable_name_simple()?;

        if let Some(token) = self.tokens.peek()
            && token.token == Token::KeywordList
        {
            self.tokens.next();
        }

        Ok(Statement::ClearListStatement {
            list_name,
            line: clear_token.line,
            column: clear_token.column,
        })
    }
}
impl<'a> Parser<'a> {
    pub(crate) fn parse_display_statement(&mut self) -> Result<Statement, ParseError> {
        self.tokens.next();
        let expr = self.parse_expression()?;
        let token_pos = if let Some(token) = self.tokens.peek() {
            token
        } else {
            return match expr {
                Expression::Literal(_, line, column) => Ok(Statement::DisplayStatement {
                    value: expr,
                    line,
                    column,
                }),
                Expression::Variable(_, line, column) => Ok(Statement::DisplayStatement {
                    value: expr,
                    line,
                    column,
                }),
                Expression::BinaryOperation { line, column, .. } => {
                    Ok(Statement::DisplayStatement {
                        value: expr,
                        line,
                        column,
                    })
                }
                Expression::UnaryOperation { line, column, .. } => {
                    Ok(Statement::DisplayStatement {
                        value: expr,
                        line,
                        column,
                    })
                }
                Expression::FunctionCall { line, column, .. } => Ok(Statement::DisplayStatement {
                    value: expr,
                    line,
                    column,
                }),
                Expression::MemberAccess { line, column, .. } => Ok(Statement::DisplayStatement {
                    value: expr,
                    line,
                    column,
                }),
                Expression::IndexAccess { line, column, .. } => Ok(Statement::DisplayStatement {
                    value: expr,
                    line,
                    column,
                }),
                Expression::Concatenation { line, column, .. } => Ok(Statement::DisplayStatement {
                    value: expr,
                    line,
                    column,
                }),
                Expression::PatternMatch { line, column, .. } => Ok(Statement::DisplayStatement {
                    value: expr,
                    line,
                    column,
                }),
                Expression::PatternFind { line, column, .. } => Ok(Statement::DisplayStatement {
                    value: expr,
                    line,
                    column,
                }),
                Expression::PatternReplace { line, column, .. } => {
                    Ok(Statement::DisplayStatement {
                        value: expr,
                        line,
                        column,
                    })
                }
                Expression::PatternSplit { line, column, .. } => Ok(Statement::DisplayStatement {
                    value: expr,
                    line,
                    column,
                }),
                Expression::AwaitExpression { line, column, .. } => {
                    Ok(Statement::DisplayStatement {
                        value: expr,
                        line,
                        column,
                    })
                }
                Expression::ActionCall { line, column, .. } => Ok(Statement::DisplayStatement {
                    value: expr,
                    line,
                    column,
                }),
                Expression::StaticMemberAccess { line, column, .. } => {
                    Ok(Statement::DisplayStatement {
                        value: expr,
                        line,
                        column,
                    })
                }
                Expression::MethodCall { line, column, .. } => Ok(Statement::DisplayStatement {
                    value: expr,
                    line,
                    column,
                }),
                Expression::PropertyAccess { line, column, .. } => {
                    Ok(Statement::DisplayStatement {
                        value: expr,
                        line,
                        column,
                    })
                }
                Expression::FileExists { line, column, .. } => Ok(Statement::DisplayStatement {
                    value: expr,
                    line,
                    column,
                }),
                Expression::DirectoryExists { line, column, .. } => {
                    Ok(Statement::DisplayStatement {
                        value: expr,
                        line,
                        column,
                    })
                }
                Expression::ListFiles { line, column, .. } => Ok(Statement::DisplayStatement {
                    value: expr,
                    line,
                    column,
                }),
                Expression::ReadContent { line, column, .. } => Ok(Statement::DisplayStatement {
                    value: expr,
                    line,
                    column,
                }),
                Expression::ListFilesRecursive { line, column, .. } => {
                    Ok(Statement::DisplayStatement {
                        value: expr,
                        line,
                        column,
                    })
                }
                Expression::ListFilesFiltered { line, column, .. } => {
                    Ok(Statement::DisplayStatement {
                        value: expr,
                        line,
                        column,
                    })
                }
            };
        };
        Ok(Statement::DisplayStatement {
            value: expr,
            line: token_pos.line,
            column: token_pos.column,
        })
    }

    pub(crate) fn parse_if_statement(&mut self) -> Result<Statement, ParseError> {
        let check_token = self.tokens.next().unwrap();
        self.expect_token(Token::KeywordIf, "Expected 'if' after 'check'")?;
        let condition = self.parse_expression()?;
        if let Some(token) = self.tokens.peek()
            && matches!(token.token, Token::Colon)
        {
            self.tokens.next();
        }
        let mut then_block = Vec::with_capacity(8);
        while let Some(token) = self.tokens.peek().cloned() {
            match &token.token {
                Token::KeywordOtherwise | Token::KeywordEnd => break,
                _ => match self.parse_statement() {
                    Ok(stmt) => then_block.push(stmt),
                    Err(e) => return Err(e),
                },
            }
        }
        let else_block = if let Some(token) = self.tokens.peek() {
            if matches!(token.token, Token::KeywordOtherwise) {
                self.tokens.next();
                if let Some(token) = self.tokens.peek()
                    && matches!(token.token, Token::Colon)
                {
                    self.tokens.next();
                }
                let mut else_stmts = Vec::with_capacity(8);
                while let Some(token) = self.tokens.peek().cloned() {
                    if matches!(token.token, Token::KeywordEnd) {
                        break;
                    }
                    match self.parse_statement() {
                        Ok(stmt) => else_stmts.push(stmt),
                        Err(e) => return Err(e),
                    }
                }
                Some(else_stmts)
            } else {
                None
            }
        } else {
            None
        };
        if let Some(&token) = self.tokens.peek() {
            if matches!(token.token, Token::KeywordEnd) {
                self.tokens.next();
                if let Some(&next_token) = self.tokens.peek() {
                    if matches!(next_token.token, Token::KeywordCheck) {
                        self.tokens.next();
                    } else {
                        return Err(ParseError::new(
                            format!("Expected 'check' after 'end', found {:?}", next_token.token),
                            next_token.line,
                            next_token.column,
                        ));
                    }
                } else {
                    return Err(ParseError::new(
                        "Expected 'check' after 'end', found end of input".to_string(),
                        token.line,
                        token.column,
                    ));
                }
            } else {
                return Err(ParseError::new(
                    format!("Expected 'end' after if block, found {:?}", token.token),
                    token.line,
                    token.column,
                ));
            }
        } else {
            return Err(ParseError::new(
                "Expected 'end' after if block, found end of input".to_string(),
                0,
                0,
            ));
        }
        Ok(Statement::IfStatement {
            condition,
            then_block,
            else_block,
            line: check_token.line,
            column: check_token.column,
        })
    }

    pub(crate) fn parse_single_line_if(&mut self) -> Result<Statement, ParseError> {
        let if_token = self.tokens.next().unwrap();
        let condition = self.parse_expression()?;
        self.expect_token(Token::KeywordThen, "Expected 'then' after if condition")?;
        let mut then_block = Vec::new();
        let mut is_multiline = false;
        while let Some(token) = self.tokens.peek() {
            match &token.token {
                Token::KeywordOtherwise | Token::KeywordEnd => {
                    is_multiline = true;
                    break;
                }
                Token::Newline => {
                    self.tokens.next();
                    is_multiline = true;
                    continue;
                }
                _ => {
                    let stmt = self.parse_statement()?;
                    then_block.push(stmt);
                    if let Some(next_token) = self.tokens.peek() {
                        if matches!(
                            next_token.token,
                            Token::KeywordOtherwise | Token::KeywordEnd
                        ) {
                            is_multiline = true;
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
        }
        let else_block = if let Some(token) = self.tokens.peek() {
            if matches!(token.token, Token::KeywordOtherwise) {
                self.tokens.next();
                let mut else_stmts = Vec::new();
                while let Some(token) = self.tokens.peek() {
                    match &token.token {
                        Token::KeywordEnd => break,
                        Token::Newline => {
                            self.tokens.next();
                            continue;
                        }
                        _ => {
                            let stmt = self.parse_statement()?;
                            else_stmts.push(stmt);
                        }
                    }
                }
                Some(else_stmts)
            } else {
                None
            }
        } else {
            None
        };
        if is_multiline {
            self.expect_token(Token::KeywordEnd, "Expected 'end' after if block")?;
            self.expect_token(Token::KeywordIf, "Expected 'if' after 'end'")?;
        }
        if is_multiline {
            Ok(Statement::IfStatement {
                condition,
                then_block,
                else_block,
                line: if_token.line,
                column: if_token.column,
            })
        } else {
            let then_stmt = if then_block.is_empty() {
                return Err(ParseError::new(
                    "Expected statement after 'then'".to_string(),
                    if_token.line,
                    if_token.column,
                ));
            } else {
                Box::new(then_block.into_iter().next().unwrap())
            };
            let else_stmt = else_block.and_then(|stmts| {
                if stmts.is_empty() {
                    None
                } else {
                    Some(Box::new(stmts.into_iter().next().unwrap()))
                }
            });
            Ok(Statement::SingleLineIf {
                condition,
                then_stmt,
                else_stmt,
                line: if_token.line,
                column: if_token.column,
            })
        }
    }

    pub(crate) fn parse_for_each_loop(&mut self) -> Result<Statement, ParseError> {
        self.tokens.next();
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
        let reversed = if let Some(token) = self.tokens.peek() {
            if matches!(token.token, Token::KeywordReversed) {
                self.tokens.next();
                true
            } else {
                false
            }
        } else {
            false
        };
        let collection = self.parse_expression()?;
        if let Some(token) = self.tokens.peek()
            && matches!(token.token, Token::Colon)
        {
            self.tokens.next();
        }
        let mut body = Vec::with_capacity(10);
        while let Some(token) = self.tokens.peek().cloned() {
            if matches!(token.token, Token::KeywordEnd) {
                let mut la = self.tokens.clone();
                let _ = la.next();
                if let Some(next_tok) = la.peek()
                    && matches!(next_tok.token, Token::KeywordFor)
                {
                    break;
                }
            }
            match self.parse_statement() {
                Ok(stmt) => body.push(stmt),
                Err(e) => return Err(e),
            }
        }
        self.expect_token(Token::KeywordEnd, "Expected 'end' after for-each loop body")?;
        self.expect_token(Token::KeywordFor, "Expected 'for' after 'end'")?;
        let token_pos = self.tokens.peek().map_or(
            &crate::lexer::token::TokenWithPosition {
                token: Token::KeywordFor,
                line: 0,
                column: 0,
                length: 0,
            },
            |v| v,
        );
        Ok(Statement::ForEachLoop {
            item_name,
            collection,
            reversed,
            body,
            line: token_pos.line,
            column: token_pos.column,
        })
    }

    pub(crate) fn parse_count_loop(&mut self) -> Result<Statement, ParseError> {
        self.tokens.next();
        exec_trace!(
            "After 'count', next token is: {:?}",
            self.tokens.peek().map(|t| &t.token)
        );
        self.expect_token(Token::KeywordFrom, "Expected 'from' after 'count'")?;
        let start = self.parse_expression()?;
        let downward = if let Some(token) = self.tokens.peek() {
            if let Token::Identifier(id) = &token.token {
                if id.to_lowercase() == "down" {
                    self.tokens.next();
                    self.expect_token(Token::KeywordTo, "Expected 'to' after 'down'")?;
                    true
                } else if matches!(token.token, Token::KeywordTo) {
                    self.tokens.next();
                    false
                } else {
                    return Err(ParseError::new(
                        format!("Expected 'to' or 'down to', found {:?}", token.token),
                        token.line,
                        token.column,
                    ));
                }
            } else if matches!(token.token, Token::KeywordTo) {
                self.tokens.next();
                false
            } else {
                return Err(ParseError::new(
                    format!("Expected 'to' or 'down to', found {:?}", token.token),
                    token.line,
                    token.column,
                ));
            }
        } else {
            return Err(ParseError::new(
                "Unexpected end of input after count from expression".to_string(),
                0,
                0,
            ));
        };
        let end = self.parse_expression()?;
        let step = if let Some(token) = self.tokens.peek() {
            if matches!(token.token, Token::KeywordBy) {
                self.tokens.next();
                Some(self.parse_expression()?)
            } else {
                None
            }
        } else {
            None
        };
        if let Some(token) = self.tokens.peek()
            && matches!(token.token, Token::Colon)
        {
            self.tokens.next();
        }
        let mut body = Vec::with_capacity(10);
        while let Some(token) = self.tokens.peek().cloned() {
            if matches!(token.token, Token::KeywordEnd) {
                let mut la = self.tokens.clone();
                let _ = la.next();
                if let Some(next_tok) = la.peek()
                    && matches!(next_tok.token, Token::KeywordCount)
                {
                    break;
                }
            }
            match self.parse_statement() {
                Ok(stmt) => body.push(stmt),
                Err(e) => return Err(e),
            }
        }
        self.expect_token(Token::KeywordEnd, "Expected 'end' after count loop body")?;
        self.expect_token(Token::KeywordCount, "Expected 'count' after 'end'")?;
        let token_pos = self.tokens.peek().map_or(
            &crate::lexer::token::TokenWithPosition {
                token: Token::KeywordCount,
                line: 0,
                column: 0,
                length: 0,
            },
            |v| v,
        );
        Ok(Statement::CountLoop {
            start,
            end,
            step,
            downward,
            body,
            line: token_pos.line,
            column: token_pos.column,
        })
    }

    pub(crate) fn parse_action_definition(&mut self) -> Result<Statement, ParseError> {
        exec_trace!("Parsing action definition");
        self.tokens.next();
        exec_trace!("Expecting 'action' after 'define'");
        self.expect_token(Token::KeywordAction, "Expected 'action' after 'define'")?;
        exec_trace!("Expecting 'called' after 'action'");
        self.expect_token(Token::KeywordCalled, "Expected 'called' after 'action'")?;
        exec_trace!("Expecting identifier after 'called'");
        let name = if let Some(token) = self.tokens.peek() {
            if let Token::Identifier(id) = &token.token {
                exec_trace!("Found action name: {}", id);
                self.tokens.next();
                id.clone()
            } else {
                exec_trace!(
                    "Expected identifier after 'called', found {:?}",
                    token.token
                );
                return Err(ParseError::new(
                    format!(
                        "Expected identifier after 'called', found {:?}",
                        token.token
                    ),
                    token.line,
                    token.column,
                ));
            }
        } else {
            exec_trace!("Unexpected end of input after 'called'");
            return Err(ParseError::new(
                "Unexpected end of input after 'called'".to_string(),
                0,
                0,
            ));
        };
        exec_trace!("Action name parsed: {}", name);
        let mut parameters = Vec::with_capacity(4);
        if let Some(token) = self.tokens.peek().cloned()
            && (matches!(token.token, Token::KeywordNeeds)
                || matches!(token.token, Token::KeywordWith))
        {
            let _keyword = if matches!(token.token, Token::KeywordNeeds) {
                "needs"
            } else {
                "with"
            };
            exec_trace!("Found '{}' keyword, parsing parameters", _keyword);
            self.tokens.next();
            while let Some(token) = self.tokens.peek().cloned() {
                exec_trace!("Checking token for parameter: {:?}", token.token);
                let (param_name, param_line, param_column) =
                    if let Token::Identifier(id) = &token.token {
                        exec_trace!("Found parameter: {}", id);
                        let line = token.line;
                        let column = token.column;
                        self.tokens.next();
                        (id.clone(), line, column)
                    } else {
                        exec_trace!("Not an identifier, breaking parameter parsing");
                        break;
                    };
                let param_type = if let Some(token) = self.tokens.peek() {
                    if matches!(token.token, Token::KeywordOf) {
                        self.tokens.next();
                        if let Some(token) = self.tokens.peek() {
                            match &token.token {
                                Token::KeywordText => {
                                    self.tokens.next();
                                    Some(Type::Text)
                                }
                                Token::Identifier(typ) if typ == "number" => {
                                    self.tokens.next();
                                    Some(Type::Number)
                                }
                                Token::Identifier(typ) if typ == "boolean" => {
                                    self.tokens.next();
                                    Some(Type::Boolean)
                                }
                                Token::Identifier(typ) => {
                                    let typ = typ.clone();
                                    self.tokens.next();
                                    Some(Type::Custom(typ))
                                }
                                _ => None,
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };
                let default_value = if let Some(token) = self.tokens.peek() {
                    if matches!(token.token, Token::KeywordIs) {
                        self.tokens.next();
                        Some(self.parse_expression()?)
                    } else {
                        None
                    }
                } else {
                    None
                };
                parameters.push(Parameter {
                    name: param_name,
                    param_type,
                    default_value,
                    line: param_line,
                    column: param_column,
                });
                if let Some(token) = self.tokens.peek()
                    && matches!(token.token, Token::KeywordAnd)
                {
                    self.tokens.next();
                    continue;
                }
                break;
            }
        }
        let return_type = if let Some(token) = self.tokens.peek() {
            if matches!(token.token, Token::KeywordReturn) {
                self.tokens.next();
                if let Some(token) = self.tokens.peek() {
                    match &token.token {
                        Token::KeywordText => {
                            self.tokens.next();
                            Some(Type::Text)
                        }
                        Token::Identifier(typ) if typ == "number" => {
                            self.tokens.next();
                            Some(Type::Number)
                        }
                        Token::Identifier(typ) if typ == "boolean" => {
                            self.tokens.next();
                            Some(Type::Boolean)
                        }
                        Token::Identifier(typ) => {
                            let typ = typ.clone();
                            self.tokens.next();
                            Some(Type::Custom(typ))
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        self.expect_token(Token::Colon, "Expected ':' after action declaration")?;
        let mut body = Vec::with_capacity(16);
        loop {
            if let Some(token) = self.tokens.peek() {
                exec_trace!("Action body loop at token: {:?}", token.token);
                if token.token == Token::KeywordEnd {
                    let mut la = self.tokens.clone();
                    let _ = la.next();
                    if let Some(next_tok) = la.peek()
                        && matches!(next_tok.token, Token::KeywordAction)
                    {
                        exec_trace!("Found 'end action', breaking action body");
                        self.tokens.next();
                        self.expect_token(Token::KeywordAction, "Expected 'action' after 'end'")?;
                        break;
                    } else if let Some(next_tok) = la.peek() {
                        match next_tok.token {
                            Token::KeywordCheck
                            | Token::KeywordFor
                            | Token::KeywordCount
                            | Token::KeywordRepeat
                            | Token::KeywordTry
                            | Token::KeywordLoop
                            | Token::KeywordMap
                            | Token::KeywordWhile
                            | Token::KeywordPattern => {
                                exec_trace!(
                                    "Skipping stray 'end {:?}' inside action body",
                                    next_tok.token
                                );
                                self.tokens.next();
                                self.tokens.next();
                                continue;
                            }
                            _ => {}
                        }
                    }
                }
                body.push(self.parse_statement()?);
            } else {
                return Err(ParseError::new(
                    "Unexpected end of input in action body".to_string(),
                    0,
                    0,
                ));
            }
        }
        let token_pos = self.tokens.peek().map_or(
            &crate::lexer::token::TokenWithPosition {
                token: Token::KeywordDefine,
                line: 0,
                column: 0,
                length: 0,
            },
            |v| v,
        );
        self.known_actions.insert(name.clone());
        Ok(Statement::ActionDefinition {
            name,
            parameters,
            body,
            return_type,
            line: token_pos.line,
            column: token_pos.column,
        })
    }

    pub(crate) fn parse_assignment(&mut self) -> Result<Statement, ParseError> {
        self.tokens.next();
        let name = self.parse_variable_name_simple()?;
        self.expect_token(Token::KeywordTo, "Expected 'to' after identifier(s)")?;
        let value = self.parse_expression()?;
        let token_pos = self.tokens.peek().map_or(
            &crate::lexer::token::TokenWithPosition {
                token: Token::KeywordChange,
                line: 0,
                column: 0,
                length: 0,
            },
            |v| v,
        );
        Ok(Statement::Assignment {
            name,
            value,
            line: token_pos.line,
            column: token_pos.column,
        })
    }

    pub(crate) fn parse_arithmetic_operation(&mut self) -> Result<Statement, ParseError> {
        let operator = match self.tokens.peek() {
            Some(t) => match t.token {
                Token::KeywordAdd => Operator::Plus,
                Token::KeywordSubtract => Operator::Minus,
                Token::KeywordMultiply => Operator::Multiply,
                Token::KeywordDivide => Operator::Divide,
                _ => Operator::Plus,
            },
            None => Operator::Plus,
        };
        self.tokens.next();
        let value = self.parse_expression()?;
        self.expect_token(Token::KeywordTo, "Expected 'to' after value")?;
        let name = self.parse_variable_name_simple()?;
        let binary_expr = Expression::BinaryOperation {
            left: Box::new(Expression::Variable(name.clone(), 0, 0)),
            operator,
            right: Box::new(value),
            line: 0,
            column: 0,
        };
        Ok(Statement::Assignment {
            name,
            value: binary_expr,
            line: 0,
            column: 0,
        })
    }

    pub(crate) fn parse_variable_name_simple(&mut self) -> Result<String, ParseError> {
        if let Some(token) = self.tokens.peek() {
            if let Token::Identifier(id) = &token.token {
                let id = id.clone();
                self.tokens.next();
                Ok(id)
            } else {
                Err(ParseError::new(
                    format!("Expected identifier, found {:?}", token.token),
                    token.line,
                    token.column,
                ))
            }
        } else {
            Err(ParseError::new(
                "Unexpected end of input when expecting identifier".to_string(),
                0,
                0,
            ))
        }
    }

    pub(crate) fn parse_return_statement(&mut self) -> Result<Statement, ParseError> {
        let token_pos = self.tokens.next().unwrap();
        let value = if let Some(token) = self.tokens.peek() {
            if matches!(token.token, Token::KeywordBack) {
                self.tokens.next();
                // After "back", parse the expression if there is one
                if let Some(next_token) = self.tokens.peek() {
                    if !matches!(next_token.token, Token::KeywordEnd) {
                        Some(self.parse_expression()?)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                Some(self.parse_expression()?)
            }
        } else {
            None
        };
        Ok(Statement::ReturnStatement {
            value,
            line: token_pos.line,
            column: token_pos.column,
        })
    }

    pub(crate) fn parse_open_file_statement(&mut self) -> Result<Statement, ParseError> {
        let open_tok = self.tokens.next().unwrap();
        self.expect_token(Token::KeywordFile, "Expected 'file' after 'open'")?;
        self.expect_token(Token::KeywordAt, "Expected 'at' after 'file'")?;
        let path_expr = self.parse_primary_expression()?;
        self.expect_token(Token::KeywordAs, "Expected 'as' after file path")?;
        let variable_name = self.parse_variable_name_simple()?;
        Ok(Statement::OpenFileStatement {
            path: path_expr,
            variable_name,
            mode: FileOpenMode::Read,
            line: open_tok.line,
            column: open_tok.column,
        })
    }

    pub(crate) fn parse_open_file_read_statement(&mut self) -> Result<Statement, ParseError> {
        let open_tok = self.tokens.next().unwrap();
        self.expect_token(Token::KeywordFile, "Expected 'file' after 'open'")?;
        self.expect_token(Token::KeywordAt, "Expected 'at' after 'file'")?;
        let path_expr = self.parse_primary_expression()?;
        self.expect_token(Token::KeywordAnd, "Expected 'and' after file path")?;
        self.expect_token(Token::KeywordRead, "Expected 'read' after 'and'")?;
        self.expect_token(Token::KeywordContent, "Expected 'content' after 'read'")?;
        self.expect_token(Token::KeywordAs, "Expected 'as' after 'content'")?;
        let variable_name = if let Some(tok) = self.tokens.peek() {
            match tok.token {
                Token::KeywordContent => {
                    self.tokens.next();
                    "content".to_string()
                }
                _ => self.parse_variable_name_simple()?,
            }
        } else {
            return Err(ParseError::new(
                "Unexpected end of input when expecting variable name".to_string(),
                0,
                0,
            ));
        };
        Ok(Statement::ReadFileStatement {
            path: path_expr,
            variable_name,
            line: open_tok.line,
            column: open_tok.column,
        })
    }

    pub(crate) fn parse_wait_for_statement(&mut self) -> Result<Statement, ParseError> {
        let wait_tok = self.tokens.next().unwrap();
        self.expect_token(Token::KeywordFor, "Expected 'for' after 'wait'")?;
        let stmt = if let Some(token) = self.tokens.peek() {
            match token.token {
                Token::KeywordOpen => {
                    let mut tokens_clone = self.tokens.clone();
                    let mut has_read_pattern = false;

                    if let Some(token) = tokens_clone.next()
                        && token.token == Token::KeywordOpen
                        && let Some(token) = tokens_clone.next()
                        && token.token == Token::KeywordFile
                        && let Some(token) = tokens_clone.next()
                        && token.token == Token::KeywordAt
                        && let Some(token) = tokens_clone.next()
                        && matches!(token.token, Token::StringLiteral(_) | Token::Identifier(_))
                        && let Some(token) = tokens_clone.next()
                        && token.token == Token::KeywordAnd
                        && let Some(token) = tokens_clone.next()
                        && token.token == Token::KeywordRead
                        && let Some(token) = tokens_clone.next()
                        && token.token == Token::KeywordContent
                        && let Some(token) = tokens_clone.next()
                        && token.token == Token::KeywordAs
                        && let Some(token) = tokens_clone.next()
                        && matches!(token.token, Token::Identifier(_) | Token::KeywordContent)
                    {
                        has_read_pattern = true;
                    }

                    if has_read_pattern {
                        self.parse_open_file_read_statement()?
                    } else {
                        self.parse_open_file_statement()?
                    }
                }
                Token::KeywordClose => {
                    self.tokens.next();
                    self.parse_close_file_statement()?
                }
                Token::KeywordWrite => {
                    self.tokens.next();
                    self.expect_token(Token::KeywordContent, "Expected 'content' after 'write'")?;
                    let content = self.parse_expression()?;
                    self.expect_token(Token::KeywordInto, "Expected 'into' after content")?;
                    let file = self.parse_expression()?;
                    Statement::WriteFileStatement {
                        file,
                        content,
                        mode: WriteMode::Overwrite,
                        line: wait_tok.line,
                        column: wait_tok.column,
                    }
                }
                Token::KeywordAppend => {
                    self.tokens.next();
                    self.expect_token(Token::KeywordContent, "Expected 'content' after 'append'")?;
                    let content = self.parse_expression()?;
                    self.expect_token(Token::KeywordInto, "Expected 'into' after content")?;
                    let file = self.parse_expression()?;
                    Statement::WriteFileStatement {
                        file,
                        content,
                        mode: WriteMode::Append,
                        line: wait_tok.line,
                        column: wait_tok.column,
                    }
                }
                _ => {
                    return Err(ParseError::new(
                        "Expected 'open', 'close', 'write', or 'append' after 'wait for'"
                            .to_string(),
                        wait_tok.line,
                        wait_tok.column,
                    ));
                }
            }
        } else {
            return Err(ParseError::new(
                "Expected 'open', 'close', 'write', or 'append' after 'wait for'".to_string(),
                wait_tok.line,
                wait_tok.column,
            ));
        };
        Ok(Statement::WaitForStatement {
            inner: Box::new(stmt),
            line: wait_tok.line,
            column: wait_tok.column,
        })
    }

    pub(crate) fn parse_expression_statement(&mut self) -> Result<Statement, ParseError> {
        let binding = crate::lexer::token::TokenWithPosition {
            token: Token::Identifier(String::new()),
            line: 0,
            column: 0,
            length: 0,
        };
        let default_token = self.tokens.peek().map_or(&binding, |v| v);
        let expr = self.parse_expression()?;
        Ok(Statement::ExpressionStatement {
            expression: expr,
            line: default_token.line,
            column: default_token.column,
        })
    }

    pub(crate) fn parse_close_file_statement(&mut self) -> Result<Statement, ParseError> {
        let close_tok = self.tokens.next().unwrap();
        self.expect_token(Token::KeywordFile, "Expected 'file' after 'close'")?;
        let variable_name = self.parse_variable_name_simple()?;
        Ok(Statement::CloseFileStatement {
            file: Expression::Variable(variable_name, close_tok.line, close_tok.column),
            line: close_tok.line,
            column: close_tok.column,
        })
    }

    pub(crate) fn parse_create_directory_statement(&mut self) -> Result<Statement, ParseError> {
        self.tokens.next();
        self.expect_token(
            Token::KeywordDirectory,
            "Expected 'directory' after 'create'",
        )?;
        let expr = self.parse_expression()?;
        Ok(Statement::CreateDirectoryStatement {
            path: expr,
            line: 0,
            column: 0,
        })
    }

    pub(crate) fn parse_create_file_statement(&mut self) -> Result<Statement, ParseError> {
        let create_tok = self.tokens.next().unwrap();
        self.expect_token(Token::KeywordFile, "Expected 'file' after 'create'")?;
        let path = self.parse_expression()?;
        self.expect_token(Token::KeywordContent, "Expected 'content' after file path")?;
        let content = self.parse_expression()?;
        Ok(Statement::CreateFileStatement {
            path,
            content,
            line: create_tok.line,
            column: create_tok.column,
        })
    }

    pub(crate) fn parse_write_to_statement(&mut self) -> Result<Statement, ParseError> {
        let write_tok = self.tokens.next().unwrap();
        let content = self.parse_expression()?;
        self.expect_token(Token::KeywordTo, "Expected 'to' after write value")?;
        let file = self.parse_expression()?;
        Ok(Statement::WriteToStatement {
            content,
            file,
            line: write_tok.line,
            column: write_tok.column,
        })
    }

    pub(crate) fn parse_delete_statement(&mut self) -> Result<Statement, ParseError> {
        let delete_tok = self.tokens.next().unwrap();
        if let Some(tok) = self.tokens.peek() {
            match tok.token {
                Token::KeywordFile => {
                    self.tokens.next();
                    let path = self.parse_expression()?;
                    return Ok(Statement::DeleteFileStatement {
                        path,
                        line: delete_tok.line,
                        column: delete_tok.column,
                    });
                }
                Token::KeywordDirectory => {
                    self.tokens.next();
                    let path = self.parse_expression()?;
                    return Ok(Statement::DeleteDirectoryStatement {
                        path,
                        line: delete_tok.line,
                        column: delete_tok.column,
                    });
                }
                _ => {}
            }
        }
        Err(ParseError::new(
            "Expected 'file' or 'directory' after 'delete'".to_string(),
            delete_tok.line,
            delete_tok.column,
        ))
    }

    pub(crate) fn parse_try_statement(&mut self) -> Result<Statement, ParseError> {
        let try_tok = self.tokens.next().unwrap();
        self.expect_token(Token::Colon, "Expected ':' after 'try'")?;
        let mut body = Vec::new();
        while let Some(token) = self.tokens.peek() {
            if matches!(token.token, Token::KeywordEnd) {
                break;
            }
            body.push(self.parse_statement()?);
        }
        self.expect_token(Token::KeywordEnd, "Expected 'end' after try block")?;
        self.expect_token(Token::KeywordTry, "Expected 'try' after 'end'")?;
        Ok(Statement::TryStatement {
            body,
            when_clauses: Vec::new(),
            otherwise_block: None,
            line: try_tok.line,
            column: try_tok.column,
        })
    }

    pub(crate) fn parse_main_loop(&mut self) -> Result<Statement, ParseError> {
        let main_token = self.tokens.next().unwrap();
        self.expect_token(Token::KeywordLoop, "Expected 'loop' after 'main'")?;
        self.expect_token(Token::Colon, "Expected ':' after 'loop'")?;
        let mut body = Vec::new();
        while let Some(token) = self.tokens.peek().cloned() {
            if matches!(token.token, Token::KeywordEnd) {
                let mut la = self.tokens.clone();
                let _ = la.next();
                if let Some(next_tok) = la.peek()
                    && matches!(next_tok.token, Token::KeywordLoop)
                {
                    break;
                }
            }
            body.push(self.parse_statement()?);
        }
        self.expect_token(Token::KeywordEnd, "Expected 'end' after main loop body")?;
        self.expect_token(Token::KeywordLoop, "Expected 'loop' after 'end'")?;
        Ok(Statement::MainLoop {
            body,
            line: main_token.line,
            column: main_token.column,
        })
    }

    pub(crate) fn parse_repeat_statement(&mut self) -> Result<Statement, ParseError> {
        let repeat_token = self.tokens.next().unwrap();
        if let Some(token) = self.tokens.peek().cloned() {
            match token.token {
                Token::KeywordWhile => {
                    self.tokens.next();
                    let condition = self.parse_expression()?;
                    if let Some(token) = self.tokens.peek()
                        && matches!(token.token, Token::Colon)
                    {
                        self.tokens.next();
                    }
                    let mut body = Vec::new();
                    while let Some(token) = self.tokens.peek().cloned() {
                        if matches!(token.token, Token::KeywordEnd) {
                            let mut la = self.tokens.clone();
                            let _ = la.next();
                            if let Some(next_tok) = la.peek()
                                && matches!(next_tok.token, Token::KeywordRepeat)
                            {
                                break;
                            }
                        }
                        body.push(self.parse_statement()?);
                    }
                    self.expect_token(Token::KeywordEnd, "Expected 'end' after repeat while body")?;
                    self.expect_token(Token::KeywordRepeat, "Expected 'repeat' after 'end'")?;
                    Ok(Statement::RepeatWhileLoop {
                        condition,
                        body,
                        line: repeat_token.line,
                        column: repeat_token.column,
                    })
                }
                Token::KeywordUntil => {
                    self.tokens.next();
                    let condition = self.parse_expression()?;
                    if let Some(token) = self.tokens.peek()
                        && matches!(token.token, Token::Colon)
                    {
                        self.tokens.next();
                    }
                    let mut body = Vec::new();
                    while let Some(token) = self.tokens.peek().cloned() {
                        if matches!(token.token, Token::KeywordEnd) {
                            let mut la = self.tokens.clone();
                            let _ = la.next();
                            if let Some(next_tok) = la.peek()
                                && matches!(next_tok.token, Token::KeywordRepeat)
                            {
                                break;
                            }
                        }
                        body.push(self.parse_statement()?);
                    }
                    self.expect_token(Token::KeywordEnd, "Expected 'end' after repeat until body")?;
                    self.expect_token(Token::KeywordRepeat, "Expected 'repeat' after 'end'")?;
                    Ok(Statement::RepeatUntilLoop {
                        condition,
                        body,
                        line: repeat_token.line,
                        column: repeat_token.column,
                    })
                }
                Token::KeywordForever => {
                    self.tokens.next();
                    self.expect_token(Token::Colon, "Expected ':' after 'forever'")?;
                    let mut body = Vec::new();
                    while let Some(token) = self.tokens.peek().cloned() {
                        if matches!(token.token, Token::KeywordEnd) {
                            let mut la = self.tokens.clone();
                            let _ = la.next();
                            if let Some(next_tok) = la.peek()
                                && matches!(next_tok.token, Token::KeywordRepeat)
                            {
                                break;
                            }
                        }
                        body.push(self.parse_statement()?);
                    }
                    self.expect_token(Token::KeywordEnd, "Expected 'end' after forever body")?;
                    self.expect_token(Token::KeywordRepeat, "Expected 'repeat' after 'end'")?;
                    Ok(Statement::ForeverLoop {
                        body,
                        line: repeat_token.line,
                        column: repeat_token.column,
                    })
                }
                Token::Colon => {
                    self.tokens.next();
                    let mut body = Vec::new();
                    while let Some(token) = self.tokens.peek().cloned() {
                        if matches!(token.token, Token::KeywordUntil) {
                            break;
                        }
                        body.push(self.parse_statement()?);
                    }
                    self.expect_token(Token::KeywordUntil, "Expected 'until' after repeat body")?;
                    let condition = self.parse_expression()?;
                    Ok(Statement::RepeatUntilLoop {
                        condition,
                        body,
                        line: repeat_token.line,
                        column: repeat_token.column,
                    })
                }
                _ => Err(ParseError::new(
                    format!(
                        "Expected 'while', 'until', 'forever', or ':' after 'repeat', found {:?}",
                        token.token
                    ),
                    token.line,
                    token.column,
                )),
            }
        } else {
            Err(ParseError::new(
                "Unexpected end of input after 'repeat'".to_string(),
                repeat_token.line,
                repeat_token.column,
            ))
        }
    }

    pub(crate) fn parse_exit_statement(&mut self) -> Result<Statement, ParseError> {
        let exit_token = self.tokens.next().unwrap();
        if let Some(token) = self.tokens.peek().cloned()
            && let Token::Identifier(id) = &token.token
            && id.to_lowercase() == "loop"
        {
            self.tokens.next();
        }
        Ok(Statement::ExitStatement {
            line: exit_token.line,
            column: exit_token.column,
        })
    }

    pub(crate) fn parse_push_statement(&mut self) -> Result<Statement, ParseError> {
        let push_token = self.tokens.next().unwrap();
        self.expect_token(Token::KeywordWith, "Expected 'with' after 'push'")?;
        let list_expr = self.parse_primary_expression()?;
        self.expect_token(Token::KeywordAnd, "Expected 'and' after list expression")?;
        let start_line = if let Some(token) = self.tokens.peek() {
            token.line
        } else {
            push_token.line
        };
        let mut value_expr = self.parse_primary_expression()?;
        if let Some(token) = self.tokens.peek()
            && token.line == start_line
            && !super::Parser::is_statement_starter(&token.token)
        {
            value_expr = self.parse_binary_expression(0)?;
        }
        let stmt = Statement::PushStatement {
            list: list_expr,
            value: value_expr,
            line: push_token.line,
            column: push_token.column,
        };
        Ok(stmt)
    }
}
