use super::Parser;
use super::ast::*;
use super::error::ParseError;
use crate::lexer::token::Token;

impl<'a> Parser<'a> {
    pub(crate) fn parse_container_definition(&mut self) -> Result<Statement, ParseError> {
        let start_token = self.tokens.next().unwrap();
        let line = start_token.line;
        let column = start_token.column;

        self.expect_token(
            Token::KeywordContainer,
            "Expected 'container' after 'create'",
        )?;

        let name = if let Some(token) = self.tokens.peek() {
            if let Token::Identifier(id) = &token.token {
                self.tokens.next();
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

        let (extends, implements) = self.parse_inheritance2()?;

        self.expect_token(Token::Colon, "Expected ':' after container name")?;

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

    pub(crate) fn parse_interface_definition(&mut self) -> Result<Statement, ParseError> {
        let start_token = self.tokens.next().unwrap();
        let line = start_token.line;
        let column = start_token.column;

        self.expect_token(
            Token::KeywordInterface,
            "Expected 'interface' after 'create'",
        )?;

        let name = if let Some(token) = self.tokens.peek() {
            if let Token::Identifier(id) = &token.token {
                self.tokens.next();
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

        Ok(Statement::InterfaceDefinition {
            name,
            extends: Vec::new(),
            required_actions: Vec::new(),
            line,
            column,
        })
    }

    pub(crate) fn parse_container_instantiation(&mut self) -> Result<Statement, ParseError> {
        let start_token = self.tokens.next().unwrap();
        let line = start_token.line;
        let column = start_token.column;

        self.expect_token(Token::KeywordNew, "Expected 'new' after 'create'")?;

        if let Some(token) = self.tokens.peek()
            && matches!(token.token, Token::KeywordConstant)
        {
            eprintln!(
                "Warning: 'create new constant' syntax is deprecated and will be removed in a future version. Please use 'store new constant' instead."
            );

            self.tokens.next();

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

        let container_type = if let Some(token) = self.tokens.peek() {
            if let Token::Identifier(id) = &token.token {
                self.tokens.next();
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

        let instance_name = if let Some(token) = self.tokens.peek() {
            if let Token::Identifier(id) = &token.token {
                self.tokens.next();
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

        let instantiation_body = self.parse_instantiation_body()?;

        Ok(Statement::ContainerInstantiation {
            container_type,
            instance_name,
            arguments: Vec::new(),
            property_initializers: instantiation_body,
            line,
            column,
        })
    }

    pub(crate) fn parse_event_definition(&mut self) -> Result<Statement, ParseError> {
        let start = self.tokens.next().unwrap();
        self.expect_token(Token::KeywordCalled, "Expected 'called' after 'event'")?;

        let (name, name_line, name_column) = if let Some(token) = self.tokens.peek() {
            if let Token::Identifier(id) = &token.token {
                let line = token.line;
                let column = token.column;
                let n = id.clone();
                self.tokens.next();
                (n, line, column)
            } else {
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
            return Err(ParseError::new(
                "Expected identifier after 'called'".to_string(),
                start.line,
                start.column,
            ));
        };

        Ok(Statement::EventDefinition {
            name,
            parameters: Vec::new(),
            line: name_line,
            column: name_column,
        })
    }

    pub(crate) fn parse_event_trigger(&mut self) -> Result<Statement, ParseError> {
        let start = self.tokens.next().unwrap();
        self.expect_token(Token::KeywordEvent, "Expected 'event' after 'trigger'")?;

        let (name, name_line, name_column) = if let Some(token) = self.tokens.peek() {
            if let Token::Identifier(id) = &token.token {
                let line = token.line;
                let column = token.column;
                let n = id.clone();
                self.tokens.next();
                (n, line, column)
            } else {
                return Err(ParseError::new(
                    format!("Expected identifier after 'event', found {:?}", token.token),
                    token.line,
                    token.column,
                ));
            }
        } else {
            return Err(ParseError::new(
                "Expected identifier after 'event'".to_string(),
                start.line,
                start.column,
            ));
        };

        Ok(Statement::EventTrigger {
            name,
            arguments: Vec::new(),
            line: name_line,
            column: name_column,
        })
    }

    pub(crate) fn parse_event_handler(&mut self) -> Result<Statement, ParseError> {
        let start = self.tokens.next().unwrap();
        self.expect_token(Token::KeywordEvent, "Expected 'event' after 'on'")?;

        let (event_name, name_line, name_column) = if let Some(token) = self.tokens.peek() {
            if let Token::Identifier(id) = &token.token {
                let line = token.line;
                let column = token.column;
                let n = id.clone();
                self.tokens.next();
                (n, line, column)
            } else {
                return Err(ParseError::new(
                    format!("Expected identifier after 'event', found {:?}", token.token),
                    token.line,
                    token.column,
                ));
            }
        } else {
            return Err(ParseError::new(
                "Expected identifier after 'event'".to_string(),
                start.line,
                start.column,
            ));
        };

        self.expect_token(Token::Colon, "Expected ':' after event name")?;

        let mut handler_body = Vec::new();
        while let Some(token) = self.tokens.peek() {
            if matches!(token.token, Token::KeywordEnd) {
                break;
            }
            handler_body.push(self.parse_statement()?);
        }

        self.expect_token(Token::KeywordEnd, "Expected 'end' after event handler body")?;
        self.expect_token(Token::KeywordOn, "Expected 'on' after 'end'")?;

        Ok(Statement::EventHandler {
            event_source: Expression::Variable("self".to_string(), start.line, start.column),
            event_name,
            handler_body,
            line: name_line,
            column: name_column,
        })
    }

    pub(crate) fn parse_parent_method_call(&mut self) -> Result<Statement, ParseError> {
        let start = self.tokens.next().unwrap();
        self.expect_token(Token::KeywordAction, "Expected 'action' after 'parent'")?;

        let (method_name, name_line, name_column) = if let Some(token) = self.tokens.peek() {
            if let Token::Identifier(id) = &token.token {
                let line = token.line;
                let column = token.column;
                let n = id.clone();
                self.tokens.next();
                (n, line, column)
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
                start.line,
                start.column,
            ));
        };

        let arguments = if let Some(tok) = self.tokens.peek() {
            if tok.token == Token::KeywordWith {
                self.tokens.next();
                self.parse_argument_list()?
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        Ok(Statement::ParentMethodCall {
            method_name,
            arguments,
            line: name_line,
            column: name_column,
        })
    }

    #[allow(clippy::type_complexity)]
    pub(crate) fn parse_container_body(
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

        while let Some(token) = self.tokens.peek().cloned() {
            match token.token {
                Token::KeywordEnd => {
                    self.tokens.next();
                    self.expect_token(Token::KeywordContainer, "Expected 'container' after 'end'")?;
                    break;
                }
                Token::KeywordProperty => {
                    let prop = self.parse_property_definition(false)?;
                    properties.push(prop);
                }
                Token::KeywordStatic => {
                    self.tokens.next();
                    if let Some(next) = self.tokens.peek() {
                        match next.token {
                            Token::KeywordProperty => {
                                let prop = self.parse_property_definition(true)?;
                                static_properties.push(prop);
                            }
                            Token::KeywordAction => {
                                let action = self.parse_container_action_definition()?;
                                static_methods.push(action);
                            }
                            _ => {
                                return Err(ParseError::new(
                                    format!(
                                        "Expected 'property' or 'action' after 'static', found {:?}",
                                        next.token
                                    ),
                                    next.line,
                                    next.column,
                                ));
                            }
                        }
                    } else {
                        return Err(ParseError::new(
                            "Unexpected end of input after 'static'".to_string(),
                            token.line,
                            token.column,
                        ));
                    }
                }
                Token::KeywordAction => {
                    let action = self.parse_container_action_definition()?;
                    methods.push(action);
                }
                Token::KeywordEvent => {
                    let event = self.parse_event_definition_full()?;
                    events.push(event);
                }
                _ => {
                    return Err(ParseError::new(
                        format!(
                            "Expected 'property', 'static', 'action', 'event', or 'end', found {:?}",
                            token.token
                        ),
                        token.line,
                        token.column,
                    ));
                }
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

    pub(crate) fn parse_property_definition(
        &mut self,
        is_static: bool,
    ) -> Result<PropertyDefinition, ParseError> {
        let start = self.tokens.next().unwrap();

        let name = if let Some(token) = self.tokens.peek() {
            if let Token::Identifier(id) = &token.token {
                let name = id.clone();
                self.tokens.next();
                name
            } else {
                return Err(ParseError::new(
                    format!(
                        "Expected identifier after 'property', found {:?}",
                        token.token
                    ),
                    token.line,
                    token.column,
                ));
            }
        } else {
            return Err(ParseError::new(
                "Expected identifier after 'property'".to_string(),
                0,
                0,
            ));
        };

        let property_type = if let Some(token) = self.tokens.peek() {
            match &token.token {
                Token::KeywordOf => {
                    self.tokens.next();
                    if let Some(t2) = self.tokens.peek() {
                        match &t2.token {
                            Token::KeywordText => {
                                self.tokens.next();
                                Some(Type::Text)
                            }
                            Token::Identifier(tid) => {
                                let tid = tid.clone();
                                self.tokens.next();
                                Some(Type::Custom(tid))
                            }
                            _ => None,
                        }
                    } else {
                        None
                    }
                }
                _ => None,
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

        Ok(PropertyDefinition {
            name,
            property_type,
            default_value,
            validation_rules: Vec::new(),
            visibility: Visibility::Public,
            is_static,
            line: start.line,
            column: start.column,
        })
    }

    pub(crate) fn parse_event_definition_full(&mut self) -> Result<EventDefinition, ParseError> {
        let start = self.tokens.next().unwrap();
        self.expect_token(Token::KeywordCalled, "Expected 'called' after 'event'")?;

        let (name, name_line, name_column) = if let Some(token) = self.tokens.peek() {
            if let Token::Identifier(id) = &token.token {
                let line = token.line;
                let column = token.column;
                let n = id.clone();
                self.tokens.next();
                (n, line, column)
            } else {
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
            return Err(ParseError::new(
                "Expected identifier after 'called'".to_string(),
                start.line,
                start.column,
            ));
        };

        Ok(EventDefinition {
            name,
            parameters: Vec::new(),
            line: name_line,
            column: name_column,
        })
    }

    pub(crate) fn parse_parameter_list(&mut self) -> Result<Vec<Parameter>, ParseError> {
        let mut params = Vec::new();

        while let Some(token) = self.tokens.peek().cloned() {
            let (param_name, param_line, param_column) = if let Token::Identifier(id) = &token.token
            {
                let line = token.line;
                let column = token.column;
                self.tokens.next();
                (id.clone(), line, column)
            } else {
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

            params.push(Parameter {
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

        Ok(params)
    }

    pub(crate) fn parse_instantiation_body(
        &mut self,
    ) -> Result<Vec<PropertyInitializer>, ParseError> {
        self.expect_token(Token::Colon, "Expected ':' after container instantiation")?;

        let mut property_initializers: Vec<PropertyInitializer> = Vec::new();

        while let Some(token) = self.tokens.peek().cloned() {
            match token.token {
                Token::KeywordEnd => {
                    self.tokens.next();
                    self.expect_token(
                        Token::KeywordCreate,
                        "Expected 'create' after 'end' in container instantiation",
                    )?;
                    break;
                }
                Token::KeywordProperty => {
                    self.tokens.next();
                    let (name, name_line, name_column) = if let Some(token) = self.tokens.peek() {
                        if let Token::Identifier(id) = &token.token {
                            let line = token.line;
                            let column = token.column;
                            let n = id.clone();
                            self.tokens.next();
                            (n, line, column)
                        } else {
                            return Err(ParseError::new(
                                format!(
                                    "Expected identifier after 'property', found {:?}",
                                    token.token
                                ),
                                token.line,
                                token.column,
                            ));
                        }
                    } else {
                        return Err(ParseError::new(
                            "Expected identifier after 'property'".to_string(),
                            0,
                            0,
                        ));
                    };

                    self.expect_token(Token::KeywordIs, "Expected 'is' after property name")?;
                    let value = self.parse_expression()?;

                    property_initializers.push(PropertyInitializer {
                        name,
                        value,
                        line: name_line,
                        column: name_column,
                    });
                }
                Token::KeywordAction => {
                    return Err(ParseError::new(
                        "Methods are not supported in container instantiation bodies".to_string(),
                        token.line,
                        token.column,
                    ));
                }
                _ => {
                    return Err(ParseError::new(
                        format!(
                            "Expected 'property', 'method', or 'end' in instantiation body, found {:?}",
                            token.token
                        ),
                        token.line,
                        token.column,
                    ));
                }
            }
        }

        Ok(property_initializers)
    }

    pub(crate) fn parse_container_action_definition(&mut self) -> Result<Statement, ParseError> {
        self.tokens.next();

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

        if let Some(token) = self.tokens.peek().cloned()
            && matches!(token.token, Token::KeywordNeeds)
        {
            self.tokens.next();
            parameters = self.parse_parameter_list()?;
        }

        let return_type = None;

        self.expect_token(Token::Colon, "Expected ':' after action declaration")?;

        let mut body = Vec::new();

        loop {
            if let Some(token) = self.tokens.peek() {
                if token.token == Token::KeywordEnd {
                    self.tokens.next();
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
}

impl<'a> Parser<'a> {
    pub(crate) fn parse_inheritance2(
        &mut self,
    ) -> Result<(Option<String>, Vec<String>), ParseError> {
        let mut extends = None;
        let mut implements = Vec::new();

        if let Some(token) = self.tokens.peek()
            && token.token == Token::KeywordExtends
        {
            self.tokens.next();

            if let Some(token) = self.tokens.peek() {
                if let Token::Identifier(id) = &token.token {
                    extends = Some(id.clone());
                    self.tokens.next();
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

        if let Some(token) = self.tokens.peek()
            && token.token == Token::KeywordImplements
        {
            self.tokens.next();

            loop {
                if let Some(token) = self.tokens.peek() {
                    if let Token::Identifier(id) = &token.token {
                        implements.push(id.clone());
                        self.tokens.next();

                        if let Some(next_token) = self.tokens.peek() {
                            if next_token.token == Token::Comma {
                                self.tokens.next();
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
}
