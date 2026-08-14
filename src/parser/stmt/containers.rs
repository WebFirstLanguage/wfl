//! Container (OOP) statement parsing

use super::super::{
    ActionSignature, Argument, EventDefinition, ParseError, Parser, PropertyDefinition,
    PropertyInitializer, Statement, Type, Visibility,
};
use super::actions::colon_type_from_token;
use super::{ActionParser, StmtParser};
use crate::lexer::token::Token;
use crate::parser::expr::{BinaryExprParser, ExprParser, PrimaryExprParser};

pub(crate) trait ContainerParser<'a>: ExprParser<'a> + ActionParser<'a> {
    fn parse_container_definition(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>;

    fn parse_interface_definition(&mut self) -> Result<Statement, ParseError>;
    fn parse_interface_body(
        &mut self,
        interface_name: &str,
        header_line: usize,
        header_column: usize,
    ) -> Result<Vec<ActionSignature>, ParseError>;
    fn parse_container_instantiation(&mut self) -> Result<Statement, ParseError>;
    fn parse_event_definition(&mut self) -> Result<Statement, ParseError>;
    fn parse_event_trigger(&mut self) -> Result<Statement, ParseError>;
    fn parse_event_handler(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>;
    fn parse_event_name(
        &mut self,
        at_token: &crate::lexer::token::TokenWithPosition,
    ) -> Result<String, ParseError>;

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

impl<'a> Parser<'a> {
    /// Parse the active colon-style property type grammar. `List` without an
    /// element annotation is explicitly dynamic; `List of T` retains `T`, and
    /// nesting is recursive. Other spellings keep the historical colon-style
    /// case rules in `colon_type_from_token`.
    fn parse_property_type_annotation(
        &mut self,
        property_token: &crate::lexer::token::TokenWithPosition,
    ) -> Result<Type, ParseError> {
        let Some(type_token) = self.cursor.peek().cloned() else {
            return Err(ParseError::from_token(
                "Expected type name after ':'".to_string(),
                property_token,
            ));
        };

        let is_list = matches!(type_token.token, Token::KeywordList)
            || matches!(&type_token.token, Token::Identifier(name) if name == "List");
        if is_list {
            self.bump_sync();
            if self
                .cursor
                .peek()
                .is_some_and(|token| token.token == Token::KeywordOf)
            {
                self.bump_sync();
                let element_type = self.parse_property_type_annotation(&type_token)?;
                return Ok(Type::List(Box::new(element_type)));
            }
            return Ok(Type::List(Box::new(Type::Any)));
        }

        if let Some(property_type) = colon_type_from_token(&type_token.token) {
            self.bump_sync();
            Ok(property_type)
        } else {
            Err(ParseError::from_token(
                "Expected type name after ':'".to_string(),
                &type_token,
            ))
        }
    }
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

        // Optional 'extends' with one or more parent interfaces
        let mut extends = Vec::new();
        if let Some(token) = self.cursor.peek()
            && token.token == Token::KeywordExtends
        {
            let extends_token = self.bump_sync().unwrap(); // Consume 'extends'
            loop {
                if let Some(token) = self.cursor.peek() {
                    if let Token::Identifier(id) = &token.token {
                        extends.push(id.clone());
                        self.bump_sync(); // Consume the identifier
                        if let Some(next_token) = self.cursor.peek()
                            && next_token.token == Token::Comma
                        {
                            self.bump_sync(); // Consume comma
                            continue;
                        }
                        break;
                    } else {
                        return Err(ParseError::from_token(
                            "Expected interface name after 'extends'".to_string(),
                            token,
                        ));
                    }
                } else {
                    return Err(ParseError::from_token(
                        "Expected interface name after 'extends'".to_string(),
                        extends_token,
                    ));
                }
            }
        }

        // A bare 'create interface Name' (no colon) stays valid as an empty
        // contract for backward compatibility. A colon opens a body of
        // 'requires action ...' signatures terminated by 'end'.
        let mut required_actions = Vec::new();
        if let Some(token) = self.cursor.peek()
            && token.token == Token::Colon
        {
            self.bump_sync(); // Consume ':'
            required_actions = self.parse_interface_body(&name, line, column)?;
        }

        Ok(Statement::InterfaceDefinition {
            name,
            extends,
            required_actions,
            line,
            column,
        })
    }

    fn parse_interface_body(
        &mut self,
        interface_name: &str,
        header_line: usize,
        header_column: usize,
    ) -> Result<Vec<ActionSignature>, ParseError> {
        let mut required_actions = Vec::new();
        let mut last_position = (header_line, header_column);

        loop {
            let Some(token) = self.cursor.peek() else {
                return Err(ParseError::from_span(
                    format!(
                        "Unexpected end of input in interface body: interface '{interface_name}' is missing 'end'"
                    ),
                    crate::diagnostics::Span { start: 0, end: 0 },
                    last_position.0,
                    last_position.1,
                ));
            };
            last_position = (token.line, token.column);

            match &token.token {
                Token::KeywordEnd => {
                    self.bump_sync(); // Consume 'end'
                    break;
                }
                Token::Eol => {
                    self.bump_sync(); // Skip Eol between requirements
                    continue;
                }
                Token::KeywordRequires => {
                    let requires_token = self.bump_sync().unwrap(); // Consume 'requires'
                    let line = requires_token.line;
                    let column = requires_token.column;

                    self.expect_token(
                        Token::KeywordAction,
                        "Expected 'action' after 'requires' in interface body",
                    )?;

                    let name = if let Some(token) = self.cursor.peek() {
                        if let Token::Identifier(id) = &token.token {
                            self.bump_sync(); // Consume the identifier
                            id.clone()
                        } else {
                            return Err(ParseError::from_token(
                                "Expected action name after 'requires action'".to_string(),
                                token,
                            ));
                        }
                    } else {
                        return Err(ParseError::from_token(
                            "Expected action name after 'requires action'".to_string(),
                            requires_token,
                        ));
                    };

                    let mut parameters = Vec::new();
                    if let Some(token) = self.cursor.peek()
                        && matches!(&token.token, Token::KeywordNeeds | Token::KeywordWith)
                    {
                        self.bump_sync(); // Consume 'needs' / 'with'
                        parameters = self.parse_parameter_list()?;
                    }

                    // Optional return type: 'requires action get_area: Number'
                    let return_type = if let Some(token) = self.cursor.peek()
                        && token.token == Token::Colon
                    {
                        self.bump_sync(); // Consume ':'
                        if let Some(type_token) = self.cursor.peek() {
                            if let Some(parsed) = colon_type_from_token(&type_token.token) {
                                self.bump_sync(); // Consume type name
                                Some(parsed)
                            } else {
                                return Err(ParseError::from_token(
                                    "Expected type name after ':' in interface action signature"
                                        .to_string(),
                                    type_token,
                                ));
                            }
                        } else {
                            return Err(ParseError::from_token(
                                "Expected type name after ':' in interface action signature"
                                    .to_string(),
                                requires_token,
                            ));
                        }
                    } else {
                        None
                    };

                    required_actions.push(ActionSignature {
                        name,
                        parameters,
                        return_type,
                        line,
                        column,
                    });
                }
                _ => {
                    return Err(ParseError::from_token(
                        format!(
                            "Unexpected token in interface body: {:?}. Interface bodies contain 'requires action <name>' signatures.",
                            token.token
                        ),
                        token,
                    ));
                }
            }
        }

        Ok(required_actions)
    }

    fn parse_container_instantiation(&mut self) -> Result<Statement, ParseError> {
        let start_token = self.bump_sync().unwrap(); // Consume 'create'
        let line = start_token.line;
        let column = start_token.column;

        self.expect_token(Token::KeywordNew, "Expected 'new' after 'create'")?;

        // Check for deprecated "create new constant" syntax
        if let Some(token) = self.cursor.peek()
            && matches!(&token.token, Token::KeywordConstant)
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

    /// Parses a statement-level `event <name> [needs <params>]` definition.
    ///
    /// Shares its grammar with the in-container form so an event declared at
    /// the top level accepts the same `needs` parameter list.
    fn parse_event_definition(&mut self) -> Result<Statement, ParseError> {
        let definition = self.parse_event_definition_full()?;

        Ok(Statement::EventDefinition {
            name: definition.name,
            parameters: definition.parameters,
            line: definition.line,
            column: definition.column,
        })
    }

    /// Parses `trigger <event> [with <arg> and <arg> ...]`.
    ///
    /// The `with` clause supplies positional values for the parameters the
    /// event declared with `needs`; handlers see them bound by name.
    fn parse_event_trigger(&mut self) -> Result<Statement, ParseError> {
        let start_token = self.bump_sync().unwrap(); // Consume 'trigger'
        let line = start_token.line;
        let column = start_token.column;

        let name = self.parse_event_name(start_token)?;

        let arguments = if self
            .cursor
            .peek()
            .is_some_and(|t| t.token == Token::KeywordWith)
        {
            self.bump_sync(); // Consume 'with'
            self.parse_argument_list()?
        } else {
            Vec::new()
        };

        Ok(Statement::EventTrigger {
            name,
            arguments,
            line,
            column,
        })
    }

    /// Parses an event handler block:
    ///
    /// ```text
    /// on <event> of <container instance>:
    ///     <body>
    /// end on
    /// ```
    ///
    /// The `of <source>` clause is optional; without it the handler attaches to
    /// the event of that name already in scope (a top-level `event` definition,
    /// or the container's own event inside one of its actions).
    ///
    /// The event name comes first because the lexer merges adjacent bare words
    /// into one identifier — `on btn on_click` would lex as a single
    /// `btn on_click` identifier, leaving no way to tell source from event.
    /// The `of` keyword is the separator that keeps the two apart.
    fn parse_event_handler(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>,
    {
        let start_token = self.bump_sync().unwrap(); // Consume 'on'
        let line = start_token.line;
        let column = start_token.column;

        let event_name = self.parse_event_name(start_token)?;

        let event_source = if self
            .cursor
            .peek()
            .is_some_and(|t| t.token == Token::KeywordOf)
        {
            self.bump_sync(); // Consume 'of'
            // Primary expression only: the binary-expression parser silently
            // swallows a trailing ':', which would eat the header's colon.
            Some(self.parse_primary_expression()?)
        } else {
            None
        };

        self.expect_token(
            Token::Colon,
            "Expected ':' after the event handler header (`on <event> of <instance>:`)",
        )?;

        // Parse the handler body until `end on`.
        self.skip_eol();
        let mut handler_body = Vec::new();
        loop {
            self.skip_eol();
            match self.cursor.peek() {
                Some(t) if t.token == Token::KeywordEnd => break,
                None => {
                    return Err(ParseError::from_token(
                        "Expected 'end on' to close the event handler".to_string(),
                        start_token,
                    ));
                }
                _ => handler_body.push(self.parse_statement()?),
            }
        }
        self.expect_token(
            Token::KeywordEnd,
            "Expected 'end' to close the event handler",
        )?;
        self.expect_token(
            Token::KeywordOn,
            "Expected 'on' after 'end' in the event handler",
        )?;

        Ok(Statement::EventHandler {
            event_source,
            event_name,
            handler_body,
            line,
            column,
        })
    }

    /// Consumes the identifier naming an event, reporting `at_token`'s position
    /// when input runs out first.
    fn parse_event_name(
        &mut self,
        at_token: &crate::lexer::token::TokenWithPosition,
    ) -> Result<String, ParseError> {
        if let Some(token) = self.cursor.peek() {
            if let Token::Identifier(id) = &token.token {
                let name = id.clone();
                self.bump_sync(); // Consume the identifier
                Ok(name)
            } else {
                Err(ParseError::from_token(
                    format!(
                        "Expected identifier for event name, found {:?}",
                        token.token
                    ),
                    token,
                ))
            }
        } else {
            Err(ParseError::from_token(
                "Expected identifier for event name, found end of input".to_string(),
                at_token,
            ))
        }
    }

    fn parse_inheritance(&mut self) -> Result<(Option<String>, Vec<String>), ParseError> {
        let mut extends = None;
        let mut implements = Vec::new();

        // Check for 'extends' keyword
        if let Some(token) = self.cursor.peek()
            && token.token == Token::KeywordExtends
        {
            let extends_pos = self.bump_sync().unwrap(); // Consume 'extends' and keep position

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
            } else {
                return Err(ParseError::from_token(
                    "Expected identifier after 'extends'".to_string(),
                    extends_pos,
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

                Some(self.parse_property_type_annotation(start_token)?)
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
