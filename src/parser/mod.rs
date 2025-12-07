pub mod ast;
mod cursor;
mod helpers;
mod expr;
mod stmt;
#[cfg(test)]
mod tests;

use crate::exec_trace;
use crate::lexer::token::{Token, TokenWithPosition};
use ast::*;
use cursor::Cursor;
use helpers::is_reserved_pattern_name;
use expr::{ExprParser, PrimaryExprParser, BinaryExprParser};
use stmt::{StmtParser, VariableParser, CollectionParser, IoParser, ProcessParser, WebParser, ActionParser, ErrorHandlingParser, ControlFlowParser, PatternParser};

pub struct Parser<'a> {
    /// Cursor for efficient token navigation
    cursor: Cursor<'a>,
    /// Parse errors accumulated during parsing
    errors: Vec<ParseError>,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [TokenWithPosition]) -> Self {
        Parser {
            cursor: Cursor::new(tokens),
            errors: Vec::with_capacity(4),
        }
    }

    /// Consume token from cursor and advance position.
    ///
    /// This is a convenience wrapper around `cursor.bump()` that maintains
    /// consistent naming with the rest of the parser.
    #[inline]
    fn bump_sync(&mut self) -> Option<&'a TokenWithPosition> {
        self.cursor.bump()
    }

    pub fn parse(&mut self) -> Result<Program, Vec<ParseError>> {
        let mut program = Program::new();
        program.statements.reserve(self.cursor.remaining() / 5);

        while self.cursor.peek().is_some() {
            let start_pos = self.cursor.pos();

            // Skip any leading Eol tokens
            if let Some(token) = self.cursor.peek() {
                if matches!(token.token, Token::Eol) {
                    self.bump_sync();
                    continue;
                }
            }

            // Comprehensive handling of "end" tokens that might be left unconsumed
            // Check first two tokens without cloning
            if let Some(first_token) = self.cursor.peek() {
                if first_token.token == Token::KeywordEnd {
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
                                exec_trace!(
                                    "Found standalone 'end' at line {}",
                                    first_token.line
                                );
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
                                self.errors.push(ParseError::new(
                                    format!(
                                        "Unexpected 'end' followed by {:?}",
                                        second_token.token
                                    ),
                                    first_token.line,
                                    first_token.column,
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

    // Container-related parsing methods
    pub fn parse_container_definition(&mut self) -> Result<Statement, ParseError> {
        let start_token = self.bump_sync().unwrap(); // Consume 'create'
        let line = start_token.line;
        let column = start_token.column;

        self.expect_token(
            Token::KeywordContainer,
            "Expected 'container' after 'create'",
        )?;

        // Parse container name
        let name = if let Some(token) = self.cursor.peek() {
            if let Token::Identifier(id) = &token.token {
                self.bump_sync(); // Consume the identifier
                id.clone()
            } else {
                return Err(ParseError::new(
                    format!(
                        "Expected identifier for container name, found {:?}",
                        token.token
                    ),
                    token.line,
                    token.column,
                ));
            }
        } else {
            return Err(ParseError::new(
                "Expected identifier for container name, found end of input".to_string(),
                line,
                column,
            ));
        };

        // Parse inheritance and interfaces
        let (extends, implements) = self.parse_inheritance()?;

        // Expect colon after container declaration
        self.expect_token(Token::Colon, "Expected ':' after container name")?;

        // Parse container body
        let (properties, methods, events, static_properties, static_methods) =
            self.parse_container_body()?;

        Ok(Statement::ContainerDefinition {
            name,
            extends,
            implements,
            properties,
            methods,
            events,
            static_properties,
            static_methods,
            line,
            column,
        })
    }

    pub fn parse_interface_definition(&mut self) -> Result<Statement, ParseError> {
        let start_token = self.bump_sync().unwrap(); // Consume 'create'
        let line = start_token.line;
        let column = start_token.column;

        self.expect_token(
            Token::KeywordInterface,
            "Expected 'interface' after 'create'",
        )?;

        // Parse interface name
        let name = if let Some(token) = self.cursor.peek() {
            if let Token::Identifier(id) = &token.token {
                self.bump_sync(); // Consume the identifier
                id.clone()
            } else {
                return Err(ParseError::new(
                    format!(
                        "Expected identifier for interface name, found {:?}",
                        token.token
                    ),
                    token.line,
                    token.column,
                ));
            }
        } else {
            return Err(ParseError::new(
                "Expected identifier for interface name, found end of input".to_string(),
                line,
                column,
            ));
        };

        // For now, just create a simple interface definition
        Ok(Statement::InterfaceDefinition {
            name,
            extends: Vec::new(),
            required_actions: Vec::new(),
            line,
            column,
        })
    }

    pub fn parse_container_instantiation(&mut self) -> Result<Statement, ParseError> {
        let start_token = self.bump_sync().unwrap(); // Consume 'create'
        let line = start_token.line;
        let column = start_token.column;

        self.expect_token(Token::KeywordNew, "Expected 'new' after 'create'")?;

        // Check for deprecated "create new constant" syntax
        if let Some(token) = self.cursor.peek()
            && matches!(token.token, Token::KeywordConstant)
        {
            // This is the deprecated "create new constant" syntax
            eprintln!(
                "Warning: 'create new constant' syntax is deprecated and will be removed in a future version. Please use 'store new constant' instead."
            );

            self.bump_sync(); // Consume "constant"

            let name = self.parse_variable_name_list()?;
            self.expect_token(Token::KeywordAs, "Expected 'as' after constant name")?;
            let value = self.parse_expression()?;

            return Ok(Statement::VariableDeclaration {
                name,
                value,
                is_constant: true,
                line,
                column,
            });
        }

        // Parse container type
        let container_type = if let Some(token) = self.cursor.peek() {
            if let Token::Identifier(id) = &token.token {
                self.bump_sync(); // Consume the identifier
                id.clone()
            } else {
                return Err(ParseError::new(
                    format!(
                        "Expected identifier for container type, found {:?}",
                        token.token
                    ),
                    token.line,
                    token.column,
                ));
            }
        } else {
            return Err(ParseError::new(
                "Expected identifier for container type, found end of input".to_string(),
                line,
                column,
            ));
        };

        self.expect_token(Token::KeywordAs, "Expected 'as' after container type")?;

        // Parse instance name
        let instance_name = if let Some(token) = self.cursor.peek() {
            if let Token::Identifier(id) = &token.token {
                self.bump_sync(); // Consume the identifier
                id.clone()
            } else {
                return Err(ParseError::new(
                    format!(
                        "Expected identifier for instance name, found {:?}",
                        token.token
                    ),
                    token.line,
                    token.column,
                ));
            }
        } else {
            return Err(ParseError::new(
                "Expected identifier for instance name, found end of input".to_string(),
                line,
                column,
            ));
        };

        // Expect colon after instance declaration
        self.expect_token(Token::Colon, "Expected ':' after instance name")?;

        let (property_initializers, arguments) = self.parse_instantiation_body()?;

        Ok(Statement::ContainerInstantiation {
            container_type,
            instance_name,
            arguments,
            property_initializers,
            line,
            column,
        })
    }

    pub fn parse_event_definition(&mut self) -> Result<Statement, ParseError> {
        let start_token = self.bump_sync().unwrap(); // Consume 'event'
        let line = start_token.line;
        let column = start_token.column;

        // Parse event name
        let name = if let Some(token) = self.cursor.peek() {
            if let Token::Identifier(id) = &token.token {
                self.bump_sync(); // Consume the identifier
                id.clone()
            } else {
                return Err(ParseError::new(
                    format!(
                        "Expected identifier for event name, found {:?}",
                        token.token
                    ),
                    token.line,
                    token.column,
                ));
            }
        } else {
            return Err(ParseError::new(
                "Expected identifier for event name, found end of input".to_string(),
                line,
                column,
            ));
        };

        // For now, just create a simple event definition
        Ok(Statement::EventDefinition {
            name,
            parameters: Vec::new(),
            line,
            column,
        })
    }

    pub fn parse_event_trigger(&mut self) -> Result<Statement, ParseError> {
        let start_token = self.bump_sync().unwrap(); // Consume 'trigger'
        let line = start_token.line;
        let column = start_token.column;

        // Parse event name
        let name = if let Some(token) = self.cursor.peek() {
            if let Token::Identifier(id) = &token.token {
                self.bump_sync(); // Consume the identifier
                id.clone()
            } else {
                return Err(ParseError::new(
                    format!(
                        "Expected identifier for event name, found {:?}",
                        token.token
                    ),
                    token.line,
                    token.column,
                ));
            }
        } else {
            return Err(ParseError::new(
                "Expected identifier for event name, found end of input".to_string(),
                line,
                column,
            ));
        };

        // For now, just create a simple event trigger
        Ok(Statement::EventTrigger {
            name,
            arguments: Vec::new(),
            line,
            column,
        })
    }

    pub fn parse_event_handler(&mut self) -> Result<Statement, ParseError> {
        let start_token = self.bump_sync().unwrap(); // Consume 'on'
        let line = start_token.line;
        let column = start_token.column;

        // Parse event source
        let event_source = self.parse_expression()?;

        // Parse event name
        let event_name = if let Some(token) = self.cursor.peek() {
            if let Token::Identifier(id) = &token.token {
                self.bump_sync(); // Consume the identifier
                id.clone()
            } else {
                return Err(ParseError::new(
                    format!(
                        "Expected identifier for event name, found {:?}",
                        token.token
                    ),
                    token.line,
                    token.column,
                ));
            }
        } else {
            return Err(ParseError::new(
                "Expected identifier for event name, found end of input".to_string(),
                line,
                column,
            ));
        };

        // For now, just create a simple event handler
        Ok(Statement::EventHandler {
            event_source,
            event_name,
            handler_body: Vec::new(),
            line,
            column,
        })
    }

    // Helper methods for parsing container-related constructs
    fn parse_inheritance(&mut self) -> Result<(Option<String>, Vec<String>), ParseError> {
        let mut extends = None;
        let mut implements = Vec::new();

        // Check for 'extends' keyword
        if let Some(token) = self.cursor.peek()
            && token.token == Token::KeywordExtends
        {
            self.bump_sync(); // Consume 'extends'

            if let Some(token) = self.cursor.peek() {
                if let Token::Identifier(id) = &token.token {
                    extends = Some(id.clone());
                    self.bump_sync(); // Consume the identifier
                } else {
                    return Err(ParseError::new(
                        "Expected identifier after 'extends'".to_string(),
                        token.line,
                        token.column,
                    ));
                }
            } else {
                return Err(ParseError::new(
                    "Expected identifier after 'extends'".to_string(),
                    0,
                    0,
                ));
            }
        }

        // Check for 'implements' keyword
        if let Some(token) = self.cursor.peek()
            && token.token == Token::KeywordImplements
        {
            self.bump_sync(); // Consume 'implements'

            // Parse interface list
            loop {
                if let Some(token) = self.cursor.peek() {
                    if let Token::Identifier(id) = &token.token {
                        implements.push(id.clone());
                        self.bump_sync(); // Consume the identifier

                        // Check for comma to continue or break
                        if let Some(next_token) = self.cursor.peek() {
                            if next_token.token == Token::Comma {
                                self.bump_sync(); // Consume comma
                                continue;
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    } else {
                        return Err(ParseError::new(
                            "Expected identifier in implements list".to_string(),
                            token.line,
                            token.column,
                        ));
                    }
                } else {
                    return Err(ParseError::new(
                        "Expected identifier in implements list".to_string(),
                        0,
                        0,
                    ));
                }
            }
        }

        Ok((extends, implements))
    }

    #[allow(clippy::type_complexity)]
    fn parse_container_body(
        &mut self,
    ) -> Result<
        (
            Vec<PropertyDefinition>,
            Vec<Statement>,
            Vec<EventDefinition>,
            Vec<PropertyDefinition>,
            Vec<Statement>,
        ),
        ParseError,
    > {
        let mut properties = Vec::new();
        let mut methods = Vec::new();
        let mut events = Vec::new();
        let mut static_properties = Vec::new();
        let mut static_methods = Vec::new();

        // Parse container body until 'end'
        loop {
            if let Some(token) = self.cursor.peek() {
                match &token.token {
                    Token::KeywordEnd => {
                        self.bump_sync(); // Consume 'end'
                        break;
                    }
                    Token::KeywordProperty => {
                        let prop = self.parse_property_definition(false)?;
                        properties.push(prop);
                    }
                    Token::KeywordStatic => {
                        let static_token = self.bump_sync().unwrap(); // Consume 'static'
                        if let Some(next_token) = self.cursor.peek() {
                            match &next_token.token {
                                Token::KeywordProperty => {
                                    let prop = self.parse_property_definition(true)?;
                                    static_properties.push(prop);
                                }
                                Token::KeywordAction => {
                                    let method = self.parse_container_action_definition()?;
                                    static_methods.push(method);
                                }
                                _ => {
                                    return Err(ParseError::new(
                                        "Expected 'property' or 'action' after 'static'"
                                            .to_string(),
                                        next_token.line,
                                        next_token.column,
                                    ));
                                }
                            }
                        } else {
                            return Err(ParseError::new(
                                "Expected 'property' or 'action' after 'static'".to_string(),
                                static_token.line,
                                static_token.column,
                            ));
                        }
                    }
                    Token::KeywordAction => {
                        let method = self.parse_container_action_definition()?;
                        methods.push(method);
                    }
                    Token::KeywordEvent => {
                        let event = self.parse_event_definition_full()?;
                        events.push(event);
                    }
                    Token::Eol => {
                        self.bump_sync(); // Skip Eol between definitions
                        continue;
                    }
                    _ => {
                        return Err(ParseError::new(
                            format!("Unexpected token in container body: {:?}", token.token),
                            token.line,
                            token.column,
                        ));
                    }
                }
            } else {
                return Err(ParseError::new(
                    "Unexpected end of input in container body".to_string(),
                    0,
                    0,
                ));
            }
        }

        Ok((
            properties,
            methods,
            events,
            static_properties,
            static_methods,
        ))
    }

    fn parse_property_definition(
        &mut self,
        is_static: bool,
    ) -> Result<PropertyDefinition, ParseError> {
        let start_token = self.bump_sync().unwrap(); // Consume 'property'
        let line = start_token.line;
        let column = start_token.column;

        // Parse property name
        let name = if let Some(token) = self.cursor.peek() {
            if let Token::Identifier(id) = &token.token {
                self.bump_sync(); // Consume the identifier
                id.clone()
            } else {
                return Err(ParseError::new(
                    "Expected property name after 'property'".to_string(),
                    token.line,
                    token.column,
                ));
            }
        } else {
            return Err(ParseError::new(
                "Expected property name after 'property'".to_string(),
                line,
                column,
            ));
        };

        let property_type = if let Some(token) = self.cursor.peek() {
            if token.token == Token::Colon {
                self.bump_sync(); // Consume ':'

                if let Some(type_token) = self.cursor.peek() {
                    if let Token::Identifier(type_name) = &type_token.token {
                        self.bump_sync(); // Consume type name
                        Some(match type_name.as_str() {
                            "Text" => Type::Text,
                            "Number" => Type::Number,
                            "Boolean" => Type::Boolean,
                            "Nothing" => Type::Nothing,
                            "Pattern" => Type::Pattern,
                            _ => Type::Custom(type_name.clone()),
                        })
                    } else {
                        return Err(ParseError::new(
                            "Expected type name after ':'".to_string(),
                            type_token.line,
                            type_token.column,
                        ));
                    }
                } else {
                    return Err(ParseError::new(
                        "Expected type name after ':'".to_string(),
                        line,
                        column,
                    ));
                }
            } else {
                None
            }
        } else {
            None
        };

        let default_value = if let Some(token) = self.cursor.peek() {
            if token.token == Token::KeywordDefaults {
                self.bump_sync(); // Consume 'defaults'
                Some(self.parse_expression()?)
            } else {
                None
            }
        } else {
            None
        };

        Ok(PropertyDefinition {
            name,
            property_type,
            default_value,
            validation_rules: Vec::new(),
            is_static,
            visibility: Visibility::Public,
            line,
            column,
        })
    }

    fn parse_event_definition_full(&mut self) -> Result<EventDefinition, ParseError> {
        let start_token = self.bump_sync().unwrap(); // Consume 'event'
        let line = start_token.line;
        let column = start_token.column;

        // Parse event name
        let name = if let Some(token) = self.cursor.peek() {
            if let Token::Identifier(id) = &token.token {
                self.bump_sync(); // Consume the identifier
                id.clone()
            } else {
                return Err(ParseError::new(
                    "Expected event name after 'event'".to_string(),
                    token.line,
                    token.column,
                ));
            }
        } else {
            return Err(ParseError::new(
                "Expected event name after 'event'".to_string(),
                line,
                column,
            ));
        };

        let mut parameters = Vec::new();
        if let Some(token) = self.cursor.peek()
            && token.token == Token::KeywordNeeds
        {
            self.bump_sync(); // Consume 'needs'
            parameters = self.parse_parameter_list()?;
        }

        Ok(EventDefinition {
            name,
            parameters,
            line,
            column,
        })
    }

    fn parse_instantiation_body(
        &mut self,
    ) -> Result<(Vec<PropertyInitializer>, Vec<Argument>), ParseError> {
        let mut property_initializers = Vec::new();
        let arguments = Vec::new();

        // Parse instantiation body until 'end'
        while let Some(token) = self.cursor.peek() {
            match &token.token {
                Token::KeywordEnd => {
                    self.bump_sync(); // Consume 'end'
                    break;
                }
                Token::Eol => {
                    self.bump_sync(); // Skip Eol between property initializations
                    continue;
                }
                Token::Identifier(prop_name) => {
                    let name = prop_name.clone();
                    let prop_line = token.line;
                    let prop_column = token.column;
                    self.bump_sync(); // Consume property name

                    // Expect 'is' or ':'
                    if let Some(next_token) = self.cursor.peek() {
                        if next_token.token == Token::KeywordIs || next_token.token == Token::Colon
                        {
                            self.bump_sync(); // Consume 'is' or ':'

                            let value = self.parse_expression()?;
                            property_initializers.push(PropertyInitializer {
                                name,
                                value,
                                line: prop_line,
                                column: prop_column,
                            });
                        } else {
                            return Err(ParseError::new(
                                "Expected 'is' or ':' after property name".to_string(),
                                next_token.line,
                                next_token.column,
                            ));
                        }
                    } else {
                        return Err(ParseError::new(
                            "Expected 'is' or ':' after property name".to_string(),
                            prop_line,
                            prop_column,
                        ));
                    }
                }
                _ => {
                    return Err(ParseError::new(
                        format!("Unexpected token in instantiation body: {:?}", token.token),
                        token.line,
                        token.column,
                    ));
                }
            }
        }

        Ok((property_initializers, arguments))
    }

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
                    return Err(ParseError::new(
                        format!("Cannot use a number as a variable name: {:?}", token.token),
                        token.line,
                        token.column,
                    ));
                }
                Token::KeywordAs => {
                    return Err(ParseError::new(
                        "Expected a variable name before 'as'".to_string(),
                        token.line,
                        token.column,
                    ));
                }
                _ if token.token.is_structural_keyword() => {
                    return Err(ParseError::new(
                        format!(
                            "Cannot use reserved keyword '{:?}' as a variable name",
                            token.token
                        ),
                        token.line,
                        token.column,
                    ));
                }
                _ if token.token.is_contextual_keyword() => {
                    // Contextual keywords can be used as variable names
                    let name = self.get_token_text(&token.token);
                    self.bump_sync(); // Consume the contextual keyword
                    name_parts.push(name);
                }
                _ => {
                    return Err(ParseError::new(
                        format!(
                            "Expected identifier for variable name, found {:?}",
                            token.token
                        ),
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
                    return Err(ParseError::new(
                        format!(
                            "Expected 'as' after variable name, but found number: {:?}",
                            token.token
                        ),
                        token.line,
                        token.column,
                    ));
                }
                _ => {
                    return Err(ParseError::new(
                        format!(
                            "Expected 'as' after variable name, but found {:?}",
                            token.token
                        ),
                        token.line,
                        token.column,
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
            return Err(ParseError::new(
                "Expected variable name".to_string(),
                self.cursor.peek().map_or(0, |t| t.line),
                self.cursor.peek().map_or(0, |t| t.column),
            ));
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
                            Err(ParseError::new(
                                "Unexpected 'read' - did you mean 'read output from process'?"
                                    .to_string(),
                                token_pos.line,
                                token_pos.column,
                            ))
                        }
                    } else {
                        let token_pos = self.cursor.peek().unwrap();
                        Err(ParseError::new(
                            "Unexpected 'read' at end of input".to_string(),
                            token_pos.line,
                            token_pos.column,
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
            Err(ParseError::new("Unexpected end of input".to_string(), 0, 0))
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
