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
    known_actions: std::collections::HashSet<String>,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [TokenWithPosition]) -> Self {
        Parser {
            tokens: tokens.iter().peekable(),
            errors: Vec::with_capacity(4),
            known_actions: std::collections::HashSet::new(),
        }
    }

    pub fn parse(&mut self) -> Result<Program, Vec<ParseError>> {
        let mut program = Program::new();
        program.statements.reserve(self.tokens.clone().count() / 5);

        let mut last_line = 0;

        while self.tokens.peek().is_some() {
            let start_len = self.tokens.clone().count();

            if let Some(token) = self.tokens.peek() {
                if token.line > last_line && last_line > 0 {
                    // This is especially important for statements like "push" that don't have
                }
                last_line = token.line;
            }

            // Comprehensive handling of "end" tokens that might be left unconsumed
            // Check first two tokens to avoid borrow checker issues
            let mut tokens_clone = self.tokens.clone();
            if let Some(first_token) = tokens_clone.next() {
                if first_token.token == Token::KeywordEnd {
                    if let Some(second_token) = tokens_clone.next() {
                        match &second_token.token {
                            Token::KeywordAction => {
                                exec_trace!(
                                    "Consuming orphaned 'end action' at line {}",
                                    first_token.line
                                );
                                self.tokens.next(); // Consume "end"
                                self.tokens.next(); // Consume "action"
                                continue;
                            }
                            Token::KeywordCheck => {
                                exec_trace!(
                                    "Consuming orphaned 'end check' at line {}",
                                    first_token.line
                                );
                                self.tokens.next(); // Consume "end"
                                self.tokens.next(); // Consume "check"
                                continue;
                            }
                            Token::KeywordFor => {
                                exec_trace!(
                                    "Consuming orphaned 'end for' at line {}",
                                    first_token.line
                                );
                                self.tokens.next(); // Consume "end"
                                self.tokens.next(); // Consume "for"
                                continue;
                            }
                            Token::KeywordCount => {
                                exec_trace!(
                                    "Consuming orphaned 'end count' at line {}",
                                    first_token.line
                                );
                                self.tokens.next(); // Consume "end"
                                self.tokens.next(); // Consume "count"
                                continue;
                            }
                            Token::KeywordRepeat => {
                                exec_trace!(
                                    "Consuming orphaned 'end repeat' at line {}",
                                    first_token.line
                                );
                                self.tokens.next(); // Consume "end"
                                self.tokens.next(); // Consume "repeat"
                                continue;
                            }
                            Token::KeywordTry => {
                                exec_trace!(
                                    "Consuming orphaned 'end try' at line {}",
                                    first_token.line
                                );
                                self.tokens.next(); // Consume "end"
                                self.tokens.next(); // Consume "try"
                                continue;
                            }
                            Token::KeywordLoop => {
                                exec_trace!(
                                    "Consuming orphaned 'end loop' at line {}",
                                    first_token.line
                                );
                                self.tokens.next(); // Consume "end"
                                self.tokens.next(); // Consume "loop"
                                continue;
                            }
                            Token::KeywordWhile => {
                                exec_trace!(
                                    "Consuming orphaned 'end while' at line {}",
                                    first_token.line
                                );
                                self.tokens.next(); // Consume "end"
                                self.tokens.next(); // Consume "while"
                                continue;
                            }
                            Token::KeywordPattern => {
                                exec_trace!(
                                    "Consuming orphaned 'end pattern' at line {}",
                                    first_token.line
                                );
                                self.tokens.next(); // Consume "end"
                                self.tokens.next(); // Consume "pattern"
                                continue;
                            }
                            _ => {
                                // Standalone "end" or unexpected pattern - consume and log error
                                exec_trace!(
                                    "Found unexpected 'end' followed by {:?} at line {}",
                                    second_token.token,
                                    first_token.line
                                );
                                self.tokens.next(); // Consume "end"
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
                        self.tokens.next();
                        break;
                    }
                }
            }

            match self.parse_statement() {
                Ok(statement) => program.statements.push(statement),
                Err(error) => {
                    self.errors.push(error);

                    // This is especially important for consecutive push statements
                    let current_line = if let Some(token) = self.tokens.peek() {
                        token.line
                    } else {
                        0
                    };

                    while let Some(token) = self.tokens.peek() {
                        if token.line > current_line || Parser::is_statement_starter(&token.token) {
                            break;
                        }
                        self.tokens.next(); // Skip token
                    }
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

    fn is_statement_starter(token: &Token) -> bool {
        matches!(
            token,
            Token::KeywordStore
                | Token::KeywordCreate
                | Token::KeywordDisplay
                | Token::KeywordCheck
                | Token::KeywordIf
                | Token::KeywordCount
                | Token::KeywordFor
                | Token::KeywordDefine
                | Token::KeywordChange
                | Token::KeywordTry
                | Token::KeywordRepeat
                | Token::KeywordExit
                | Token::KeywordPush
                | Token::KeywordBreak
                | Token::KeywordContinue
                | Token::KeywordSkip
                | Token::KeywordOpen
                | Token::KeywordClose
                | Token::KeywordWait
                | Token::KeywordGive
                | Token::KeywordReturn
        )
    }

    #[allow(dead_code)]
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
                | Token::KeywordIf
                | Token::KeywordPush => {
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
                                exec_trace!(
                                    "Synchronizing: consuming {:?} after 'end'",
                                    next_token.token
                                );
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

    // Container-related parsing methods
    pub fn parse_container_definition(&mut self) -> Result<Statement, ParseError> {
        let start_token = self.tokens.next().unwrap(); // Consume 'create'
        let line = start_token.line;
        let column = start_token.column;

        self.expect_token(
            Token::KeywordContainer,
            "Expected 'container' after 'create'",
        )?;

        // Parse container name
        let name = if let Some(token) = self.tokens.peek() {
            if let Token::Identifier(id) = &token.token {
                self.tokens.next(); // Consume the identifier
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
        let start_token = self.tokens.next().unwrap(); // Consume 'create'
        let line = start_token.line;
        let column = start_token.column;

        self.expect_token(
            Token::KeywordInterface,
            "Expected 'interface' after 'create'",
        )?;

        // Parse interface name
        let name = if let Some(token) = self.tokens.peek() {
            if let Token::Identifier(id) = &token.token {
                self.tokens.next(); // Consume the identifier
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
        let start_token = self.tokens.next().unwrap(); // Consume 'create'
        let line = start_token.line;
        let column = start_token.column;

        self.expect_token(Token::KeywordNew, "Expected 'new' after 'create'")?;

        // Parse container type
        let container_type = if let Some(token) = self.tokens.peek() {
            if let Token::Identifier(id) = &token.token {
                self.tokens.next(); // Consume the identifier
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
        let instance_name = if let Some(token) = self.tokens.peek() {
            if let Token::Identifier(id) = &token.token {
                self.tokens.next(); // Consume the identifier
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
        let start_token = self.tokens.next().unwrap(); // Consume 'event'
        let line = start_token.line;
        let column = start_token.column;

        // Parse event name
        let name = if let Some(token) = self.tokens.peek() {
            if let Token::Identifier(id) = &token.token {
                self.tokens.next(); // Consume the identifier
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
        let start_token = self.tokens.next().unwrap(); // Consume 'trigger'
        let line = start_token.line;
        let column = start_token.column;

        // Parse event name
        let name = if let Some(token) = self.tokens.peek() {
            if let Token::Identifier(id) = &token.token {
                self.tokens.next(); // Consume the identifier
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
        let start_token = self.tokens.next().unwrap(); // Consume 'on'
        let line = start_token.line;
        let column = start_token.column;

        // Parse event source
        let event_source = self.parse_expression()?;

        // Parse event name
        let event_name = if let Some(token) = self.tokens.peek() {
            if let Token::Identifier(id) = &token.token {
                self.tokens.next(); // Consume the identifier
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

    pub fn parse_parent_method_call(&mut self) -> Result<Statement, ParseError> {
        let start_token = self.tokens.next().unwrap(); // Consume 'parent'
        let line = start_token.line;
        let column = start_token.column;

        // Parse method name
        let method_name = if let Some(token) = self.tokens.peek() {
            if let Token::Identifier(id) = &token.token {
                self.tokens.next(); // Consume the identifier
                id.clone()
            } else {
                return Err(ParseError::new(
                    format!(
                        "Expected identifier for method name, found {:?}",
                        token.token
                    ),
                    token.line,
                    token.column,
                ));
            }
        } else {
            return Err(ParseError::new(
                "Expected identifier for method name, found end of input".to_string(),
                line,
                column,
            ));
        };

        // For now, just create a simple parent method call
        Ok(Statement::ParentMethodCall {
            method_name,
            arguments: Vec::new(),
            line,
            column,
        })
    }

    // Helper methods for parsing container-related constructs
    fn parse_inheritance(&mut self) -> Result<(Option<String>, Vec<String>), ParseError> {
        let mut extends = None;
        let mut implements = Vec::new();

        // Check for 'extends' keyword
        if let Some(token) = self.tokens.peek() {
            if token.token == Token::KeywordExtends {
                self.tokens.next(); // Consume 'extends'

                if let Some(token) = self.tokens.peek() {
                    if let Token::Identifier(id) = &token.token {
                        extends = Some(id.clone());
                        self.tokens.next(); // Consume the identifier
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
        }

        // Check for 'implements' keyword
        if let Some(token) = self.tokens.peek() {
            if token.token == Token::KeywordImplements {
                self.tokens.next(); // Consume 'implements'

                // Parse interface list
                loop {
                    if let Some(token) = self.tokens.peek() {
                        if let Token::Identifier(id) = &token.token {
                            implements.push(id.clone());
                            self.tokens.next(); // Consume the identifier

                            // Check for comma to continue or break
                            if let Some(next_token) = self.tokens.peek() {
                                if next_token.token == Token::Comma {
                                    self.tokens.next(); // Consume comma
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
            if let Some(token) = self.tokens.peek() {
                match &token.token {
                    Token::KeywordEnd => {
                        self.tokens.next(); // Consume 'end'
                        break;
                    }
                    Token::KeywordProperty => {
                        let prop = self.parse_property_definition(false)?;
                        properties.push(prop);
                    }
                    Token::KeywordStatic => {
                        let static_token = self.tokens.next().unwrap(); // Consume 'static'
                        if let Some(next_token) = self.tokens.peek() {
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
        let start_token = self.tokens.next().unwrap(); // Consume 'property'
        let line = start_token.line;
        let column = start_token.column;

        // Parse property name
        let name = if let Some(token) = self.tokens.peek() {
            if let Token::Identifier(id) = &token.token {
                self.tokens.next(); // Consume the identifier
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

        let property_type = if let Some(token) = self.tokens.peek() {
            if token.token == Token::Colon {
                self.tokens.next(); // Consume ':'

                if let Some(type_token) = self.tokens.peek() {
                    if let Token::Identifier(type_name) = &type_token.token {
                        self.tokens.next(); // Consume type name
                        Some(Type::Custom(type_name.clone()))
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

        let default_value = if let Some(token) = self.tokens.peek() {
            if token.token == Token::KeywordDefaults {
                self.tokens.next(); // Consume 'defaults'
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
        let start_token = self.tokens.next().unwrap(); // Consume 'event'
        let line = start_token.line;
        let column = start_token.column;

        // Parse event name
        let name = if let Some(token) = self.tokens.peek() {
            if let Token::Identifier(id) = &token.token {
                self.tokens.next(); // Consume the identifier
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
        if let Some(token) = self.tokens.peek() {
            if token.token == Token::KeywordNeeds {
                self.tokens.next(); // Consume 'needs'
                parameters = self.parse_parameter_list()?;
            }
        }

        Ok(EventDefinition {
            name,
            parameters,
            line,
            column,
        })
    }

    fn parse_parameter_list(&mut self) -> Result<Vec<Parameter>, ParseError> {
        let mut parameters = Vec::new();

        while let Some(token) = self.tokens.peek() {
            if let Token::Identifier(param_name) = &token.token {
                let name = param_name.clone();
                let param_line = token.line;
                let param_column = token.column;
                self.tokens.next(); // Consume parameter name

                let param_type = if let Some(type_token) = self.tokens.peek() {
                    if type_token.token == Token::Colon {
                        self.tokens.next(); // Consume ':'

                        if let Some(type_name_token) = self.tokens.peek() {
                            if let Token::Identifier(type_name) = &type_name_token.token {
                                self.tokens.next(); // Consume type name
                                Some(Type::Custom(type_name.clone()))
                            } else {
                                return Err(ParseError::new(
                                    "Expected type name after ':'".to_string(),
                                    type_name_token.line,
                                    type_name_token.column,
                                ));
                            }
                        } else {
                            return Err(ParseError::new(
                                "Expected type name after ':'".to_string(),
                                param_line,
                                param_column,
                            ));
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                parameters.push(Parameter {
                    name,
                    param_type,
                    default_value: None,
                    line: param_line,
                    column: param_column,
                });

                // Check for comma to continue or break
                if let Some(next_token) = self.tokens.peek() {
                    if next_token.token == Token::Comma {
                        self.tokens.next(); // Consume comma
                        continue;
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

        Ok(parameters)
    }

    fn parse_instantiation_body(
        &mut self,
    ) -> Result<(Vec<PropertyInitializer>, Vec<Argument>), ParseError> {
        let mut property_initializers = Vec::new();
        let arguments = Vec::new();

        // Parse instantiation body until 'end'
        while let Some(token) = self.tokens.peek() {
            match &token.token {
                Token::KeywordEnd => {
                    self.tokens.next(); // Consume 'end'
                    break;
                }
                Token::Identifier(prop_name) => {
                    let name = prop_name.clone();
                    let prop_line = token.line;
                    let prop_column = token.column;
                    self.tokens.next(); // Consume property name

                    // Expect 'is' or ':'
                    if let Some(next_token) = self.tokens.peek() {
                        if next_token.token == Token::KeywordIs || next_token.token == Token::Colon
                        {
                            self.tokens.next(); // Consume 'is' or ':'

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

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        if let Some(token) = self.tokens.peek().cloned() {
            match &token.token {
                Token::KeywordStore => self.parse_variable_declaration(),
                Token::KeywordCreate => {
                    // Check if it's "create container", "create interface", "create new", "create pattern", or regular "create"
                    let mut tokens_clone = self.tokens.clone();
                    tokens_clone.next(); // Skip "create"
                    if let Some(next_token) = tokens_clone.next() {
                        match &next_token.token {
                            Token::KeywordContainer => self.parse_container_definition(),
                            Token::KeywordInterface => self.parse_interface_definition(),
                            Token::KeywordNew => self.parse_container_instantiation(),
                            Token::KeywordPattern => self.parse_create_pattern_statement(),
                            _ => self.parse_variable_declaration(), // Default to variable declaration
                        }
                    } else {
                        self.parse_variable_declaration() // Default to variable declaration
                    }
                }
                Token::KeywordDisplay => self.parse_display_statement(),
                Token::KeywordCheck => self.parse_if_statement(),
                Token::KeywordIf => self.parse_single_line_if(),
                Token::KeywordCount => self.parse_count_loop(),
                Token::KeywordFor => self.parse_for_each_loop(),
                Token::KeywordDefine => self.parse_action_definition(),
                Token::KeywordChange => self.parse_assignment(),
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

                    if let Some(token) = tokens_clone.next() {
                        if token.token == Token::KeywordFile {
                            if let Some(token) = tokens_clone.next() {
                                if token.token == Token::KeywordAt {
                                    if let Some(token) = tokens_clone.next() {
                                        if let Token::StringLiteral(_) = token.token {
                                            if let Some(token) = tokens_clone.next() {
                                                if token.token == Token::KeywordAnd {
                                                    if let Some(token) = tokens_clone.next() {
                                                        if token.token == Token::KeywordRead {
                                                            if let Some(token) = tokens_clone.next()
                                                            {
                                                                if token.token
                                                                    == Token::KeywordContent
                                                                {
                                                                    if let Some(token) =
                                                                        tokens_clone.next()
                                                                    {
                                                                        if token.token
                                                                            == Token::KeywordAs
                                                                        {
                                                                            if let Some(token) =
                                                                                tokens_clone.next()
                                                                            {
                                                                                if let Token::Identifier(_) = token.token {
                                                                                    has_read_pattern = true;
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if has_read_pattern {
                        self.parse_open_file_read_statement()
                    } else {
                        self.parse_open_file_statement()
                    }
                }
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
                _ => self.parse_expression_statement(),
            }
        } else {
            Err(ParseError::new("Unexpected end of input".to_string(), 0, 0))
        }
    }

    fn parse_variable_declaration(&mut self) -> Result<Statement, ParseError> {
        let token_pos = self.tokens.next().unwrap();
        let is_store = matches!(token_pos.token, Token::KeywordStore);
        let _keyword = if is_store { "store" } else { "create" };

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
                return Err(ParseError::new(
                    "Expected identifier after 'as'".to_string(),
                    token_pos.line,
                    token_pos.column,
                ));
            };

            let empty_list =
                Expression::Literal(Literal::List(Vec::new()), token_pos.line, token_pos.column);

            return Ok(Statement::VariableDeclaration {
                name: list_name,
                value: empty_list,
                line: token_pos.line,
                column: token_pos.column,
            });
        }

        if let Some(token) = self.tokens.peek().cloned() {
            if !matches!(token.token, Token::KeywordAs) {
                return Err(ParseError::new(
                    format!(
                        "Expected 'as' after variable name '{}', but found {:?}",
                        name, token.token
                    ),
                    token.line,
                    token.column,
                ));
            }
        } else {
            return Err(ParseError::new(
                format!("Expected 'as' after variable name '{name}', but found end of input"),
                token_pos.line,
                token_pos.column,
            ));
        }

        self.tokens.next(); // Consume the 'as' token

        let value = self.parse_expression()?;

        Ok(Statement::VariableDeclaration {
            name,
            value,
            line: token_pos.line,
            column: token_pos.column,
        })
    }

    fn parse_variable_name_list(&mut self) -> Result<String, ParseError> {
        let mut name_parts = Vec::with_capacity(3);

        if let Some(token) = self.tokens.peek().cloned() {
            match &token.token {
                Token::Identifier(id) => {
                    self.tokens.next(); // Consume the identifier
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
                _ if token.token.is_keyword() => {
                    return Err(ParseError::new(
                        format!("Cannot use keyword '{:?}' as a variable name", token.token),
                        token.line,
                        token.column,
                    ));
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

        while let Some(token) = self.tokens.peek().cloned() {
            match &token.token {
                Token::Identifier(id) => {
                    self.tokens.next(); // Consume the identifier
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

    fn expect_token(&mut self, expected: Token, error_message: &str) -> Result<(), ParseError> {
        if let Some(token) = self.tokens.peek().cloned() {
            if token.token == expected {
                self.tokens.next();
                Ok(())
            } else {
                Err(ParseError::new(
                    format!(
                        "{}: expected {:?}, found {:?}",
                        error_message, expected, token.token
                    ),
                    token.line,
                    token.column,
                ))
            }
        } else {
            Err(ParseError::new(
                format!("{error_message}: unexpected end of input"),
                0,
                0,
            ))
        }
    }

    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        self.parse_binary_expression(0) // Start with lowest precedence
    }

    fn parse_binary_expression(&mut self, precedence: u8) -> Result<Expression, ParseError> {
        let mut left = self.parse_primary_expression()?;

        let left_line = if let Some(token) = self.tokens.peek() {
            token.line
        } else {
            0
        };

        while let Some(token_pos) = self.tokens.peek().cloned() {
            let token = token_pos.token.clone();
            let line = token_pos.line;
            let column = token_pos.column;

            // If we're on a new line or at a statement starter, stop parsing this expression
            // This is crucial for statements like "push" that don't have explicit terminators
            if line > left_line || Parser::is_statement_starter(&token) {
                break;
            }

            let op = match token {
                Token::Plus => {
                    self.tokens.next(); // Consume "+"
                    Some((Operator::Plus, 1))
                }
                Token::KeywordPlus => {
                    self.tokens.next(); // Consume "plus"
                    Some((Operator::Plus, 1))
                }
                Token::KeywordMinus => {
                    self.tokens.next(); // Consume "minus"
                    Some((Operator::Minus, 1))
                }
                Token::KeywordTimes => {
                    self.tokens.next(); // Consume "times"
                    Some((Operator::Multiply, 2))
                }
                Token::KeywordDividedBy => {
                    self.tokens.next(); // Consume "divided by"
                    Some((Operator::Divide, 2))
                }
                Token::KeywordDivided => {
                    self.tokens.next(); // Consume "divided"

                    // Still handle the legacy case where "divided" and "by" are separate tokens
                    self.expect_token(Token::KeywordBy, "Expected 'by' after 'divided'")?;
                    self.tokens.next(); // Consume "by"
                    Some((Operator::Divide, 2))
                }
                Token::Equals => {
                    self.tokens.next(); // Consume "="
                    Some((Operator::Equals, 0))
                }
                Token::KeywordIs => {
                    self.tokens.next(); // Consume "is"

                    if let Some(next_token) = self.tokens.peek().cloned() {
                        match &next_token.token {
                            Token::KeywordEqual => {
                                self.tokens.next(); // Consume "equal"

                                if let Some(to_token) = self.tokens.peek().cloned() {
                                    if matches!(to_token.token, Token::KeywordTo) {
                                        self.tokens.next(); // Consume "to"
                                        Some((Operator::Equals, 0))
                                    } else {
                                        Some((Operator::Equals, 0)) // "is equal" without "to" is valid too
                                    }
                                } else {
                                    return Err(ParseError::new(
                                        "Unexpected end of input after 'is equal'".into(),
                                        line,
                                        column,
                                    ));
                                }
                            }
                            Token::KeywordNot => {
                                self.tokens.next(); // Consume "not"
                                Some((Operator::NotEquals, 0))
                            }
                            Token::KeywordGreater => {
                                self.tokens.next(); // Consume "greater"

                                if let Some(than_token) = self.tokens.peek().cloned() {
                                    if matches!(than_token.token, Token::KeywordThan) {
                                        self.tokens.next(); // Consume "than"
                                        Some((Operator::GreaterThan, 0))
                                    } else {
                                        Some((Operator::GreaterThan, 0)) // "is greater" without "than" is valid too
                                    }
                                } else {
                                    return Err(ParseError::new(
                                        "Unexpected end of input after 'is greater'".into(),
                                        line,
                                        column,
                                    ));
                                }
                            }
                            Token::KeywordLess => {
                                self.tokens.next(); // Consume "less"

                                if let Some(than_token) = self.tokens.peek().cloned() {
                                    if matches!(than_token.token, Token::KeywordThan) {
                                        self.tokens.next(); // Consume "than"

                                        // Check for "or equal to" after "less than"
                                        if let Some(or_token) = self.tokens.peek().cloned() {
                                            if matches!(or_token.token, Token::KeywordOr) {
                                                self.tokens.next(); // Consume "or"

                                                if let Some(equal_token) =
                                                    self.tokens.peek().cloned()
                                                {
                                                    if matches!(
                                                        equal_token.token,
                                                        Token::KeywordEqual
                                                    ) {
                                                        self.tokens.next(); // Consume "equal"

                                                        if let Some(to_token) =
                                                            self.tokens.peek().cloned()
                                                        {
                                                            if matches!(
                                                                to_token.token,
                                                                Token::KeywordTo
                                                            ) {
                                                                self.tokens.next(); // Consume "to"
                                                                Some((Operator::LessThanOrEqual, 0))
                                                            } else {
                                                                Some((Operator::LessThanOrEqual, 0)) // "or equal" without "to" is valid too
                                                            }
                                                        } else {
                                                            Some((Operator::LessThanOrEqual, 0)) // "or equal" without "to" is valid too
                                                        }
                                                    } else {
                                                        Some((Operator::LessThan, 0)) // Just "less than or" without "equal" is treated as "less than"
                                                    }
                                                } else {
                                                    Some((Operator::LessThan, 0)) // Just "less than or" without "equal" is treated as "less than"
                                                }
                                            } else {
                                                Some((Operator::LessThan, 0)) // Just "less than" without "or equal to"
                                            }
                                        } else {
                                            Some((Operator::LessThan, 0)) // Just "less than" without "or equal to"
                                        }
                                    } else {
                                        Some((Operator::LessThan, 0)) // "is less" without "than" is valid too
                                    }
                                } else {
                                    return Err(ParseError::new(
                                        "Unexpected end of input after 'is less'".into(),
                                        line,
                                        column,
                                    ));
                                }
                            }
                            _ => Some((Operator::Equals, 0)), // Simple "is" means equals
                        }
                    } else {
                        return Err(ParseError::new(
                            "Unexpected end of input after 'is'".into(),
                            line,
                            column,
                        ));
                    }
                }
                Token::KeywordWith => {
                    // Only create an ActionCall if this is a known action name
                    if let Expression::Variable(ref name, var_line, var_column) = left {
                        if self.known_actions.contains(name) {
                            // This is a known action, treat it as an action call
                            self.tokens.next(); // Consume "with"
                            let arguments = self.parse_argument_list()?;

                            left = Expression::ActionCall {
                                name: name.clone(),
                                arguments,
                                line: var_line,
                                column: var_column,
                            };
                            continue; // Skip the rest of the loop since we've already updated left
                        }
                    }

                    // Default case - treat as concatenation
                    self.tokens.next(); // Consume "with"
                    let right = self.parse_expression()?;
                    left = Expression::Concatenation {
                        left: Box::new(left),
                        right: Box::new(right),
                        line: token_pos.line,
                        column: token_pos.column,
                    };
                    continue; // Skip the rest of the loop since we've already updated left
                }
                Token::KeywordAnd => {
                    self.tokens.next(); // Consume "and"
                    Some((Operator::And, 0))
                }
                Token::KeywordOr => {
                    self.tokens.next(); // Consume "or"

                    // Handle "or equal to" as a special case
                    if let Some(equal_token) = self.tokens.peek().cloned() {
                        if matches!(equal_token.token, Token::KeywordEqual) {
                            self.tokens.next(); // Consume "equal"

                            if let Some(to_token) = self.tokens.peek().cloned() {
                                if matches!(to_token.token, Token::KeywordTo) {
                                    self.tokens.next(); // Consume "to"

                                    if let Expression::BinaryOperation {
                                        operator,
                                        left: left_expr,
                                        right: right_expr,
                                        line: op_line,
                                        column: op_column,
                                    } = &left
                                    {
                                        if *operator == Operator::LessThan {
                                            left = Expression::BinaryOperation {
                                                left: left_expr.clone(),
                                                operator: Operator::LessThanOrEqual,
                                                right: right_expr.clone(),
                                                line: *op_line,
                                                column: *op_column,
                                            };
                                            continue;
                                        } else if *operator == Operator::GreaterThan {
                                            left = Expression::BinaryOperation {
                                                left: left_expr.clone(),
                                                operator: Operator::GreaterThanOrEqual,
                                                right: right_expr.clone(),
                                                line: *op_line,
                                                column: *op_column,
                                            };
                                            continue;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    Some((Operator::Or, 0))
                }
                Token::KeywordMatches => {
                    self.tokens.next(); // Consume "matches"

                    // Check if next token is "pattern" keyword (optional)
                    if let Some(pattern_token) = self.tokens.peek().cloned() {
                        if matches!(pattern_token.token, Token::KeywordPattern) {
                            self.tokens.next(); // Consume "pattern"
                        }
                    }

                    let pattern_expr = self.parse_binary_expression(precedence + 1)?;

                    left = Expression::PatternMatch {
                        text: Box::new(left),
                        pattern: Box::new(pattern_expr),
                        line,
                        column,
                    };
                    continue; // Skip the rest of the loop since we've already updated left
                }
                Token::KeywordFind => {
                    self.tokens.next(); // Consume "find"

                    // Check if next token is "pattern" keyword (optional)
                    if let Some(pattern_token) = self.tokens.peek().cloned() {
                        if matches!(pattern_token.token, Token::KeywordPattern) {
                            self.tokens.next(); // Consume "pattern"
                        }
                    }

                    let pattern_expr = self.parse_binary_expression(precedence + 1)?;

                    if let Some(in_token) = self.tokens.peek().cloned() {
                        if matches!(in_token.token, Token::KeywordIn) {
                            self.tokens.next(); // Consume "in"

                            let text_expr = self.parse_binary_expression(precedence + 1)?;

                            left = Expression::PatternFind {
                                text: Box::new(text_expr),
                                pattern: Box::new(pattern_expr),
                                line,
                                column,
                            };
                            continue; // Skip the rest of the loop since we've already updated left
                        }
                    }

                    left = Expression::PatternFind {
                        text: Box::new(left),
                        pattern: Box::new(pattern_expr),
                        line,
                        column,
                    };
                    continue; // Skip the rest of the loop since we've already updated left
                }
                Token::KeywordReplace => {
                    self.tokens.next(); // Consume "replace"

                    // Check if next token is "pattern" keyword (optional)
                    if let Some(pattern_token) = self.tokens.peek().cloned() {
                        if matches!(pattern_token.token, Token::KeywordPattern) {
                            self.tokens.next(); // Consume "pattern"
                        }
                    }

                    let pattern_expr = self.parse_binary_expression(precedence + 1)?;

                    if let Some(with_token) = self.tokens.peek().cloned() {
                        if matches!(with_token.token, Token::KeywordWith) {
                            self.tokens.next(); // Consume "with"

                            let replacement_expr = self.parse_binary_expression(precedence + 1)?;

                            if let Some(in_token) = self.tokens.peek().cloned() {
                                if matches!(in_token.token, Token::KeywordIn) {
                                    self.tokens.next(); // Consume "in"

                                    let text_expr = self.parse_binary_expression(precedence + 1)?;

                                    left = Expression::PatternReplace {
                                        text: Box::new(text_expr),
                                        pattern: Box::new(pattern_expr),
                                        replacement: Box::new(replacement_expr),
                                        line,
                                        column,
                                    };
                                    continue; // Skip the rest of the loop since we've already updated left
                                }
                            }

                            left = Expression::PatternReplace {
                                text: Box::new(left),
                                pattern: Box::new(pattern_expr),
                                replacement: Box::new(replacement_expr),
                                line,
                                column,
                            };
                            continue; // Skip the rest of the loop since we've already updated left
                        }
                    }

                    return Err(ParseError::new(
                        "Expected 'with' after pattern in replace operation".to_string(),
                        line,
                        column,
                    ));
                }
                Token::KeywordSplit => {
                    self.tokens.next(); // Consume "split"

                    // Handle "split text on pattern name" syntax
                    let text_expr = self.parse_binary_expression(precedence + 1)?;

                    if let Some(on_token) = self.tokens.peek().cloned() {
                        if matches!(on_token.token, Token::KeywordOn) {
                            self.tokens.next(); // Consume "on"

                            // Check if next token is "pattern" keyword (optional)
                            if let Some(pattern_token) = self.tokens.peek().cloned() {
                                if matches!(pattern_token.token, Token::KeywordPattern) {
                                    self.tokens.next(); // Consume "pattern"
                                }
                            }

                            let pattern_expr = self.parse_binary_expression(precedence + 1)?;

                            left = Expression::PatternSplit {
                                text: Box::new(text_expr),
                                pattern: Box::new(pattern_expr),
                                line,
                                column,
                            };
                            continue; // Skip the rest of the loop since we've already updated left
                        }
                    }

                    return Err(ParseError::new(
                        "Expected 'on' after text in split operation".to_string(),
                        line,
                        column,
                    ));
                }
                Token::KeywordContains => {
                    self.tokens.next(); // Consume "contains"

                    if let Some(pattern_token) = self.tokens.peek().cloned() {
                        if matches!(pattern_token.token, Token::KeywordPattern) {
                            self.tokens.next(); // Consume "pattern"

                            let pattern_expr = self.parse_binary_expression(precedence + 1)?;

                            left = Expression::PatternMatch {
                                text: Box::new(left),
                                pattern: Box::new(pattern_expr),
                                line,
                                column,
                            };
                            continue; // Skip the rest of the loop since we've already updated left
                        }
                    }

                    Some((Operator::Contains, 0))
                }
                Token::Colon => {
                    self.tokens.next(); // Consume ":"
                    continue;
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
            let result = match &token.token {
                Token::LeftBracket => {
                    let bracket_token = self.tokens.next().unwrap(); // Consume '['

                    // Check for empty list
                    if let Some(next_token) = self.tokens.peek() {
                        if next_token.token == Token::RightBracket {
                            self.tokens.next(); // Consume ']'
                            return Ok(Expression::Literal(
                                Literal::List(Vec::new()),
                                bracket_token.line,
                                bracket_token.column,
                            ));
                        }
                    }

                    let mut elements = Vec::new();

                    elements.push(self.parse_expression()?);

                    while let Some(next_token) = self.tokens.peek() {
                        if next_token.token == Token::RightBracket {
                            self.tokens.next(); // Consume ']'
                            return Ok(Expression::Literal(
                                Literal::List(elements),
                                bracket_token.line,
                                bracket_token.column,
                            ));
                        } else if next_token.token == Token::KeywordAnd
                            || next_token.token == Token::Colon
                        {
                            self.tokens.next(); // Consume separator
                            elements.push(self.parse_expression()?);
                        } else {
                            return Err(ParseError::new(
                                format!(
                                    "Expected ']' or 'and' in list literal, found {:?}",
                                    next_token.token
                                ),
                                next_token.line,
                                next_token.column,
                            ));
                        }
                    }

                    return Err(ParseError::new(
                        "Unexpected end of input while parsing list literal".into(),
                        bracket_token.line,
                        bracket_token.column,
                    ));
                }
                Token::LeftParen => {
                    self.tokens.next(); // Consume '('
                    let expr = self.parse_expression()?;

                    if let Some(token) = self.tokens.peek().cloned() {
                        if token.token == Token::RightParen {
                            self.tokens.next(); // Consume ')'
                            return Ok(expr);
                        } else {
                            return Err(ParseError::new(
                                format!("Expected closing parenthesis, found {:?}", token.token),
                                token.line,
                                token.column,
                            ));
                        }
                    } else {
                        return Err(ParseError::new(
                            "Expected closing parenthesis, found end of input".into(),
                            token.line,
                            token.column,
                        ));
                    }
                }
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
                    self.tokens.next();

                    // Check for property access (dot notation)
                    if let Some(next_token) = self.tokens.peek().cloned() {
                        if next_token.token == Token::Dot {
                            self.tokens.next(); // Consume '.'

                            if let Some(property_token) = self.tokens.peek().cloned() {
                                if let Token::Identifier(property_name) = &property_token.token {
                                    self.tokens.next(); // Consume property name

                                    // Check for method call with parentheses
                                    if let Some(paren_token) = self.tokens.peek().cloned() {
                                        if paren_token.token == Token::LeftParen {
                                            self.tokens.next(); // Consume '('

                                            let mut arguments = Vec::new();

                                            if let Some(next_token) = self.tokens.peek() {
                                                if next_token.token != Token::RightParen {
                                                    let expr = self.parse_expression()?;
                                                    arguments.push(Argument {
                                                        name: None,
                                                        value: expr,
                                                    });

                                                    while let Some(comma_token) = self.tokens.peek()
                                                    {
                                                        if comma_token.token == Token::Comma {
                                                            self.tokens.next(); // Consume ','
                                                            let expr = self.parse_expression()?;
                                                            arguments.push(Argument {
                                                                name: None,
                                                                value: expr,
                                                            });
                                                        } else {
                                                            break;
                                                        }
                                                    }
                                                }
                                            }

                                            self.expect_token(
                                                Token::RightParen,
                                                "Expected ')' after method arguments",
                                            )?;

                                            return Ok(Expression::MethodCall {
                                                object: Box::new(Expression::Variable(
                                                    name.clone(),
                                                    token.line,
                                                    token.column,
                                                )),
                                                method: property_name.clone(),
                                                arguments,
                                                line: token.line,
                                                column: token.column,
                                            });
                                        }
                                    }

                                    // Property access without method call
                                    return Ok(Expression::PropertyAccess {
                                        object: Box::new(Expression::Variable(
                                            name.clone(),
                                            token.line,
                                            token.column,
                                        )),
                                        property: property_name.clone(),
                                        line: token.line,
                                        column: token.column,
                                    });
                                } else {
                                    return Err(ParseError::new(
                                        "Expected property name after '.'".to_string(),
                                        property_token.line,
                                        property_token.column,
                                    ));
                                }
                            } else {
                                return Err(ParseError::new(
                                    "Expected property name after '.'".to_string(),
                                    token.line,
                                    token.column,
                                ));
                            }
                        } else if let Token::Identifier(id) = &next_token.token {
                            if id.to_lowercase() == "with" {
                                self.tokens.next(); // Consume "with"

                                let arguments = self.parse_argument_list()?;

                                let token_line = token.line;
                                let token_column = token.column;
                                return Ok(Expression::ActionCall {
                                    name: name.clone(),
                                    arguments,
                                    line: token_line,
                                    column: token_column,
                                });
                            }
                        }
                    }

                    let is_standalone = false;

                    let token_line = token.line;
                    let token_column = token.column;

                    if is_standalone {
                        exec_trace!(
                            "Found standalone identifier '{}', treating as function call",
                            name
                        );
                        Ok(Expression::ActionCall {
                            name: name.clone(),
                            arguments: Vec::new(),
                            line: token_line,
                            column: token_column,
                        })
                    } else {
                        Ok(Expression::Variable(name.clone(), token_line, token_column))
                    }
                }
                Token::KeywordNot => {
                    self.tokens.next(); // Consume "not"
                    let expr = self.parse_primary_expression()?;
                    let token_line = token.line;
                    let token_column = token.column;
                    Ok(Expression::UnaryOperation {
                        operator: UnaryOperator::Not,
                        expression: Box::new(expr),
                        line: token_line,
                        column: token_column,
                    })
                }
                Token::KeywordWith => {
                    self.tokens.next(); // Consume "with"
                    let expr = self.parse_expression()?;
                    Ok(expr)
                }
                Token::KeywordCount => {
                    self.tokens.next(); // Consume "count"
                    let token_line = token.line;
                    let token_column = token.column;
                    Ok(Expression::Variable(
                        "count".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordPattern => {
                    self.tokens.next(); // Consume "pattern"

                    if let Some(pattern_token) = self.tokens.peek().cloned() {
                        if let Token::StringLiteral(pattern) = &pattern_token.token {
                            let token_pos = self.tokens.next().unwrap();
                            return Ok(Expression::Literal(
                                Literal::Pattern(pattern.clone()),
                                token_pos.line,
                                token_pos.column,
                            ));
                        } else {
                            return Err(ParseError::new(
                                format!(
                                    "Expected string literal after 'pattern', found {:?}",
                                    pattern_token.token
                                ),
                                pattern_token.line,
                                pattern_token.column,
                            ));
                        }
                    } else {
                        return Err(ParseError::new(
                            "Unexpected end of input after 'pattern'".to_string(),
                            token.line,
                            token.column,
                        ));
                    }
                }
                Token::KeywordLoop => {
                    self.tokens.next(); // Consume "loop"
                    let token_line = token.line;
                    let token_column = token.column;
                    Ok(Expression::Variable(
                        "loop".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordRepeat => {
                    self.tokens.next(); // Consume "repeat"
                    let token_line = token.line;
                    let token_column = token.column;
                    Ok(Expression::Variable(
                        "repeat".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordExit => {
                    self.tokens.next(); // Consume "exit"
                    let token_line = token.line;
                    let token_column = token.column;
                    Ok(Expression::Variable(
                        "exit".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordBack => {
                    self.tokens.next(); // Consume "back"
                    let token_line = token.line;
                    let token_column = token.column;
                    Ok(Expression::Variable(
                        "back".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordTry => {
                    self.tokens.next(); // Consume "try"
                    let token_line = token.line;
                    let token_column = token.column;
                    Ok(Expression::Variable(
                        "try".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordWhen => {
                    self.tokens.next(); // Consume "when"
                    let token_line = token.line;
                    let token_column = token.column;
                    Ok(Expression::Variable(
                        "when".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordError => {
                    self.tokens.next(); // Consume "error"
                    let token_line = token.line;
                    let token_column = token.column;
                    Ok(Expression::Variable(
                        "error".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordFind => {
                    self.tokens.next(); // Consume "find"
                    let pattern_expr = self.parse_expression()?;
                    self.expect_token(
                        Token::KeywordIn,
                        "Expected 'in' after pattern in find expression",
                    )?;
                    let text_expr = self.parse_expression()?;
                    Ok(Expression::PatternFind {
                        pattern: Box::new(pattern_expr),
                        text: Box::new(text_expr),
                        line: token.line,
                        column: token.column,
                    })
                }
                Token::KeywordReplace => {
                    self.tokens.next(); // Consume "replace"
                    let pattern_expr = self.parse_primary_expression()?;
                    self.expect_token(
                        Token::KeywordWith,
                        "Expected 'with' after pattern in replace expression",
                    )?;
                    let replacement_expr = self.parse_expression()?;
                    self.expect_token(
                        Token::KeywordIn,
                        "Expected 'in' after replacement in replace expression",
                    )?;
                    let text_expr = self.parse_expression()?;
                    Ok(Expression::PatternReplace {
                        pattern: Box::new(pattern_expr),
                        replacement: Box::new(replacement_expr),
                        text: Box::new(text_expr),
                        line: token.line,
                        column: token.column,
                    })
                }
                Token::KeywordSplit => {
                    self.tokens.next(); // Consume "split"
                    let text_expr = self.parse_expression()?;
                    self.expect_token(
                        Token::KeywordOn,
                        "Expected 'on' after text in split expression",
                    )?;
                    self.expect_token(
                        Token::KeywordPattern,
                        "Expected 'pattern' after 'on' in split expression",
                    )?;
                    let pattern_expr = self.parse_expression()?;
                    Ok(Expression::PatternSplit {
                        text: Box::new(text_expr),
                        pattern: Box::new(pattern_expr),
                        line: token.line,
                        column: token.column,
                    })
                }
                _ => Err(ParseError::new(
                    format!("Unexpected token in expression: {:?}", token.token),
                    token.line,
                    token.column,
                )),
            };

            if let Ok(mut expr) = result {
                while let Some(token) = self.tokens.peek().cloned() {
                    match &token.token {
                        Token::KeywordOf => {
                            self.tokens.next(); // Consume "of"

                            // Parse the first argument after "of"
                            let first_arg = self.parse_expression()?;

                            let is_function_call = matches!(
                                expr,
                                Expression::Variable(_, _, _) | Expression::FunctionCall { .. }
                            );

                            if is_function_call {
                                let mut arguments = Vec::with_capacity(4);

                                arguments.push(Argument {
                                    name: None,
                                    value: first_arg,
                                });

                                while let Some(and_token) = self.tokens.peek().cloned() {
                                    if let Token::KeywordAnd = &and_token.token {
                                        self.tokens.next(); // Consume "and"

                                        let arg_value = self.parse_expression()?;

                                        arguments.push(Argument {
                                            name: None,
                                            value: arg_value,
                                        });
                                    } else {
                                        break;
                                    }
                                }

                                expr = Expression::FunctionCall {
                                    function: Box::new(expr),
                                    arguments,
                                    line: token.line,
                                    column: token.column,
                                };
                            } else {
                                return Err(ParseError::new(
                                    "Member access not supported with expression arguments"
                                        .to_string(),
                                    token.line,
                                    token.column,
                                ));
                            }
                        }
                        Token::KeywordAt => {
                            self.tokens.next(); // Consume "at"

                            let index = self.parse_expression()?;

                            expr = Expression::IndexAccess {
                                collection: Box::new(expr),
                                index: Box::new(index),
                                line: token.line,
                                column: token.column,
                            };
                        }
                        // Handle static member access: "Container.staticMember"
                        Token::Identifier(id) if id == "." => {
                            self.tokens.next(); // Consume "."

                            if let Some(member_token) = self.tokens.peek().cloned() {
                                if let Token::Identifier(member) = &member_token.token {
                                    self.tokens.next(); // Consume member name

                                    // Extract container name from expression
                                    let container = if let Expression::Variable(name, _, _) = &expr
                                    {
                                        name.clone()
                                    } else {
                                        return Err(ParseError::new(
                                            "Static member access requires a container name"
                                                .to_string(),
                                            token.line,
                                            token.column,
                                        ));
                                    };

                                    expr = Expression::StaticMemberAccess {
                                        container,
                                        member: member.clone(),
                                        line: token.line,
                                        column: token.column,
                                    };
                                } else {
                                    return Err(ParseError::new(
                                        format!(
                                            "Expected identifier after '.', found {:?}",
                                            member_token.token
                                        ),
                                        member_token.line,
                                        member_token.column,
                                    ));
                                }
                            } else {
                                return Err(ParseError::new(
                                    "Unexpected end of input after '.'".to_string(),
                                    token.line,
                                    token.column,
                                ));
                            }
                        }
                        _ => break,
                    }
                }

                Ok(expr)
            } else {
                result
            }
        } else {
            Err(ParseError::new(
                "Unexpected end of input while parsing expression".to_string(),
                0,
                0,
            ))
        }
    }

    fn parse_display_statement(&mut self) -> Result<Statement, ParseError> {
        self.tokens.next(); // Consume "display"

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
            };
        };

        Ok(Statement::DisplayStatement {
            value: expr,
            line: token_pos.line,
            column: token_pos.column,
        })
    }

    fn parse_if_statement(&mut self) -> Result<Statement, ParseError> {
        let check_token = self.tokens.next().unwrap(); // Consume "check" and store for line/column info

        self.expect_token(Token::KeywordIf, "Expected 'if' after 'check'")?;

        let condition = self.parse_expression()?;

        if let Some(token) = self.tokens.peek() {
            if matches!(token.token, Token::Colon) {
                self.tokens.next(); // Consume the colon if present
            }
        }

        let mut then_block = Vec::with_capacity(8);

        while let Some(token) = self.tokens.peek().cloned() {
            match &token.token {
                Token::KeywordOtherwise | Token::KeywordEnd => {
                    break;
                }
                _ => match self.parse_statement() {
                    Ok(stmt) => then_block.push(stmt),
                    Err(e) => return Err(e),
                },
            }
        }

        // Handle the "otherwise" clause (else block)
        let else_block = if let Some(token) = self.tokens.peek() {
            if matches!(token.token, Token::KeywordOtherwise) {
                self.tokens.next(); // Consume "otherwise"

                if let Some(token) = self.tokens.peek() {
                    if matches!(token.token, Token::Colon) {
                        self.tokens.next(); // Consume the colon if present
                    }
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

        // Handle the "end check" part
        if let Some(&token) = self.tokens.peek() {
            if matches!(token.token, Token::KeywordEnd) {
                self.tokens.next(); // Consume "end"

                // Look for the "check" after "end"
                if let Some(&next_token) = self.tokens.peek() {
                    if matches!(next_token.token, Token::KeywordCheck) {
                        self.tokens.next(); // Consume "check"
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

    fn parse_single_line_if(&mut self) -> Result<Statement, ParseError> {
        let if_token = self.tokens.next().unwrap(); // Consume "if"

        let condition = self.parse_expression()?;

        self.expect_token(Token::KeywordThen, "Expected 'then' after if condition")?;

        // Check if this is a multi-line if by looking ahead for newlines or multiple statements
        let mut then_block = Vec::new();
        let mut is_multiline = false;

        // Parse then block - could be single statement or multiple statements
        while let Some(token) = self.tokens.peek() {
            match &token.token {
                Token::KeywordOtherwise | Token::KeywordEnd => {
                    is_multiline = true;
                    break;
                }
                Token::Newline => {
                    self.tokens.next(); // Consume newline
                    is_multiline = true;
                    continue;
                }
                _ => {
                    let stmt = self.parse_statement()?;
                    then_block.push(stmt);

                    // Check if there's more content after this statement
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

        // Handle else block
        let else_block = if let Some(token) = self.tokens.peek() {
            if matches!(token.token, Token::KeywordOtherwise) {
                self.tokens.next(); // Consume "otherwise"

                let mut else_stmts = Vec::new();
                while let Some(token) = self.tokens.peek() {
                    match &token.token {
                        Token::KeywordEnd => break,
                        Token::Newline => {
                            self.tokens.next(); // Consume newline
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

    fn parse_for_each_loop(&mut self) -> Result<Statement, ParseError> {
        self.tokens.next(); // Consume "for"

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
                self.tokens.next(); // Consume "reversed"
                true
            } else {
                false
            }
        } else {
            false
        };

        let collection = self.parse_expression()?;

        if let Some(token) = self.tokens.peek() {
            if matches!(token.token, Token::Colon) {
                self.tokens.next(); // Consume the colon if present
            }
        }

        let mut body = Vec::with_capacity(10);

        while let Some(token) = self.tokens.peek().cloned() {
            if matches!(token.token, Token::KeywordEnd) {
                break;
            }

            match self.parse_statement() {
                Ok(stmt) => body.push(stmt),
                Err(e) => return Err(e),
            }
        }

        self.expect_token(Token::KeywordEnd, "Expected 'end' after for-each loop body")?;
        self.expect_token(Token::KeywordFor, "Expected 'for' after 'end'")?;

        let token_pos = self.tokens.peek().map_or(
            &TokenWithPosition {
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

    fn parse_count_loop(&mut self) -> Result<Statement, ParseError> {
        self.tokens.next(); // Consume "count"

        self.expect_token(Token::KeywordFrom, "Expected 'from' after 'count'")?;

        let start = self.parse_expression()?;

        let downward = if let Some(token) = self.tokens.peek() {
            if let Token::Identifier(id) = &token.token {
                if id.to_lowercase() == "down" {
                    self.tokens.next(); // Consume "down"
                    self.expect_token(Token::KeywordTo, "Expected 'to' after 'down'")?;
                    true
                } else if matches!(token.token, Token::KeywordTo) {
                    self.tokens.next(); // Consume "to"
                    false
                } else {
                    return Err(ParseError::new(
                        format!("Expected 'to' or 'down to', found {:?}", token.token),
                        token.line,
                        token.column,
                    ));
                }
            } else if matches!(token.token, Token::KeywordTo) {
                self.tokens.next(); // Consume "to"
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
                self.tokens.next(); // Consume "by"
                Some(self.parse_expression()?)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(token) = self.tokens.peek() {
            if matches!(token.token, Token::Colon) {
                self.tokens.next(); // Consume the colon if present
            }
        }

        let mut body = Vec::with_capacity(10);

        while let Some(token) = self.tokens.peek().cloned() {
            if matches!(token.token, Token::KeywordEnd) {
                break;
            }

            match self.parse_statement() {
                Ok(stmt) => body.push(stmt),
                Err(e) => return Err(e),
            }
        }

        self.expect_token(Token::KeywordEnd, "Expected 'end' after count loop body")?;
        self.expect_token(Token::KeywordCount, "Expected 'count' after 'end'")?;

        let token_pos = self.tokens.peek().map_or(
            &TokenWithPosition {
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

    fn parse_action_definition(&mut self) -> Result<Statement, ParseError> {
        exec_trace!("Parsing action definition");
        self.tokens.next(); // Consume "define"

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

        if let Some(token) = self.tokens.peek().cloned() {
            if matches!(token.token, Token::KeywordNeeds)
                || matches!(token.token, Token::KeywordWith)
            {
                let _keyword = if matches!(token.token, Token::KeywordNeeds) {
                    "needs"
                } else {
                    "with"
                };
                exec_trace!("Found '{}' keyword, parsing parameters", _keyword);
                self.tokens.next(); // Consume "needs" or "with"

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
                        if matches!(token.token, Token::KeywordAs) {
                            self.tokens.next(); // Consume "as"

                            if let Some(type_token) = self.tokens.peek() {
                                if let Token::Identifier(type_name) = &type_token.token {
                                    self.tokens.next();

                                    let typ = match type_name.as_str() {
                                        "text" => Type::Text,
                                        "number" => Type::Number,
                                        "boolean" => Type::Boolean,
                                        "nothing" => Type::Nothing,
                                        _ => Type::Custom(type_name.clone()),
                                    };

                                    Some(typ)
                                } else {
                                    return Err(ParseError::new(
                                        format!(
                                            "Expected type name after 'as', found {:?}",
                                            type_token.token
                                        ),
                                        type_token.line,
                                        type_token.column,
                                    ));
                                }
                            } else {
                                return Err(ParseError::new(
                                    "Unexpected end of input after 'as'".to_string(),
                                    0,
                                    0,
                                ));
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    let default_value = if let Some(token) = self.tokens.peek() {
                        if let Token::Identifier(id) = &token.token {
                            if id.to_lowercase() == "default" {
                                self.tokens.next(); // Consume "default"

                                Some(self.parse_expression()?)
                            } else {
                                None
                            }
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

                    if let Some(token) = self.tokens.peek().cloned() {
                        if matches!(token.token, Token::KeywordAnd)
                            || matches!(token.token, Token::Identifier(ref id) if id.to_lowercase() == "and")
                        {
                            self.tokens.next(); // Consume "and"
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
        }

        let return_type = if let Some(token) = self.tokens.peek() {
            if let Token::Identifier(id) = &token.token {
                if id.to_lowercase() == "returns" {
                    self.tokens.next(); // Consume "returns"

                    if let Some(type_token) = self.tokens.peek() {
                        if let Token::Identifier(type_name) = &type_token.token {
                            self.tokens.next();

                            let typ = match type_name.as_str() {
                                "text" => Type::Text,
                                "number" => Type::Number,
                                "boolean" => Type::Boolean,
                                "nothing" => Type::Nothing,
                                _ => Type::Custom(type_name.clone()),
                            };

                            Some(typ)
                        } else {
                            return Err(ParseError::new(
                                format!(
                                    "Expected type name after 'returns', found {:?}",
                                    type_token.token
                                ),
                                type_token.line,
                                type_token.column,
                            ));
                        }
                    } else {
                        return Err(ParseError::new(
                            "Unexpected end of input after 'returns'".to_string(),
                            0,
                            0,
                        ));
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

        // Check for KeywordAnd that might be mistakenly present after the last parameter
        if let Some(token) = self.tokens.peek().cloned() {
            if let Token::Identifier(id) = &token.token {
                if id == "and" {
                    self.tokens.next(); // Consume the extra "and"
                }
            }
        }

        self.expect_token(Token::Colon, "Expected ':' after action definition")?;

        let mut body = Vec::with_capacity(10);

        while let Some(token) = self.tokens.peek().cloned() {
            if matches!(token.token, Token::KeywordEnd) {
                break;
            }

            match self.parse_statement() {
                Ok(stmt) => body.push(stmt),
                Err(e) => return Err(e),
            }
        }

        let before_count = self.tokens.clone().count();

        if let Some(token) = self.tokens.peek() {
            if matches!(token.token, Token::KeywordEnd) {
                self.tokens.next(); // Consume "end"

                if let Some(token) = self.tokens.peek() {
                    if matches!(token.token, Token::KeywordAction) {
                        self.tokens.next(); // Consume "action"
                    } else {
                        return Err(ParseError::new(
                            "Expected 'action' after 'end'".to_string(),
                            token.line,
                            token.column,
                        ));
                    }
                } else {
                    return Err(ParseError::new(
                        "Expected 'action' after 'end'".to_string(),
                        0,
                        0,
                    ));
                }
            } else {
                return Err(ParseError::new(
                    "Expected 'end' after action body".to_string(),
                    token.line,
                    token.column,
                ));
            }
        } else {
            return Err(ParseError::new(
                "Expected 'end' after action body".to_string(),
                0,
                0,
            ));
        }

        let after_count = self.tokens.clone().count();
        assert!(
            after_count < before_count,
            "Parser made no progress while parsing end action tokens"
        );

        // Add the action name to our known actions
        self.known_actions.insert(name.clone());

        let token_pos = self.tokens.peek().map_or(
            &TokenWithPosition {
                token: Token::KeywordDefine,
                line: 0,
                column: 0,
                length: 0,
            },
            |v| v,
        );
        Ok(Statement::ActionDefinition {
            name,
            parameters,
            body,
            return_type,
            line: token_pos.line,
            column: token_pos.column,
        })
    }

    fn parse_assignment(&mut self) -> Result<Statement, ParseError> {
        self.tokens.next(); // Consume "change"

        let mut name = String::new();
        let mut has_identifier = false;

        while let Some(token) = self.tokens.peek().cloned() {
            if let Token::Identifier(id) = &token.token {
                has_identifier = true;
                if !name.is_empty() {
                    name.push(' ');
                }
                name.push_str(id);
                self.tokens.next();
            } else if let Token::KeywordTo = &token.token {
                break;
            } else {
                // Provide a more specific error message if we've seen at least one identifier
                if has_identifier {
                    return Err(ParseError::new(
                        format!(
                            "Expected 'to' after identifier(s), but found {:?}",
                            token.token
                        ),
                        token.line,
                        token.column,
                    ));
                } else {
                    return Err(ParseError::new(
                        format!("Expected identifier or 'to', found {:?}", token.token),
                        token.line,
                        token.column,
                    ));
                }
            }
        }

        self.expect_token(
            Token::KeywordTo,
            "Expected 'to' after variable name in change statement",
        )?;

        let value = self.parse_expression()?;

        let token_pos = self.tokens.peek().map_or(
            &TokenWithPosition {
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

    fn parse_return_statement(&mut self) -> Result<Statement, ParseError> {
        let return_token = self.tokens.next().unwrap(); // Consume "give" or "return"

        if matches!(return_token.token, Token::KeywordGive) {
            self.expect_token(Token::KeywordBack, "Expected 'back' after 'give'")?;
        }

        let value = if let Some(token) = self.tokens.peek().cloned() {
            if matches!(token.token, Token::NothingLiteral) {
                self.tokens.next(); // Consume "nothing"
                None
            } else {
                Some(self.parse_expression()?)
            }
        } else {
            None
        };

        Ok(Statement::ReturnStatement {
            value,
            line: return_token.line,
            column: return_token.column,
        })
    }

    fn parse_open_file_statement(&mut self) -> Result<Statement, ParseError> {
        let open_token = self.tokens.next().unwrap(); // Consume "open"

        // Check if the next token is "file" or "url"
        if let Some(next_token) = self.tokens.peek() {
            match next_token.token {
                Token::KeywordFile => {
                    // Existing file handling
                    self.tokens.next(); // Consume "file"
                }
                Token::KeywordUrl => {
                    // New URL handling
                    self.tokens.next(); // Consume "url"

                    // Continue with URL-specific parsing
                    if let Some(token) = self.tokens.peek().cloned() {
                        if token.token == Token::KeywordAt {
                            self.tokens.next(); // Consume "at"

                            let url_expr = if let Some(token) = self.tokens.peek().cloned() {
                                if let Token::StringLiteral(url_str) = &token.token {
                                    let token_clone = token;
                                    self.tokens.next(); // Consume the string literal
                                    Expression::Literal(
                                        Literal::String(url_str.clone()),
                                        token_clone.line,
                                        token_clone.column,
                                    )
                                } else {
                                    return Err(ParseError::new(
                                        format!(
                                            "Expected string literal for URL, found {:?}",
                                            token.token
                                        ),
                                        token.line,
                                        token.column,
                                    ));
                                }
                            } else {
                                return Err(ParseError::new(
                                    "Unexpected end of input".to_string(),
                                    0,
                                    0,
                                ));
                            };

                            // Check for "and read content as" pattern
                            if let Some(next_token) = self.tokens.peek().cloned() {
                                if next_token.token == Token::KeywordAnd {
                                    self.tokens.next(); // Consume "and"
                                    self.expect_token(
                                        Token::KeywordRead,
                                        "Expected 'read' after 'and'",
                                    )?;
                                    self.expect_token(
                                        Token::KeywordContent,
                                        "Expected 'content' after 'read'",
                                    )?;
                                    self.expect_token(
                                        Token::KeywordAs,
                                        "Expected 'as' after 'content'",
                                    )?;

                                    let variable_name = if let Some(token) =
                                        self.tokens.peek().cloned()
                                    {
                                        if let Token::Identifier(name) = &token.token {
                                            self.tokens.next(); // Consume the identifier
                                            name.clone()
                                        } else {
                                            return Err(ParseError::new(
                                                format!(
                                                    "Expected identifier for variable name, found {:?}",
                                                    token.token
                                                ),
                                                token.line,
                                                token.column,
                                            ));
                                        }
                                    } else {
                                        return Err(ParseError::new(
                                            "Unexpected end of input".to_string(),
                                            0,
                                            0,
                                        ));
                                    };

                                    // Use HttpGetStatement for URL handling
                                    return Ok(Statement::HttpGetStatement {
                                        url: url_expr,
                                        variable_name,
                                        line: open_token.line,
                                        column: open_token.column,
                                    });
                                } else if next_token.token == Token::KeywordAs {
                                    // Handle "open url at "..." as variable" syntax
                                    self.tokens.next(); // Consume "as"

                                    let variable_name = if let Some(token) =
                                        self.tokens.peek().cloned()
                                    {
                                        if let Token::Identifier(name) = &token.token {
                                            self.tokens.next(); // Consume the identifier
                                            name.clone()
                                        } else {
                                            return Err(ParseError::new(
                                                format!(
                                                    "Expected identifier for variable name, found {:?}",
                                                    token.token
                                                ),
                                                token.line,
                                                token.column,
                                            ));
                                        }
                                    } else {
                                        return Err(ParseError::new(
                                            "Unexpected end of input".to_string(),
                                            0,
                                            0,
                                        ));
                                    };

                                    // Use HttpGetStatement for URL handling with direct "as" syntax
                                    return Ok(Statement::HttpGetStatement {
                                        url: url_expr,
                                        variable_name,
                                        line: open_token.line,
                                        column: open_token.column,
                                    });
                                } else {
                                    return Err(ParseError::new(
                                        format!(
                                            "Expected 'and' or 'as' after URL, found {:?}",
                                            next_token.token
                                        ),
                                        next_token.line,
                                        next_token.column,
                                    ));
                                }
                            }
                        }
                    }

                    return Err(ParseError::new(
                        "Expected 'at' after 'url'".to_string(),
                        open_token.line,
                        open_token.column + 5, // Approximate position after "open url"
                    ));
                }
                _ => {
                    return Err(ParseError::new(
                        format!(
                            "Expected 'file' or 'url' after 'open', found {:?}",
                            next_token.token
                        ),
                        next_token.line,
                        next_token.column,
                    ));
                }
            }
        } else {
            return Err(ParseError::new(
                "Unexpected end of input after 'open'".to_string(),
                open_token.line,
                open_token.column + 4, // Approximate position after "open"
            ));
        }

        if let Some(token) = self.tokens.peek().cloned() {
            if token.token == Token::KeywordAt {
                self.tokens.next(); // Consume "at"

                let path_expr = if let Some(token) = self.tokens.peek().cloned() {
                    if let Token::StringLiteral(path_str) = &token.token {
                        let token_clone = token;
                        self.tokens.next(); // Consume the string literal
                        Expression::Literal(
                            Literal::String(path_str.clone()),
                            token_clone.line,
                            token_clone.column,
                        )
                    } else {
                        return Err(ParseError::new(
                            format!(
                                "Expected string literal for file path, found {:?}",
                                token.token
                            ),
                            token.line,
                            token.column,
                        ));
                    }
                } else {
                    return Err(ParseError::new("Unexpected end of input".to_string(), 0, 0));
                };

                // Check for both "and read content as" pattern AND direct "as" pattern
                if let Some(next_token) = self.tokens.peek().cloned() {
                    if next_token.token == Token::KeywordAnd {
                        // Original pattern: "open file at "path" and read content as variable"
                        self.tokens.next(); // Consume "and"
                        self.expect_token(Token::KeywordRead, "Expected 'read' after 'and'")?;
                        self.expect_token(
                            Token::KeywordContent,
                            "Expected 'content' after 'read'",
                        )?;
                        self.expect_token(Token::KeywordAs, "Expected 'as' after 'content'")?;

                        let variable_name = if let Some(token) = self.tokens.peek().cloned() {
                            if let Token::Identifier(name) = &token.token {
                                self.tokens.next(); // Consume the identifier
                                name.clone()
                            } else if let Token::KeywordContent = &token.token {
                                // Special case for "content" as an identifier
                                self.tokens.next(); // Consume the "content" keyword
                                "content".to_string()
                            } else {
                                return Err(ParseError::new(
                                    format!(
                                        "Expected identifier for variable name, found {:?}",
                                        token.token
                                    ),
                                    token.line,
                                    token.column,
                                ));
                            }
                        } else {
                            return Err(ParseError::new(
                                "Unexpected end of input".to_string(),
                                0,
                                0,
                            ));
                        };

                        return Ok(Statement::ReadFileStatement {
                            path: path_expr,
                            variable_name,
                            line: open_token.line,
                            column: open_token.column,
                        });
                    } else if next_token.token == Token::KeywordAs {
                        // NEW pattern: "open file at "path" as variable"
                        self.tokens.next(); // Consume "as"

                        let variable_name = if let Some(token) = self.tokens.peek().cloned() {
                            if let Token::Identifier(id) = &token.token {
                                self.tokens.next();
                                id.clone()
                            } else {
                                return Err(ParseError::new(
                                    format!(
                                        "Expected identifier after 'as', found {:?}",
                                        token.token
                                    ),
                                    token.line,
                                    token.column,
                                ));
                            }
                        } else {
                            return Err(ParseError::new(
                                "Unexpected end of input after 'as'".to_string(),
                                0,
                                0,
                            ));
                        };

                        return Ok(Statement::OpenFileStatement {
                            path: path_expr,
                            variable_name,
                            line: open_token.line,
                            column: open_token.column,
                        });
                    } else {
                        return Err(ParseError::new(
                            format!(
                                "Expected 'and' or 'as' after file path, found {:?}",
                                next_token.token
                            ),
                            next_token.line,
                            next_token.column,
                        ));
                    }
                } else {
                    return Err(ParseError::new(
                        "Unexpected end of input after file path".to_string(),
                        0,
                        0,
                    ));
                }
            }
        }

        let path = self.parse_expression()?;

        self.expect_token(Token::KeywordAs, "Expected 'as' after file path")?;

        let variable_name = if let Some(token) = self.tokens.peek().cloned() {
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
            return Err(ParseError::new(
                "Unexpected end of input after 'as'".to_string(),
                0,
                0,
            ));
        };

        Ok(Statement::OpenFileStatement {
            path,
            variable_name,
            line: open_token.line,
            column: open_token.column,
        })
    }

    fn parse_open_file_read_statement(&mut self) -> Result<Statement, ParseError> {
        let open_token = self.tokens.next().unwrap(); // Consume "open"

        self.expect_token(Token::KeywordFile, "Expected 'file' after 'open'")?;
        self.expect_token(Token::KeywordAt, "Expected 'at' after 'file'")?;

        let path_expr = if let Some(token) = self.tokens.peek().cloned() {
            if let Token::StringLiteral(path_str) = &token.token {
                let token_clone = token;
                self.tokens.next(); // Consume the string literal
                Expression::Literal(
                    Literal::String(path_str.clone()),
                    token_clone.line,
                    token_clone.column,
                )
            } else {
                return Err(ParseError::new(
                    format!(
                        "Expected string literal for file path, found {:?}",
                        token.token
                    ),
                    token.line,
                    token.column,
                ));
            }
        } else {
            return Err(ParseError::new("Unexpected end of input".to_string(), 0, 0));
        };

        self.expect_token(Token::KeywordAnd, "Expected 'and' after file path")?;
        self.expect_token(Token::KeywordRead, "Expected 'read' after 'and'")?;
        self.expect_token(Token::KeywordContent, "Expected 'content' after 'read'")?;
        self.expect_token(Token::KeywordAs, "Expected 'as' after 'content'")?;

        let variable_name = if let Some(token) = self.tokens.peek().cloned() {
            if let Token::Identifier(name) = &token.token {
                self.tokens.next(); // Consume the identifier
                name.clone()
            } else if let Token::KeywordContent = &token.token {
                self.tokens.next(); // Consume the "content" keyword
                "content".to_string()
            } else {
                return Err(ParseError::new(
                    format!(
                        "Expected identifier for variable name, found {:?}",
                        token.token
                    ),
                    token.line,
                    token.column,
                ));
            }
        } else {
            return Err(ParseError::new("Unexpected end of input".to_string(), 0, 0));
        };

        Ok(Statement::ReadFileStatement {
            path: path_expr,
            variable_name,
            line: open_token.line,
            column: open_token.column,
        })
    }

    fn parse_wait_for_statement(&mut self) -> Result<Statement, ParseError> {
        let wait_token_pos = self.tokens.peek().map_or(
            &TokenWithPosition {
                token: Token::KeywordWait,
                line: 0,
                column: 0,
                length: 0,
            },
            |v| v,
        );

        self.tokens.next(); // Consume "wait"
        self.expect_token(Token::KeywordFor, "Expected 'for' after 'wait'")?;

        // Check for write mode (append or write)
        let write_mode = if let Some(token) = self.tokens.peek() {
            match &token.token {
                Token::KeywordAppend => {
                    self.tokens.next(); // Consume "append"
                    crate::parser::ast::WriteMode::Append
                }
                Token::KeywordWrite => {
                    self.tokens.next(); // Consume "write"
                    crate::parser::ast::WriteMode::Overwrite
                }
                Token::Identifier(id) if id == "write" => {
                    self.tokens.next(); // Consume "write" identifier
                    crate::parser::ast::WriteMode::Overwrite
                }
                _ => {
                    let inner = Box::new(self.parse_statement()?);
                    return Ok(Statement::WaitForStatement {
                        inner,
                        line: wait_token_pos.line,
                        column: wait_token_pos.column,
                    });
                }
            }
        } else {
            return Err(ParseError::new("Unexpected end of input".to_string(), 0, 0));
        };

        if let Some(token) = self.tokens.peek() {
            // Check for "content" keyword
            if matches!(token.token, Token::KeywordContent)
                || matches!(token.token, Token::Identifier(ref id) if id == "content")
            {
                self.tokens.next(); // Consume "content"

                let content = self.parse_expression()?;

                self.expect_token(
                    Token::KeywordInto,
                    "Expected 'into' after content expression",
                )?;

                let file = self.parse_expression()?;

                let write_stmt = Statement::WriteFileStatement {
                    file,
                    content,
                    mode: write_mode,
                    line: wait_token_pos.line,
                    column: wait_token_pos.column,
                };

                return Ok(Statement::WaitForStatement {
                    inner: Box::new(write_stmt),
                    line: wait_token_pos.line,
                    column: wait_token_pos.column,
                });
            }
        }

        Err(ParseError::new(
            "Expected 'content' after 'write' or 'append'".to_string(),
            wait_token_pos.line,
            wait_token_pos.column,
        ))
    }

    fn parse_expression_statement(&mut self) -> Result<Statement, ParseError> {
        let expr = self.parse_expression()?;

        let default_token = TokenWithPosition {
            token: Token::Identifier("expression".to_string()),
            line: 0,
            column: 0,
            length: 0,
        };
        let token_pos = self.tokens.peek().map_or(&default_token, |v| v);
        Ok(Statement::ExpressionStatement {
            expression: expr,
            line: token_pos.line,
            column: token_pos.column,
        })
    }

    fn parse_close_file_statement(&mut self) -> Result<Statement, ParseError> {
        let token_pos = self.tokens.next().unwrap(); // Consume "close"
        self.expect_token(Token::KeywordFile, "Expected 'file' after 'close'")?;

        let file = self.parse_expression()?;

        Ok(Statement::CloseFileStatement {
            file,
            line: token_pos.line,
            column: token_pos.column,
        })
    }

    fn parse_argument_list(&mut self) -> Result<Vec<Argument>, ParseError> {
        let mut arguments = Vec::with_capacity(4);

        let before_count = self.tokens.clone().count();

        loop {
            // Check for named arguments (name: value)
            let arg_name = if let Some(name_token) = self.tokens.peek().cloned() {
                if let Token::Identifier(id) = &name_token.token {
                    if let Some(next) = self.tokens.clone().nth(1) {
                        if matches!(next.token, Token::Colon) {
                            self.tokens.next(); // Consume name
                            self.tokens.next(); // Consume ":"
                            Some(id.to_string())
                        } else {
                            None
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

            let arg_value = self.parse_primary_expression()?;

            arguments.push(Argument {
                name: arg_name,
                value: arg_value,
            });

            if let Some(token) = self.tokens.peek().cloned() {
                if matches!(token.token, Token::KeywordAnd) {
                    self.tokens.next(); // Consume "and"
                    continue; // Continue parsing next argument
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        let after_count = self.tokens.clone().count();
        assert!(
            after_count < before_count,
            "Parser made no progress while parsing argument list"
        );

        Ok(arguments)
    }

    fn parse_try_statement(&mut self) -> Result<Statement, ParseError> {
        let try_token = self.tokens.next().unwrap(); // Consume "try"
        self.expect_token(Token::Colon, "Expected ':' after 'try'")?;

        let mut body = Vec::new();
        while let Some(token) = self.tokens.peek().cloned() {
            if matches!(token.token, Token::KeywordWhen | Token::KeywordEnd) {
                break;
            }
            body.push(self.parse_statement()?);
        }

        self.expect_token(Token::KeywordWhen, "Expected 'when' after try block")?;
        self.expect_token(Token::KeywordError, "Expected 'error' after 'when'")?;
        self.expect_token(Token::Colon, "Expected ':' after 'when error'")?;

        let mut when_block = Vec::new();
        while let Some(token) = self.tokens.peek().cloned() {
            if matches!(token.token, Token::KeywordEnd) {
                break;
            }
            when_block.push(self.parse_statement()?);
        }

        self.expect_token(Token::KeywordEnd, "Expected 'end' after when block")?;
        self.expect_token(Token::KeywordTry, "Expected 'try' after 'end'")?;

        Ok(Statement::TryStatement {
            body,
            error_name: "error".to_string(), // Default error name
            when_block,
            otherwise_block: None,
            line: try_token.line,
            column: try_token.column,
        })
    }

    fn parse_repeat_statement(&mut self) -> Result<Statement, ParseError> {
        let repeat_token = self.tokens.next().unwrap(); // Consume "repeat"

        if let Some(token) = self.tokens.peek().cloned() {
            match token.token {
                Token::KeywordWhile => {
                    self.tokens.next(); // Consume "while"
                    let condition = self.parse_expression()?;
                    if let Some(token) = self.tokens.peek() {
                        if matches!(token.token, Token::Colon) {
                            self.tokens.next(); // Consume the colon if present
                        }
                    }

                    let mut body = Vec::new();
                    while let Some(token) = self.tokens.peek().cloned() {
                        if matches!(token.token, Token::KeywordEnd) {
                            break;
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
                    self.tokens.next(); // Consume "until"
                    let condition = self.parse_expression()?;
                    if let Some(token) = self.tokens.peek() {
                        if matches!(token.token, Token::Colon) {
                            self.tokens.next(); // Consume the colon if present
                        }
                    }

                    let mut body = Vec::new();
                    while let Some(token) = self.tokens.peek().cloned() {
                        if matches!(token.token, Token::KeywordEnd) {
                            break;
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
                    self.tokens.next(); // Consume "forever"
                    self.expect_token(Token::Colon, "Expected ':' after 'forever'")?;

                    let mut body = Vec::new();
                    while let Some(token) = self.tokens.peek().cloned() {
                        if matches!(token.token, Token::KeywordEnd) {
                            break;
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
                    self.tokens.next(); // Consume ":"

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

    fn parse_exit_statement(&mut self) -> Result<Statement, ParseError> {
        let exit_token = self.tokens.next().unwrap(); // Consume "exit"

        // Check for "loop" after "exit"
        if let Some(token) = self.tokens.peek().cloned() {
            if let Token::Identifier(id) = &token.token {
                if id.to_lowercase() == "loop" {
                    self.tokens.next(); // Consume "loop"
                }
            }
        }

        Ok(Statement::ExitStatement {
            line: exit_token.line,
            column: exit_token.column,
        })
    }

    fn parse_push_statement(&mut self) -> Result<Statement, ParseError> {
        let push_token = self.tokens.next().unwrap(); // Consume "push"

        self.expect_token(Token::KeywordWith, "Expected 'with' after 'push'")?;

        // Parse the list expression but limit it to just the primary expression
        let list_expr = self.parse_primary_expression()?;

        self.expect_token(Token::KeywordAnd, "Expected 'and' after list expression")?;

        let start_line = if let Some(token) = self.tokens.peek() {
            token.line
        } else {
            push_token.line
        };

        let mut value_expr = self.parse_primary_expression()?;

        if let Some(token) = self.tokens.peek() {
            if token.line == start_line && !Parser::is_statement_starter(&token.token) {
                // so we can continue parsing the expression
                value_expr = self.parse_binary_expression(0)?;
            }
        }

        let stmt = Statement::PushStatement {
            list: list_expr,
            value: value_expr,
            line: push_token.line,
            column: push_token.column,
        };

        Ok(stmt)
    }

    fn parse_container_action_definition(&mut self) -> Result<Statement, ParseError> {
        self.tokens.next(); // Consume "action"

        let name = if let Some(token) = self.tokens.peek() {
            if let Token::Identifier(id) = &token.token {
                self.tokens.next();
                id.clone()
            } else {
                return Err(ParseError::new(
                    format!(
                        "Expected identifier after 'action', found {:?}",
                        token.token
                    ),
                    token.line,
                    token.column,
                ));
            }
        } else {
            return Err(ParseError::new(
                "Expected identifier after 'action'".to_string(),
                0,
                0,
            ));
        };

        let mut parameters = Vec::new();

        // Check for parameters
        if let Some(token) = self.tokens.peek().cloned() {
            if matches!(token.token, Token::KeywordNeeds) {
                self.tokens.next(); // Consume "needs"
                parameters = self.parse_parameter_list()?;
            }
        }

        // For now, container actions don't support explicit return types
        let return_type = None;

        self.expect_token(Token::Colon, "Expected ':' after action declaration")?;

        let mut body = Vec::new();

        // Parse action body until 'end'
        loop {
            if let Some(token) = self.tokens.peek() {
                if token.token == Token::KeywordEnd {
                    self.tokens.next(); // Consume 'end'
                    break;
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

        Ok(Statement::ActionDefinition {
            name,
            parameters,
            body,
            return_type,
            line: 0,
            column: 0,
        })
    }

    fn parse_create_pattern_statement(&mut self) -> Result<Statement, ParseError> {
        let create_token = self.tokens.next().unwrap(); // Consume "create"
        self.expect_token(Token::KeywordPattern, "Expected 'pattern' after 'create'")?;

        let pattern_name = if let Some(token) = self.tokens.next() {
            if let Token::Identifier(name) = &token.token {
                name.clone()
            } else {
                return Err(ParseError::new(
                    "Expected pattern name after 'create pattern'".to_string(),
                    token.line,
                    token.column,
                ));
            }
        } else {
            return Err(ParseError::new(
                "Expected pattern name after 'create pattern'".to_string(),
                create_token.line,
                create_token.column,
            ));
        };

        self.expect_token(Token::Colon, "Expected ':' after pattern name")?;

        let mut pattern_parts = Vec::new();
        let mut depth = 1; // Track nesting depth for proper end matching

        while let Some(token) = self.tokens.peek() {
            match &token.token {
                Token::KeywordEnd => {
                    let mut tokens_clone = self.tokens.clone();
                    tokens_clone.next(); // Skip "end"
                    if let Some(next_token) = tokens_clone.next() {
                        if next_token.token == Token::KeywordPattern {
                            depth -= 1;
                            if depth == 0 {
                                self.tokens.next(); // Consume "end"
                                self.tokens.next(); // Consume "pattern"
                                break;
                            }
                        }
                    }
                    pattern_parts.push(self.tokens.next().unwrap().clone());
                }
                Token::KeywordCreate => {
                    // Check for nested pattern creation
                    let mut tokens_clone = self.tokens.clone();
                    tokens_clone.next(); // Skip "create"
                    if let Some(next_token) = tokens_clone.next() {
                        if next_token.token == Token::KeywordPattern {
                            depth += 1;
                        }
                    }
                    pattern_parts.push(self.tokens.next().unwrap().clone());
                }
                _ => {
                    pattern_parts.push(self.tokens.next().unwrap().clone());
                }
            }
        }

        if depth > 0 {
            return Err(ParseError::new(
                "Expected 'end pattern' to close pattern definition".to_string(),
                create_token.line,
                create_token.column,
            ));
        }

        let ir_string = Self::compile_pattern_to_ir(&pattern_parts)?;

        Ok(Statement::VariableDeclaration {
            name: pattern_name,
            value: Expression::Literal(
                Literal::Pattern(ir_string),
                create_token.line,
                create_token.column,
            ),
            line: create_token.line,
            column: create_token.column,
        })
    }

    fn compile_pattern_to_ir(tokens: &[TokenWithPosition]) -> Result<String, ParseError> {
        let mut i = 0;
        let sequence_parts = Self::parse_sequence(tokens, &mut i)?;

        if sequence_parts.is_empty() {
            return Err(ParseError::new(
                "Empty pattern definition".to_string(),
                0,
                0,
            ));
        }

        if sequence_parts.len() == 1 {
            Ok(sequence_parts[0].clone())
        } else {
            Ok(format!("seq({})", sequence_parts.join(",")))
        }
    }

    fn parse_sequence(
        tokens: &[TokenWithPosition],
        i: &mut usize,
    ) -> Result<Vec<String>, ParseError> {
        let mut sequence_parts = Vec::new();

        while *i < tokens.len() {
            let token = &tokens[*i];
            match &token.token {
                Token::Newline => {
                    *i += 1;
                    continue;
                }
                _ => {
                    let element = Self::parse_element(tokens, i)?;
                    sequence_parts.push(element);
                }
            }
        }

        Ok(sequence_parts)
    }

    fn parse_element(tokens: &[TokenWithPosition], i: &mut usize) -> Result<String, ParseError> {
        if *i >= tokens.len() {
            return Err(ParseError::new(
                "Expected pattern element".to_string(),
                0,
                0,
            ));
        }

        let token = &tokens[*i];
        match &token.token {
            // Handle quantifiers first
            Token::KeywordOne => {
                *i += 1;
                if *i < tokens.len() && tokens[*i].token == Token::KeywordOr {
                    *i += 1;
                    if *i < tokens.len() && tokens[*i].token == Token::KeywordMore {
                        *i += 1;
                        let inner = Self::parse_quantified_content(tokens, i)?;
                        Ok(format!("rep(1,inf,{inner})"))
                    } else {
                        Err(ParseError::new(
                            "Expected 'more' after 'one or'".to_string(),
                            token.line,
                            token.column,
                        ))
                    }
                } else {
                    Err(ParseError::new(
                        "Expected 'or' after 'one'".to_string(),
                        token.line,
                        token.column,
                    ))
                }
            }

            Token::KeywordOptional => {
                *i += 1;
                let inner = Self::parse_quantified_content(tokens, i)?;
                Ok(format!("rep(0,1,{inner})"))
            }

            Token::KeywordBetween => {
                *i += 1;
                if *i + 3 < tokens.len() {
                    if let Token::IntLiteral(min) = &tokens[*i].token {
                        let min_val = *min;
                        *i += 1;
                        if tokens[*i].token == Token::KeywordAnd {
                            *i += 1;
                            if let Token::IntLiteral(max) = &tokens[*i].token {
                                let max_val = *max;
                                *i += 1;
                                let inner = Self::parse_quantified_content(tokens, i)?;
                                Ok(format!("rep({min_val},{max_val},{inner})"))
                            } else {
                                Err(ParseError::new(
                                    "Expected number after 'and'".to_string(),
                                    token.line,
                                    token.column,
                                ))
                            }
                        } else {
                            Err(ParseError::new(
                                "Expected 'and' after first number".to_string(),
                                token.line,
                                token.column,
                            ))
                        }
                    } else {
                        Err(ParseError::new(
                            "Expected number after 'between'".to_string(),
                            token.line,
                            token.column,
                        ))
                    }
                } else {
                    Err(ParseError::new(
                        "Incomplete 'between' quantifier".to_string(),
                        token.line,
                        token.column,
                    ))
                }
            }

            // Handle captures
            Token::KeywordCapture => {
                *i += 1;
                if *i < tokens.len() && tokens[*i].token == Token::LeftBrace {
                    *i += 1;
                    let mut capture_tokens = Vec::new();
                    let mut brace_count = 1;

                    while *i < tokens.len() && brace_count > 0 {
                        match &tokens[*i].token {
                            Token::LeftBrace => brace_count += 1,
                            Token::RightBrace => brace_count -= 1,
                            _ => {}
                        }
                        if brace_count > 0 {
                            capture_tokens.push(tokens[*i].clone());
                        }
                        *i += 1;
                    }

                    if *i + 1 < tokens.len() && tokens[*i].token == Token::KeywordAs {
                        *i += 1;
                        if let Token::Identifier(name) = &tokens[*i].token {
                            let capture_name = name.clone();
                            *i += 1;
                            let inner_ir = Self::compile_pattern_to_ir(&capture_tokens)?;
                            Ok(format!("cap(\"{capture_name}\",{inner_ir})"))
                        } else {
                            Err(ParseError::new(
                                "Expected identifier after 'as'".to_string(),
                                token.line,
                                token.column,
                            ))
                        }
                    } else {
                        Err(ParseError::new(
                            "Expected 'as' after capture group".to_string(),
                            token.line,
                            token.column,
                        ))
                    }
                } else {
                    Err(ParseError::new(
                        "Expected '{' after 'capture'".to_string(),
                        token.line,
                        token.column,
                    ))
                }
            }

            // Handle anchors
            Token::KeywordAt => {
                *i += 1;
                if *i < tokens.len() && tokens[*i].token == Token::KeywordStart {
                    *i += 1;
                    if *i < tokens.len() && tokens[*i].token == Token::KeywordOf {
                        *i += 1;
                        if *i < tokens.len() && tokens[*i].token == Token::KeywordText {
                            *i += 1;
                            Ok("anchor(start)".to_string())
                        } else {
                            Err(ParseError::new(
                                "Expected 'text' after 'of'".to_string(),
                                token.line,
                                token.column,
                            ))
                        }
                    } else {
                        Err(ParseError::new(
                            "Expected 'of' after 'start'".to_string(),
                            token.line,
                            token.column,
                        ))
                    }
                } else if *i < tokens.len() && tokens[*i].token == Token::KeywordEnd {
                    *i += 1;
                    if *i < tokens.len() && tokens[*i].token == Token::KeywordOf {
                        *i += 1;
                        if *i < tokens.len() && tokens[*i].token == Token::KeywordText {
                            *i += 1;
                            Ok("anchor(end)".to_string())
                        } else {
                            Err(ParseError::new(
                                "Expected 'text' after 'of'".to_string(),
                                token.line,
                                token.column,
                            ))
                        }
                    } else {
                        Err(ParseError::new(
                            "Expected 'of' after 'end'".to_string(),
                            token.line,
                            token.column,
                        ))
                    }
                } else {
                    Err(ParseError::new(
                        "Expected 'start' or 'end' after 'at'".to_string(),
                        token.line,
                        token.column,
                    ))
                }
            }

            // Handle basic elements
            Token::StringLiteral(s) => {
                *i += 1;
                Ok(format!("lit(\"{}\")", s.replace("\"", "\\\"")))
            }
            Token::KeywordDigit => {
                *i += 1;
                Ok("class(digit)".to_string())
            }
            Token::KeywordLetter => {
                *i += 1;
                Ok("class(letter)".to_string())
            }
            Token::KeywordWhitespace => {
                *i += 1;
                Ok("class(whitespace)".to_string())
            }

            _ => Err(ParseError::new(
                "Invalid pattern element".to_string(),
                token.line,
                token.column,
            )),
        }
    }

    fn parse_quantified_content(
        tokens: &[TokenWithPosition],
        i: &mut usize,
    ) -> Result<String, ParseError> {
        let mut alternatives = Vec::new();

        loop {
            if *i >= tokens.len() {
                break;
            }

            let token = &tokens[*i];
            match &token.token {
                Token::StringLiteral(s) => {
                    *i += 1;
                    alternatives.push(format!("lit(\"{}\")", s.replace("\"", "\\\"")));
                }
                Token::KeywordDigit => {
                    *i += 1;
                    alternatives.push("class(digit)".to_string());
                }
                Token::KeywordLetter => {
                    *i += 1;
                    alternatives.push("class(letter)".to_string());
                }
                Token::KeywordWhitespace => {
                    *i += 1;
                    alternatives.push("class(whitespace)".to_string());
                }
                Token::KeywordOr => {
                    *i += 1;
                    // Continue to next alternative
                    continue;
                }
                _ => {
                    break;
                }
            }

            // Check if there's an "or" following
            if *i < tokens.len() && tokens[*i].token == Token::KeywordOr {
                continue;
            } else {
                break; // End of alternatives
            }
        }

        if alternatives.is_empty() {
            return Err(ParseError::new(
                "Expected pattern element after quantifier".to_string(),
                0,
                0,
            ));
        }

        if alternatives.len() == 1 {
            Ok(alternatives[0].clone())
        } else {
            Ok(format!("alt({})", alternatives.join(",")))
        }
    }
}
