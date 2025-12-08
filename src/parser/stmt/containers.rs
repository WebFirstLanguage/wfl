//! Container (OOP) statement parsing

use super::super::{
    Argument, EventDefinition, ParseError, Parser, PropertyDefinition, PropertyInitializer,
    Statement, Type, Visibility,
};
use super::{ActionParser, StmtParser};
use crate::lexer::token::Token;
use crate::parser::expr::ExprParser;

pub(crate) trait ContainerParser<'a>: ExprParser<'a> + ActionParser<'a> {
    fn parse_container_definition(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>;

    fn parse_interface_definition(&mut self) -> Result<Statement, ParseError>;
    fn parse_container_instantiation(&mut self) -> Result<Statement, ParseError>;
    fn parse_event_definition(&mut self) -> Result<Statement, ParseError>;
    fn parse_event_trigger(&mut self) -> Result<Statement, ParseError>;
    fn parse_event_handler(&mut self) -> Result<Statement, ParseError>;

    fn parse_inheritance(&mut self) -> Result<(Option<String>, Vec<String>), ParseError>;

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
    >
    where
        Self: StmtParser<'a>;

    fn parse_property_definition(
        &mut self,
        is_static: bool,
    ) -> Result<PropertyDefinition, ParseError>;

    fn parse_event_definition_full(&mut self) -> Result<EventDefinition, ParseError>;

    fn parse_instantiation_body(
        &mut self,
    ) -> Result<(Vec<PropertyInitializer>, Vec<Argument>), ParseError>;
}

impl<'a> ContainerParser<'a> for Parser<'a> {
    fn parse_container_definition(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>,
    {
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
                return Err(ParseError::from_token(
                    format!(
                        "Expected identifier for container name, found {:?}",
                        token.token
                    ),
                    token,
                ));
            }
        } else {
            return Err(ParseError::from_token(
                "Expected identifier for container name, found end of input".to_string(),
                start_token,
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

    fn parse_interface_definition(&mut self) -> Result<Statement, ParseError> {
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
                return Err(ParseError::from_token(
                    format!(
                        "Expected identifier for interface name, found {:?}",
                        token.token
                    ),
                    token,
                ));
            }
        } else {
            return Err(ParseError::from_token(
                "Expected identifier for interface name, found end of input".to_string(),
                start_token,
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

    fn parse_container_instantiation(&mut self) -> Result<Statement, ParseError> {
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
                return Err(ParseError::from_token(
                    format!(
                        "Expected identifier for container type, found {:?}",
                        token.token
                    ),
                    token,
                ));
            }
        } else {
            return Err(ParseError::from_token(
                "Expected identifier for container type, found end of input".to_string(),
                start_token,
            ));
        };

        self.expect_token(Token::KeywordAs, "Expected 'as' after container type")?;

        // Parse instance name
        let instance_name = if let Some(token) = self.cursor.peek() {
            if let Token::Identifier(id) = &token.token {
                self.bump_sync(); // Consume the identifier
                id.clone()
            } else {
                return Err(ParseError::from_token(
                    format!(
                        "Expected identifier for instance name, found {:?}",
                        token.token
                    ),
                    token,
                ));
            }
        } else {
            return Err(ParseError::from_token(
                "Expected identifier for instance name, found end of input".to_string(),
                start_token,
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

    fn parse_event_definition(&mut self) -> Result<Statement, ParseError> {
        let start_token = self.bump_sync().unwrap(); // Consume 'event'
        let line = start_token.line;
        let column = start_token.column;

        // Parse event name
        let name = if let Some(token) = self.cursor.peek() {
            if let Token::Identifier(id) = &token.token {
                self.bump_sync(); // Consume the identifier
                id.clone()
            } else {
                return Err(ParseError::from_token(
                    format!(
                        "Expected identifier for event name, found {:?}",
                        token.token
                    ),
                    token,
                ));
            }
        } else {
            return Err(ParseError::from_token(
                "Expected identifier for event name, found end of input".to_string(),
                start_token,
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

    fn parse_event_trigger(&mut self) -> Result<Statement, ParseError> {
        let start_token = self.bump_sync().unwrap(); // Consume 'trigger'
        let line = start_token.line;
        let column = start_token.column;

        // Parse event name
        let name = if let Some(token) = self.cursor.peek() {
            if let Token::Identifier(id) = &token.token {
                self.bump_sync(); // Consume the identifier
                id.clone()
            } else {
                return Err(ParseError::from_token(
                    format!(
                        "Expected identifier for event name, found {:?}",
                        token.token
                    ),
                    token,
                ));
            }
        } else {
            return Err(ParseError::from_token(
                "Expected identifier for event name, found end of input".to_string(),
                start_token,
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

    fn parse_event_handler(&mut self) -> Result<Statement, ParseError> {
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
                return Err(ParseError::from_token(
                    format!(
                        "Expected identifier for event name, found {:?}",
                        token.token
                    ),
                    token,
                ));
            }
        } else {
            return Err(ParseError::from_token(
                "Expected identifier for event name, found end of input".to_string(),
                start_token,
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

    fn parse_inheritance(&mut self) -> Result<(Option<String>, Vec<String>), ParseError> {
        let mut extends = None;
        let mut implements = Vec::new();

        // Check for 'extends' keyword
        if let Some(token) = self.cursor.peek()
            && token.token == Token::KeywordExtends
        {
            self.bump_sync(); // Consume 'extends'

            let extends_token = self.cursor.peek().cloned();
            if let Some(token) = self.cursor.peek() {
                if let Token::Identifier(id) = &token.token {
                    extends = Some(id.clone());
                    self.bump_sync(); // Consume the identifier
                } else {
                    return Err(ParseError::from_token(
                        "Expected identifier after 'extends'".to_string(),
                        token,
                    ));
                }
            } else if let Some(ref ext_tok) = extends_token {
                return Err(ParseError::from_token(
                    "Expected identifier after 'extends'".to_string(),
                    ext_tok,
                ));
            } else {
                return Err(ParseError::from_span(
                    "Expected identifier after 'extends'".to_string(),
                    crate::diagnostics::Span { start: 0, end: 0 },
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
                        return Err(ParseError::from_token(
                            "Expected identifier in implements list".to_string(),
                            token,
                        ));
                    }
                } else {
                    return Err(ParseError::from_span(
                        "Expected identifier in implements list".to_string(),
                        crate::diagnostics::Span { start: 0, end: 0 },
                        0,
                        0,
                    ));
                }
            }
        }

        Ok((extends, implements))
    }

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
    >
    where
        Self: StmtParser<'a>,
    {
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
                                    return Err(ParseError::from_token(
                                        "Expected 'property' or 'action' after 'static'"
                                            .to_string(),
                                        next_token,
                                    ));
                                }
                            }
                        } else {
                            return Err(ParseError::from_token(
                                "Expected 'property' or 'action' after 'static'".to_string(),
                                static_token,
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
                        return Err(ParseError::from_token(
                            format!("Unexpected token in container body: {:?}", token.token),
                            token,
                        ));
                    }
                }
            } else {
                return Err(ParseError::from_span(
                    "Unexpected end of input in container body".to_string(),
                    crate::diagnostics::Span { start: 0, end: 0 },
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
                return Err(ParseError::from_token(
                    "Expected property name after 'property'".to_string(),
                    token,
                ));
            }
        } else {
            return Err(ParseError::from_token(
                "Expected property name after 'property'".to_string(),
                start_token,
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
                        return Err(ParseError::from_token(
                            "Expected type name after ':'".to_string(),
                            type_token,
                        ));
                    }
                } else {
                    return Err(ParseError::from_token(
                        "Expected type name after ':'".to_string(),
                        start_token,
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
                return Err(ParseError::from_token(
                    "Expected event name after 'event'".to_string(),
                    token,
                ));
            }
        } else {
            return Err(ParseError::from_token(
                "Expected event name after 'event'".to_string(),
                start_token,
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
                            return Err(ParseError::from_token(
                                "Expected 'is' or ':' after property name".to_string(),
                                next_token,
                            ));
                        }
                    } else {
                        return Err(ParseError::from_span(
                            "Expected 'is' or ':' after property name".to_string(),
                            crate::diagnostics::Span { start: 0, end: 0 },
                            prop_line,
                            prop_column,
                        ));
                    }
                }
                _ => {
                    return Err(ParseError::from_token(
                        format!("Unexpected token in instantiation body: {:?}", token.token),
                        token,
                    ));
                }
            }
        }

        Ok((property_initializers, arguments))
    }
}
