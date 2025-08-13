pub mod ast;
#[cfg(test)]
mod tests;

use crate::exec_trace;
use crate::lexer::token::{Token, TokenWithPosition};
use ast::*;
use std::iter::Peekable;
use std::slice::Iter;

/// Checks if a pattern name conflicts with reserved keywords in WFL
fn is_reserved_pattern_name(name: &str) -> bool {
    matches!(name, 
        "url" | "digit" | "letter" | "file" | "database" | "data" | 
        "date" | "time" | "text" | "pattern" | "character" | "whitespace" |
        "unicode" | "category" | "script" | "greedy" | "lazy" | "zero" |
        "one" | "any" | "optional" | "between" | "start" | "ahead" |
        "behind" | "not" | "is" | "than" | "same" | "greater" | "less" |
        "equal" | "above" | "below" | "contains" | "matches" | "find" |
        "replace" | "split" | "capture" | "captured" | "more" | "exactly" |
        "push" | "add" | "subtract" | "multiply" | "divide" | "plus" |
        "minus" | "times" | "divided" | "by" | "open" | "close" |
        "read" | "write" | "append" | "content" | "wait" | "try" |
        "error" | "exists" | "list" | "map" | "remove" | "clear" |
        "files" | "found" | "permission" | "denied" | "recursively" |
        "extension" | "extensions" | "at" | "least" | "most" | "into" |
        "when" | "store" | "create" | "display" | "change" | "if" |
        "check" | "otherwise" | "then" | "end" | "as" | "to" | "from" |
        "with" | "and" | "or" | "count" | "for" | "each" | "in" |
        "reversed" | "repeat" | "while" | "until" | "forever" |
        "skip" | "continue" | "break" | "exit" | "loop" | "define" |
        "action" | "called" | "needs" | "give" | "back" | "return" |
        "directory" | "delete" | "container" | "property" | "extends" |
        "implements" | "interface" | "requires" | "event" | "trigger" |
        "on" | "static" | "public" | "private" | "parent" | "new" |
        "constant" | "must" | "defaults" | "of"
    )
}

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
            if let Some(first_token) = tokens_clone.next()
                && first_token.token == Token::KeywordEnd
            {
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
                        Token::KeywordMap => {
                            exec_trace!(
                                "Consuming orphaned 'end map' at line {}",
                                first_token.line
                            );
                            self.tokens.next(); // Consume "end"
                            self.tokens.next(); // Consume "map"
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
                                format!("Unexpected 'end' followed by {:?}", second_token.token),
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
            if let Some(token) = self.tokens.peek()
                && token.token == Token::KeywordEnd
                && start_len <= 2
            {
                // If we're at the end with just 1-2 tokens left, consume them and break
                while self.tokens.next().is_some() {}
                break;
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

        // Check for deprecated "create new constant" syntax
        if let Some(token) = self.tokens.peek()
            && matches!(token.token, Token::KeywordConstant)
        {
            // This is the deprecated "create new constant" syntax
            eprintln!(
                "Warning: 'create new constant' syntax is deprecated and will be removed in a future version. Please use 'store new constant' instead."
            );

            self.tokens.next(); // Consume "constant"

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
        if let Some(token) = self.tokens.peek()
            && token.token == Token::KeywordExtends
        {
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

        // Check for 'implements' keyword
        if let Some(token) = self.tokens.peek()
            && token.token == Token::KeywordImplements
        {
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
        if let Some(token) = self.tokens.peek()
            && token.token == Token::KeywordNeeds
        {
            self.tokens.next(); // Consume 'needs'
            parameters = self.parse_parameter_list()?;
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
                            Token::KeywordDirectory => self.parse_create_directory_statement(),
                            Token::KeywordFile => self.parse_create_file_statement(),
                            Token::KeywordList => self.parse_create_list_statement(),
                            Token::KeywordMap => self.parse_map_creation(),
                            Token::KeywordDate => self.parse_create_date_statement(),
                            Token::KeywordTime => self.parse_create_time_statement(),
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
                        && let Token::StringLiteral(_) = token.token
                        && let Some(token) = tokens_clone.next()
                        && token.token == Token::KeywordAnd
                        && let Some(token) = tokens_clone.next()
                        && token.token == Token::KeywordRead
                        && let Some(token) = tokens_clone.next()
                        && token.token == Token::KeywordContent
                        && let Some(token) = tokens_clone.next()
                        && token.token == Token::KeywordAs
                        && let Some(token) = tokens_clone.next()
                        && let Token::Identifier(_) = token.token
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
                    // Check if next token is "loop"
                    let mut tokens_clone = self.tokens.clone();
                    tokens_clone.next(); // Skip "main"
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

    fn parse_variable_declaration(&mut self) -> Result<Statement, ParseError> {
        let token_pos = self.tokens.next().unwrap();
        let is_store = matches!(token_pos.token, Token::KeywordStore);
        let _keyword = if is_store { "store" } else { "create" };

        // Check for "store new constant" syntax
        let mut is_constant = false;
        if is_store
            && let Some(next_token) = self.tokens.peek()
            && matches!(next_token.token, Token::KeywordNew)
        {
            self.tokens.next(); // Consume "new"
            if let Some(const_token) = self.tokens.peek() {
                if matches!(const_token.token, Token::KeywordConstant) {
                    self.tokens.next(); // Consume "constant"
                    is_constant = true;
                } else {
                    return Err(ParseError::new(
                        format!(
                            "Expected 'constant' after 'new', found {:?}",
                            const_token.token
                        ),
                        const_token.line,
                        const_token.column,
                    ));
                }
            } else {
                return Err(ParseError::new(
                    "Expected 'constant' after 'new'".to_string(),
                    token_pos.line,
                    token_pos.column,
                ));
            }
        }

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
                is_constant: false,
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
            is_constant,
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

    /// Consumes the next token if it matches the expected token, or returns a parse error with a custom message.
    ///
    /// Returns `Ok(())` if the next token matches `expected`. If the next token does not match or if the input is exhausted, returns a `ParseError` with the provided error message and token position.
    ///
    /// # Parameters
    /// - `expected`: The token that is expected next in the input.
    /// - `error_message`: The message to include in the error if the expectation is not met.
    ///
    /// # Returns
    /// - `Ok(())` if the expected token is found and consumed.
    /// - `Err(ParseError)` if the next token does not match or if the input ends unexpectedly.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// # use wfl::parser::Parser;
    /// # use wfl::lexer::token::Token;
    /// let mut parser = Parser::new(&tokens);
    /// parser.expect_token(Token::Colon, "Expected ':' after name")?;
    /// ```
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

    /// Consumes all tokens until "end pattern" is found to prevent cascading errors
    /// when a pattern definition fails early (e.g., due to reserved name)
    fn consume_pattern_body_on_error(&mut self) {
        // First, skip the colon if present
        if let Some(token) = self.tokens.peek() {
            if token.token == Token::Colon {
                self.tokens.next();
            }
        }
        
        let mut depth = 1; // We're inside one pattern block
        
        while let Some(token) = self.tokens.next() {
            match token.token {
                Token::KeywordEnd => {
                    // Check if this is "end pattern"
                    if let Some(next_token) = self.tokens.peek() {
                        if next_token.token == Token::KeywordPattern {
                            depth -= 1;
                            if depth == 0 {
                                self.tokens.next(); // Consume "pattern"
                                break;
                            }
                        }
                    }
                }
                Token::KeywordCreate => {
                    // Check if this is nested "create pattern"
                    if let Some(next_token) = self.tokens.peek() {
                        if next_token.token == Token::KeywordPattern {
                            depth += 1;
                        }
                    }
                }
                _ => {
                    // Continue consuming tokens
                }
            }
        }
    }

    /// Returns true if the token following the current position is the keyword "by".
    ///
    /// This method is typically used to detect the "divided by" operator sequence without advancing the main token iterator.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Assuming the parser is positioned at a "divided" token:
    /// if parser.peek_divided_by() {
    ///     // The next token is "by"
    /// }
    /// ```
    fn peek_divided_by(&mut self) -> bool {
        // Use nth(1) to directly access the token after "divided" without multiple advances
        if let Some(next_token_pos) = self.tokens.clone().nth(1) {
            matches!(next_token_pos.token, Token::KeywordBy)
        } else {
            false
        }
    }

    /// Parses an expression, starting with the lowest precedence.
    ///
    /// Returns the parsed expression or a parse error if the expression is invalid.
    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        self.parse_binary_expression(0) // Start with lowest precedence
    }

    /// Parses a binary expression with operator precedence.
    ///
    /// This method parses binary operations, handling operator precedence and associativity for a wide range of operators, including arithmetic, logical, comparison, pattern matching, and custom language constructs. It supports multi-token operators (such as "divided by", "is equal to", "is less than or equal to"), action calls, and special pattern-related expressions. The parser ensures correct grouping and evaluation order by recursively parsing sub-expressions with increasing precedence.
    ///
    /// # Parameters
    /// - `precedence`: The minimum precedence level to consider when parsing operators. Operators with lower precedence will not be parsed at this level.
    ///
    /// # Returns
    /// Returns an `Expression` representing the parsed binary expression, or a `ParseError` if the syntax is invalid.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Parses an arithmetic expression with correct precedence
    /// let expr = parser.parse_binary_expression(0)?;
    /// // Example: "a plus b times c" parses as a + (b * c)
    /// ```
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
                Token::Plus => Some((Operator::Plus, 1)),
                Token::KeywordPlus => Some((Operator::Plus, 1)),
                Token::Minus => Some((Operator::Minus, 1)),
                Token::KeywordMinus => Some((Operator::Minus, 1)),
                Token::KeywordTimes => Some((Operator::Multiply, 2)),
                Token::KeywordDividedBy => Some((Operator::Divide, 2)),
                Token::KeywordDivided => {
                    // Check if next token is "by" more efficiently
                    if self.peek_divided_by() {
                        Some((Operator::Divide, 2))
                    } else {
                        return Err(ParseError::new(
                            "Expected 'by' after 'divided'".to_string(),
                            line,
                            column,
                        ));
                    }
                }
                Token::Equals => Some((Operator::Equals, 0)),
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
                    if let Expression::Variable(ref name, var_line, var_column) = left
                        && self.known_actions.contains(name)
                    {
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
                    if let Some(equal_token) = self.tokens.peek().cloned()
                        && matches!(equal_token.token, Token::KeywordEqual)
                    {
                        self.tokens.next(); // Consume "equal"

                        if let Some(to_token) = self.tokens.peek().cloned()
                            && matches!(to_token.token, Token::KeywordTo)
                        {
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

                    Some((Operator::Or, 0))
                }
                Token::KeywordMatches => {
                    self.tokens.next(); // Consume "matches"

                    // Check if next token is "pattern" keyword (optional)
                    if let Some(pattern_token) = self.tokens.peek().cloned()
                        && matches!(pattern_token.token, Token::KeywordPattern)
                    {
                        self.tokens.next(); // Consume "pattern"
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
                    if let Some(pattern_token) = self.tokens.peek().cloned()
                        && matches!(pattern_token.token, Token::KeywordPattern)
                    {
                        self.tokens.next(); // Consume "pattern"
                    }

                    let pattern_expr = self.parse_binary_expression(precedence + 1)?;

                    if let Some(in_token) = self.tokens.peek().cloned()
                        && matches!(in_token.token, Token::KeywordIn)
                    {
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
                    if let Some(pattern_token) = self.tokens.peek().cloned()
                        && matches!(pattern_token.token, Token::KeywordPattern)
                    {
                        self.tokens.next(); // Consume "pattern"
                    }

                    let pattern_expr = self.parse_binary_expression(precedence + 1)?;

                    if let Some(with_token) = self.tokens.peek().cloned()
                        && matches!(with_token.token, Token::KeywordWith)
                    {
                        self.tokens.next(); // Consume "with"

                        let replacement_expr = self.parse_binary_expression(precedence + 1)?;

                        if let Some(in_token) = self.tokens.peek().cloned()
                            && matches!(in_token.token, Token::KeywordIn)
                        {
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

                        left = Expression::PatternReplace {
                            text: Box::new(left),
                            pattern: Box::new(pattern_expr),
                            replacement: Box::new(replacement_expr),
                            line,
                            column,
                        };
                        continue; // Skip the rest of the loop since we've already updated left
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

                    if let Some(on_token) = self.tokens.peek().cloned()
                        && matches!(on_token.token, Token::KeywordOn)
                    {
                        self.tokens.next(); // Consume "on"

                        // Check if next token is "pattern" keyword (optional)
                        if let Some(pattern_token) = self.tokens.peek().cloned()
                            && matches!(pattern_token.token, Token::KeywordPattern)
                        {
                            self.tokens.next(); // Consume "pattern"
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

                    return Err(ParseError::new(
                        "Expected 'on' after text in split operation".to_string(),
                        line,
                        column,
                    ));
                }
                Token::KeywordContains => {
                    self.tokens.next(); // Consume "contains"

                    if let Some(pattern_token) = self.tokens.peek().cloned()
                        && matches!(pattern_token.token, Token::KeywordPattern)
                    {
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

                // Now consume the operator token(s) since the precedence check passed
                match token {
                    Token::Plus => {
                        self.tokens.next(); // Consume "+"
                    }
                    Token::KeywordPlus => {
                        self.tokens.next(); // Consume "plus"
                    }
                    Token::KeywordMinus => {
                        self.tokens.next(); // Consume "minus"
                    }
                    Token::Minus => {
                        self.tokens.next(); // Consume "-"
                    }
                    Token::KeywordTimes => {
                        self.tokens.next(); // Consume "times"
                    }
                    Token::KeywordDividedBy => {
                        self.tokens.next(); // Consume "divided by"
                    }
                    Token::KeywordDivided => {
                        self.tokens.next(); // Consume "divided"
                        self.expect_token(Token::KeywordBy, "Expected 'by' after 'divided'")?;
                        self.tokens.next(); // Consume "by"
                    }
                    Token::Equals => {
                        self.tokens.next(); // Consume "="
                    }
                    _ => {
                        // For operators like "is" that have already consumed tokens in their detection
                        // No additional consumption needed
                    }
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
                    if let Some(next_token) = self.tokens.peek()
                        && next_token.token == Token::RightBracket
                    {
                        self.tokens.next(); // Consume ']'
                        return Ok(Expression::Literal(
                            Literal::List(Vec::new()),
                            bracket_token.line,
                            bracket_token.column,
                        ));
                    }

                    let mut elements = Vec::new();

                    // Parse first element - use a special method that doesn't parse operators
                    elements.push(self.parse_list_element()?);

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
                            elements.push(self.parse_list_element()?);
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
                    let token_line = token.line;
                    let token_column = token.column;

                    // Check for property access (dot notation)
                    if let Some(next_token) = self.tokens.peek().cloned() {
                        if next_token.token == Token::Dot {
                            self.tokens.next(); // Consume '.'

                            if let Some(property_token) = self.tokens.peek().cloned() {
                                if let Token::Identifier(property_name) = &property_token.token {
                                    self.tokens.next(); // Consume property name

                                    // Check for method call with parentheses
                                    if let Some(paren_token) = self.tokens.peek().cloned()
                                        && paren_token.token == Token::LeftParen
                                    {
                                        self.tokens.next(); // Consume '('

                                        let mut arguments = Vec::new();

                                        if let Some(next_token) = self.tokens.peek()
                                            && next_token.token != Token::RightParen
                                        {
                                            let expr = self.parse_expression()?;
                                            arguments.push(Argument {
                                                name: None,
                                                value: expr,
                                            });

                                            while let Some(comma_token) = self.tokens.peek() {
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

                                        self.expect_token(
                                            Token::RightParen,
                                            "Expected ')' after method arguments",
                                        )?;

                                        return Ok(Expression::MethodCall {
                                            object: Box::new(Expression::Variable(
                                                name.clone(),
                                                token_line,
                                                token_column,
                                            )),
                                            method: property_name.clone(),
                                            arguments,
                                            line: token_line,
                                            column: token_column,
                                        });
                                    }

                                    // Property access without method call
                                    return Ok(Expression::PropertyAccess {
                                        object: Box::new(Expression::Variable(
                                            name.clone(),
                                            token_line,
                                            token_column,
                                        )),
                                        property: property_name.clone(),
                                        line: token_line,
                                        column: token_column,
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
                                    token_line,
                                    token_column,
                                ));
                            }
                        } else if let Token::Identifier(id) = &next_token.token
                            && id.to_lowercase() == "with"
                        {
                            self.tokens.next(); // Consume "with"

                            let arguments = self.parse_argument_list()?;

                            return Ok(Expression::ActionCall {
                                name: name.clone(),
                                arguments,
                                line: token_line,
                                column: token_column,
                            });
                        }
                    }

                    let is_standalone = false;

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
                Token::Minus => {
                    self.tokens.next(); // Consume "-"
                    let expr = self.parse_primary_expression()?;
                    let token_line = token.line;
                    let token_column = token.column;
                    Ok(Expression::UnaryOperation {
                        operator: UnaryOperator::Minus,
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
                Token::KeywordFile => {
                    self.tokens.next(); // Consume "file"
                    let token_line = token.line;
                    let token_column = token.column;

                    // Check if it's "file exists at"
                    if let Some(next_token) = self.tokens.peek()
                        && next_token.token == Token::KeywordExists
                    {
                        self.tokens.next(); // Consume "exists"
                        self.expect_token(Token::KeywordAt, "Expected 'at' after 'file exists'")?;
                        let path = self.parse_primary_expression()?;
                        return Ok(Expression::FileExists {
                            path: Box::new(path),
                            line: token_line,
                            column: token_column,
                        });
                    }

                    // Otherwise treat "file" as a variable
                    Ok(Expression::Variable(
                        "file".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordDirectory => {
                    self.tokens.next(); // Consume "directory"
                    let token_line = token.line;
                    let token_column = token.column;

                    // Check if it's "directory exists at"
                    if let Some(next_token) = self.tokens.peek()
                        && next_token.token == Token::KeywordExists
                    {
                        self.tokens.next(); // Consume "exists"
                        self.expect_token(
                            Token::KeywordAt,
                            "Expected 'at' after 'directory exists'",
                        )?;
                        let path = self.parse_primary_expression()?;
                        return Ok(Expression::DirectoryExists {
                            path: Box::new(path),
                            line: token_line,
                            column: token_column,
                        });
                    }

                    // Otherwise treat "directory" as a variable
                    Ok(Expression::Variable(
                        "directory".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordList => {
                    self.tokens.next(); // Consume "list"
                    let token_line = token.line;
                    let token_column = token.column;

                    // Check if it's "list files [recursively] in"
                    if let Some(next_token) = self.tokens.peek()
                        && next_token.token == Token::KeywordFiles
                    {
                        self.tokens.next(); // Consume "files"

                        // Check if "recursively" comes before "in"
                        let mut is_recursive = false;
                        if let Some(token) = self.tokens.peek()
                            && token.token == Token::KeywordRecursively
                        {
                            self.tokens.next(); // Consume "recursively"
                            is_recursive = true;
                        }

                        self.expect_token(
                            Token::KeywordIn,
                            "Expected 'in' after 'list files [recursively]'",
                        )?;
                        let path = self.parse_primary_expression()?;

                        // Handle recursive listing (if not already handled)
                        if is_recursive {
                            // Check for "with extension/extensions" after recursive
                            if let Some(with_token) = self.tokens.peek()
                                && with_token.token == Token::KeywordWith
                            {
                                self.tokens.next(); // Consume "with"
                                let extensions = self.parse_extension_filter()?;
                                return Ok(Expression::ListFilesRecursive {
                                    path: Box::new(path),
                                    extensions: Some(extensions),
                                    line: token_line,
                                    column: token_column,
                                });
                            }

                            // Just recursively, no filter
                            return Ok(Expression::ListFilesRecursive {
                                path: Box::new(path),
                                extensions: None,
                                line: token_line,
                                column: token_column,
                            });
                        }

                        // Check for "recursively" or "with" after the path
                        if let Some(next) = self.tokens.peek() {
                            match &next.token {
                                Token::KeywordRecursively => {
                                    self.tokens.next(); // Consume "recursively"

                                    // Check for "with extension/extensions"
                                    if let Some(with_token) = self.tokens.peek()
                                        && with_token.token == Token::KeywordWith
                                    {
                                        self.tokens.next(); // Consume "with"
                                        let extensions = self.parse_extension_filter()?;
                                        return Ok(Expression::ListFilesRecursive {
                                            path: Box::new(path),
                                            extensions: Some(extensions),
                                            line: token_line,
                                            column: token_column,
                                        });
                                    }

                                    // Just recursively, no filter
                                    return Ok(Expression::ListFilesRecursive {
                                        path: Box::new(path),
                                        extensions: None,
                                        line: token_line,
                                        column: token_column,
                                    });
                                }
                                Token::KeywordWith => {
                                    self.tokens.next(); // Consume "with"
                                    let extensions = self.parse_extension_filter()?;
                                    return Ok(Expression::ListFilesFiltered {
                                        path: Box::new(path),
                                        extensions,
                                        line: token_line,
                                        column: token_column,
                                    });
                                }
                                _ => {}
                            }
                        }

                        // Basic list files
                        return Ok(Expression::ListFiles {
                            path: Box::new(path),
                            line: token_line,
                            column: token_column,
                        });
                    }

                    // Otherwise treat "list" as a variable
                    Ok(Expression::Variable(
                        "list".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordRead => {
                    self.tokens.next(); // Consume "read"
                    let token_line = token.line;
                    let token_column = token.column;

                    // Check if it's "read content from"
                    if let Some(next_token) = self.tokens.peek()
                        && next_token.token == Token::KeywordContent
                    {
                        self.tokens.next(); // Consume "content"
                        self.expect_token(
                            Token::KeywordFrom,
                            "Expected 'from' after 'read content'",
                        )?;
                        let file_handle = self.parse_primary_expression()?;
                        return Ok(Expression::ReadContent {
                            file_handle: Box::new(file_handle),
                            line: token_line,
                            column: token_column,
                        });
                    }

                    // Otherwise treat "read" as a variable
                    Ok(Expression::Variable(
                        "read".to_string(),
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
                        // Support direct index access: listName index (e.g., states 1)
                        Token::IntLiteral(index) => {
                            // Only treat as index access for specific base kinds and when on the same source line
                            if matches!(
                                expr,
                                Expression::Variable(_, _, _)
                                    | Expression::IndexAccess { .. }
                                    | Expression::FunctionCall { .. }
                                    | Expression::PropertyAccess { .. }
                                    | Expression::MethodCall { .. }
                            ) {
                                // Extract base expr span for anchoring
                                let (base_line, base_col) = match &expr {
                                    Expression::Variable(_, line, col)
                                    | Expression::IndexAccess {
                                        line, column: col, ..
                                    }
                                    | Expression::FunctionCall {
                                        line, column: col, ..
                                    }
                                    | Expression::PropertyAccess {
                                        line, column: col, ..
                                    }
                                    | Expression::MethodCall {
                                        line, column: col, ..
                                    } => (*line, *col),
                                    _ => (token.line, token.column),
                                };
                                // Guard: require same line to avoid cross-line capture
                                if token.line != base_line {
                                    break;
                                }
                                self.tokens.next(); // Consume the number
                                expr = Expression::IndexAccess {
                                    collection: Box::new(expr),
                                    index: Box::new(Expression::Literal(
                                        Literal::Integer(*index),
                                        token.line,
                                        token.column,
                                    )),
                                    line: base_line,
                                    column: base_col,
                                };
                            } else {
                                break; // Not an index access; stop parsing postfix operators
                            }
                        }
                        Token::KeywordOf => {
                            self.tokens.next(); // Consume "of"

                            // Parse the first argument after "of"
                            // Use parse_primary_expression to avoid treating "and" as a binary operator
                            let first_arg = self.parse_primary_expression()?;

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

                                        // Use parse_primary_expression to avoid treating next "and" as binary operator
                                        let arg_value = self.parse_primary_expression()?;

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
                        Token::LeftBracket => {
                            self.tokens.next(); // Consume "["

                            let index = self.parse_expression()?;

                            // Expect closing bracket
                            if let Some(closing_token) = self.tokens.peek().cloned() {
                                if closing_token.token == Token::RightBracket {
                                    self.tokens.next(); // Consume "]"
                                    expr = Expression::IndexAccess {
                                        collection: Box::new(expr),
                                        index: Box::new(index),
                                        line: token.line,
                                        column: token.column,
                                    };
                                } else {
                                    return Err(ParseError::new(
                                        format!(
                                            "Expected ']' after array index, found {:?}",
                                            closing_token.token
                                        ),
                                        closing_token.line,
                                        closing_token.column,
                                    ));
                                }
                            } else {
                                return Err(ParseError::new(
                                    "Expected ']' after array index, found end of input"
                                        .to_string(),
                                    token.line,
                                    token.column,
                                ));
                            }
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

    fn parse_if_statement(&mut self) -> Result<Statement, ParseError> {
        let check_token = self.tokens.next().unwrap(); // Consume "check" and store for line/column info

        self.expect_token(Token::KeywordIf, "Expected 'if' after 'check'")?;

        let condition = self.parse_expression()?;

        if let Some(token) = self.tokens.peek()
            && matches!(token.token, Token::Colon)
        {
            self.tokens.next(); // Consume the colon if present
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

                if let Some(token) = self.tokens.peek()
                    && matches!(token.token, Token::Colon)
                {
                    self.tokens.next(); // Consume the colon if present
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

        if let Some(token) = self.tokens.peek()
            && matches!(token.token, Token::Colon)
        {
            self.tokens.next(); // Consume the colon if present
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

        if let Some(token) = self.tokens.peek()
            && matches!(token.token, Token::Colon)
        {
            self.tokens.next(); // Consume the colon if present
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
        if let Some(token) = self.tokens.peek().cloned()
            && let Token::Identifier(id) = &token.token
            && id == "and"
        {
            self.tokens.next(); // Consume the extra "and"
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

    fn parse_arithmetic_operation(&mut self) -> Result<Statement, ParseError> {
        let op_token = self.tokens.next().unwrap(); // Consume "add", "subtract", "multiply", or "divide"

        // For multiply and divide: variable comes first, then "by", then value
        // For add: value comes first, then "to", then variable
        // For subtract: value comes first, then "from", then variable

        let (name, value) = match op_token.token {
            Token::KeywordAdd => {
                // add 5 to cn1
                let value = self.parse_expression()?;
                self.expect_token(Token::KeywordTo, "Expected 'to' after value in add")?;
                let name = self.parse_variable_name_simple()?;
                (name, value)
            }
            Token::KeywordSubtract => {
                // subtract 2 from cn1
                let value = self.parse_expression()?;
                self.expect_token(
                    Token::KeywordFrom,
                    "Expected 'from' after value in subtract",
                )?;
                let name = self.parse_variable_name_simple()?;
                (name, value)
            }
            Token::KeywordMultiply | Token::KeywordDivide => {
                // multiply cn1 by 3 or divide cn1 by 2
                let name = self.parse_variable_name_simple()?;
                self.expect_token(Token::KeywordBy, "Expected 'by' after variable name")?;
                let value = self.parse_expression()?;
                (name, value)
            }
            _ => unreachable!(),
        };

        // Create the appropriate operation
        let operator = match op_token.token {
            Token::KeywordAdd => Operator::Plus,
            Token::KeywordSubtract => Operator::Minus,
            Token::KeywordMultiply => Operator::Multiply,
            Token::KeywordDivide => Operator::Divide,
            _ => unreachable!(),
        };

        // Create a binary operation expression
        let var_expr = Expression::Variable(name.clone(), op_token.line, op_token.column);
        let binary_expr = Expression::BinaryOperation {
            left: Box::new(var_expr),
            operator,
            right: Box::new(value),
            line: op_token.line,
            column: op_token.column,
        };

        // Return an assignment statement
        Ok(Statement::Assignment {
            name,
            value: binary_expr,
            line: op_token.line,
            column: op_token.column,
        })
    }

    fn parse_variable_name_simple(&mut self) -> Result<String, ParseError> {
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
            } else {
                break;
            }
        }

        if !has_identifier {
            return Err(ParseError::new(
                "Expected variable name".to_string(),
                self.tokens.peek().map_or(0, |t| t.line),
                self.tokens.peek().map_or(0, |t| t.column),
            ));
        }

        Ok(name)
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
                    if let Some(token) = self.tokens.peek().cloned()
                        && token.token == Token::KeywordAt
                    {
                        self.tokens.next(); // Consume "at"

                        let url_expr = self.parse_primary_expression()?;

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

                                let variable_name = if let Some(token) = self.tokens.peek().cloned()
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

                                let variable_name = if let Some(token) = self.tokens.peek().cloned()
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

        if let Some(token) = self.tokens.peek().cloned()
            && token.token == Token::KeywordAt
        {
            self.tokens.next(); // Consume "at"

            let path_expr = self.parse_primary_expression()?;

            // Check for "for append", "and read content as" pattern AND direct "as" pattern
            if let Some(next_token) = self.tokens.peek().cloned() {
                if next_token.token == Token::KeywordFor {
                    // Check for "for [mode] as" pattern where mode can be append, reading, or writing
                    self.tokens.next(); // Consume "for"

                    let mode = if let Some(token) = self.tokens.peek().cloned() {
                        match token.token {
                            Token::KeywordAppend => {
                                self.tokens.next(); // Consume "append"
                                FileOpenMode::Append
                            }
                            Token::Identifier(ref mode_str) if mode_str == "reading" => {
                                self.tokens.next(); // Consume "reading"
                                FileOpenMode::Read
                            }
                            Token::Identifier(ref mode_str) if mode_str == "writing" => {
                                self.tokens.next(); // Consume "writing"
                                FileOpenMode::Write
                            }
                            _ => {
                                return Err(ParseError::new(
                                    "Expected 'append', 'reading', or 'writing' after 'for'"
                                        .to_string(),
                                    token.line,
                                    token.column,
                                ));
                            }
                        }
                    } else {
                        return Err(ParseError::new(
                            "Expected mode after 'for'".to_string(),
                            next_token.line,
                            next_token.column,
                        ));
                    };

                    self.expect_token(Token::KeywordAs, "Expected 'as' after file mode")?;

                    let variable_name = if let Some(token) = self.tokens.peek().cloned() {
                        if let Token::Identifier(name) = &token.token {
                            self.tokens.next(); // Consume the identifier
                            name.clone()
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

                    return Ok(Statement::OpenFileStatement {
                        path: path_expr,
                        variable_name,
                        mode,
                        line: open_token.line,
                        column: open_token.column,
                    });
                } else if next_token.token == Token::KeywordAnd {
                    // Original pattern: "open file at "path" and read content as variable"
                    self.tokens.next(); // Consume "and"
                    self.expect_token(Token::KeywordRead, "Expected 'read' after 'and'")?;
                    self.expect_token(Token::KeywordContent, "Expected 'content' after 'read'")?;
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
                        return Err(ParseError::new("Unexpected end of input".to_string(), 0, 0));
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

                    return Ok(Statement::OpenFileStatement {
                        path: path_expr,
                        variable_name,
                        mode: FileOpenMode::Read,
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

        let path = self.parse_primary_expression()?;

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
            mode: FileOpenMode::Read,
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

        // Check if the next token is "file" (for "close file file_handle" syntax)
        // Otherwise, parse the expression directly (for "close file_handle" syntax)
        if let Some(next_token) = self.tokens.peek()
            && next_token.token == Token::KeywordFile
        {
            self.tokens.next(); // Consume "file"
        }

        let file = self.parse_expression()?;

        Ok(Statement::CloseFileStatement {
            file,
            line: token_pos.line,
            column: token_pos.column,
        })
    }

    fn parse_create_directory_statement(&mut self) -> Result<Statement, ParseError> {
        let token_pos = self.tokens.next().unwrap(); // Consume "create"
        self.expect_token(
            Token::KeywordDirectory,
            "Expected 'directory' after 'create'",
        )?;
        self.expect_token(Token::KeywordAt, "Expected 'at' after 'create directory'")?;

        let path = self.parse_primary_expression()?;

        Ok(Statement::CreateDirectoryStatement {
            path,
            line: token_pos.line,
            column: token_pos.column,
        })
    }

    fn parse_create_file_statement(&mut self) -> Result<Statement, ParseError> {
        let token_pos = self.tokens.next().unwrap(); // Consume "create"
        self.expect_token(Token::KeywordFile, "Expected 'file' after 'create'")?;
        self.expect_token(Token::KeywordAt, "Expected 'at' after 'create file'")?;

        let path = self.parse_primary_expression()?;

        self.expect_token(Token::KeywordWith, "Expected 'with' after file path")?;
        let content = self.parse_expression()?;

        Ok(Statement::CreateFileStatement {
            path,
            content,
            line: token_pos.line,
            column: token_pos.column,
        })
    }

    fn parse_write_to_statement(&mut self) -> Result<Statement, ParseError> {
        let token_pos = self.tokens.next().unwrap(); // Consume "write"

        let content = self.parse_expression()?;

        self.expect_token(
            Token::KeywordTo,
            "Expected 'to' after content in write statement",
        )?;

        let file = self.parse_primary_expression()?;

        Ok(Statement::WriteToStatement {
            content,
            file,
            line: token_pos.line,
            column: token_pos.column,
        })
    }

    fn parse_delete_statement(&mut self) -> Result<Statement, ParseError> {
        let token_pos = self.tokens.next().unwrap(); // Consume "delete"

        // Check if next token is "file" or "directory"
        if let Some(next_token) = self.tokens.peek() {
            match next_token.token {
                Token::KeywordFile => {
                    self.tokens.next(); // Consume "file"
                    self.expect_token(Token::KeywordAt, "Expected 'at' after 'delete file'")?;
                    let path = self.parse_primary_expression()?;

                    Ok(Statement::DeleteFileStatement {
                        path,
                        line: token_pos.line,
                        column: token_pos.column,
                    })
                }
                Token::KeywordDirectory => {
                    self.tokens.next(); // Consume "directory"
                    self.expect_token(Token::KeywordAt, "Expected 'at' after 'delete directory'")?;
                    let path = self.parse_primary_expression()?;

                    Ok(Statement::DeleteDirectoryStatement {
                        path,
                        line: token_pos.line,
                        column: token_pos.column,
                    })
                }
                _ => Err(ParseError::new(
                    format!(
                        "Expected 'file' or 'directory' after 'delete', found {:?}",
                        next_token.token
                    ),
                    next_token.line,
                    next_token.column,
                )),
            }
        } else {
            Err(ParseError::new(
                "Expected 'file' or 'directory' after 'delete'".to_string(),
                token_pos.line,
                token_pos.column,
            ))
        }
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
            if matches!(
                token.token,
                Token::KeywordWhen | Token::KeywordOtherwise | Token::KeywordEnd
            ) {
                break;
            }
            body.push(self.parse_statement()?);
        }

        let mut when_clauses = Vec::new();
        let mut otherwise_block = None;

        // Parse when clauses
        while let Some(token) = self.tokens.peek().cloned() {
            match &token.token {
                Token::KeywordWhen => {
                    self.tokens.next(); // Consume "when"

                    // Parse error type
                    let (error_type, error_name) = if let Some(next_token) = self.tokens.peek() {
                        match &next_token.token {
                            Token::KeywordError => {
                                self.tokens.next(); // Consume "error"
                                (ast::ErrorType::General, "error".to_string())
                            }
                            Token::KeywordFile => {
                                self.tokens.next(); // Consume "file"
                                self.expect_token(
                                    Token::KeywordNot,
                                    "Expected 'not' after 'file'",
                                )?;
                                self.expect_token(
                                    Token::KeywordFound,
                                    "Expected 'found' after 'not'",
                                )?;
                                (ast::ErrorType::FileNotFound, "error".to_string())
                            }
                            Token::KeywordPermission => {
                                self.tokens.next(); // Consume "permission"
                                self.expect_token(
                                    Token::KeywordDenied,
                                    "Expected 'denied' after 'permission'",
                                )?;
                                (ast::ErrorType::PermissionDenied, "error".to_string())
                            }
                            _ => {
                                return Err(ParseError::new(
                                    format!(
                                        "Expected 'error', 'file', or 'permission' after 'when', found {:?}",
                                        next_token.token
                                    ),
                                    next_token.line,
                                    next_token.column,
                                ));
                            }
                        }
                    } else {
                        return Err(ParseError::new(
                            "Unexpected end of input after 'when'".to_string(),
                            token.line,
                            token.column,
                        ));
                    };

                    self.expect_token(Token::Colon, "Expected ':' after error type")?;

                    let mut when_body = Vec::new();
                    while let Some(token) = self.tokens.peek().cloned() {
                        if matches!(
                            token.token,
                            Token::KeywordWhen | Token::KeywordOtherwise | Token::KeywordEnd
                        ) {
                            break;
                        }
                        when_body.push(self.parse_statement()?);
                    }

                    when_clauses.push(ast::WhenClause {
                        error_type,
                        error_name,
                        body: when_body,
                    });
                }
                Token::KeywordOtherwise => {
                    self.tokens.next(); // Consume "otherwise"
                    self.expect_token(Token::Colon, "Expected ':' after 'otherwise'")?;

                    let mut otherwise_body = Vec::new();
                    while let Some(token) = self.tokens.peek().cloned() {
                        if matches!(token.token, Token::KeywordEnd) {
                            break;
                        }
                        otherwise_body.push(self.parse_statement()?);
                    }
                    otherwise_block = Some(otherwise_body);
                    break;
                }
                Token::KeywordEnd => {
                    break;
                }
                _ => {
                    return Err(ParseError::new(
                        format!(
                            "Expected 'when', 'otherwise', or 'end', found {:?}",
                            token.token
                        ),
                        token.line,
                        token.column,
                    ));
                }
            }
        }

        // Ensure at least one when clause for backward compatibility
        if when_clauses.is_empty() {
            return Err(ParseError::new(
                "Try statement must have at least one 'when' clause".to_string(),
                try_token.line,
                try_token.column,
            ));
        }

        self.expect_token(Token::KeywordEnd, "Expected 'end' after try block")?;
        self.expect_token(Token::KeywordTry, "Expected 'try' after 'end'")?;

        Ok(Statement::TryStatement {
            body,
            when_clauses,
            otherwise_block,
            line: try_token.line,
            column: try_token.column,
        })
    }

    fn parse_main_loop(&mut self) -> Result<Statement, ParseError> {
        let main_token = self.tokens.next().unwrap(); // Consume "main"
        self.expect_token(Token::KeywordLoop, "Expected 'loop' after 'main'")?;
        self.expect_token(Token::Colon, "Expected ':' after 'main loop'")?;

        let mut body = Vec::new();
        while let Some(token) = self.tokens.peek().cloned() {
            if matches!(token.token, Token::KeywordEnd) {
                break;
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

    fn parse_repeat_statement(&mut self) -> Result<Statement, ParseError> {
        let repeat_token = self.tokens.next().unwrap(); // Consume "repeat"

        if let Some(token) = self.tokens.peek().cloned() {
            match token.token {
                Token::KeywordWhile => {
                    self.tokens.next(); // Consume "while"
                    let condition = self.parse_expression()?;
                    if let Some(token) = self.tokens.peek()
                        && matches!(token.token, Token::Colon)
                    {
                        self.tokens.next(); // Consume the colon if present
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
                    if let Some(token) = self.tokens.peek()
                        && matches!(token.token, Token::Colon)
                    {
                        self.tokens.next(); // Consume the colon if present
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
        if let Some(token) = self.tokens.peek().cloned()
            && let Token::Identifier(id) = &token.token
            && id.to_lowercase() == "loop"
        {
            self.tokens.next(); // Consume "loop"
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

        if let Some(token) = self.tokens.peek()
            && token.line == start_line
            && !Parser::is_statement_starter(&token.token)
        {
            // so we can continue parsing the expression
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
        if let Some(token) = self.tokens.peek().cloned()
            && matches!(token.token, Token::KeywordNeeds)
        {
            self.tokens.next(); // Consume "needs"
            parameters = self.parse_parameter_list()?;
        }

        // Parse return type if present (after parameters or action name)
        let return_type = if let Some(token) = self.tokens.peek().cloned()
            && matches!(token.token, Token::Colon)
        {
            self.tokens.next(); // Consume ':'

            // Check if the next token is actually a type identifier
            // If it's not, this colon just marks the start of the action body (no return type)
            if let Some(type_token) = self.tokens.peek().cloned() {
                if let Token::Identifier(type_name) = &type_token.token {
                    // Check if this identifier is a valid type name
                    let is_type = matches!(
                        type_name.as_str(),
                        "Text" | "Number" | "Boolean" | "Nothing" | "Pattern"
                    ) || type_name.chars().next().is_some_and(|c| c.is_uppercase());

                    if is_type {
                        self.tokens.next(); // Consume type name
                        Some(match type_name.as_str() {
                            "Text" => Type::Text,
                            "Number" => Type::Number,
                            "Boolean" => Type::Boolean,
                            "Nothing" => Type::Nothing,
                            "Pattern" => Type::Pattern,
                            _ => Type::Custom(type_name.clone()),
                        })
                    } else {
                        // This identifier is not a type, so no return type specified
                        None
                    }
                } else {
                    // Next token after ':' is not an identifier, so no return type
                    None
                }
            } else {
                // End of input after ':', so no return type
                None
            }
        } else {
            None
        };

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

        let (pattern_name, pattern_token) = if let Some(token) = self.tokens.next() {
            match &token.token {
                Token::Identifier(name) => (name.clone(), token.clone()),
                Token::KeywordUrl => ("url".to_string(), token.clone()),
                Token::KeywordDigit => ("digit".to_string(), token.clone()),
                Token::KeywordLetter => ("letter".to_string(), token.clone()),
                Token::KeywordFile => ("file".to_string(), token.clone()),
                Token::KeywordDatabase => ("database".to_string(), token.clone()),
                Token::KeywordData => ("data".to_string(), token.clone()),
                Token::KeywordDate => ("date".to_string(), token.clone()),
                Token::KeywordTime => ("time".to_string(), token.clone()),
                Token::KeywordText => ("text".to_string(), token.clone()),
                Token::KeywordPattern => ("pattern".to_string(), token.clone()),
                Token::KeywordCharacter => ("character".to_string(), token.clone()),
                Token::KeywordWhitespace => ("whitespace".to_string(), token.clone()),
                Token::KeywordUnicode => ("unicode".to_string(), token.clone()),
                Token::KeywordCategory => ("category".to_string(), token.clone()),
                Token::KeywordScript => ("script".to_string(), token.clone()),
                _ => {
                    return Err(ParseError::new(
                        "Expected pattern name after 'create pattern'".to_string(),
                        token.line,
                        token.column,
                    ));
                }
            }
        } else {
            return Err(ParseError::new(
                "Expected pattern name after 'create pattern'".to_string(),
                create_token.line,
                create_token.column,
            ));
        };

        // Check if pattern name is a reserved keyword
        if is_reserved_pattern_name(&pattern_name) {
            // Consume tokens until we find "end pattern" to prevent cascading errors
            self.consume_pattern_body_on_error();
            
            return Err(ParseError::new(
                format!("'{}' is a predefined pattern in WFL. Please choose a different name.", pattern_name),
                pattern_token.line,
                pattern_token.column,
            ));
        }

        self.expect_token(Token::Colon, "Expected ':' after pattern name")?;

        let mut pattern_parts = Vec::new();
        let mut depth = 1; // Track nesting depth for proper end matching

        while let Some(token) = self.tokens.peek() {
            match &token.token {
                Token::KeywordEnd => {
                    let mut tokens_clone = self.tokens.clone();
                    tokens_clone.next(); // Skip "end"
                    if let Some(next_token) = tokens_clone.next()
                        && next_token.token == Token::KeywordPattern
                    {
                        depth -= 1;
                        if depth == 0 {
                            self.tokens.next(); // Consume "end"
                            self.tokens.next(); // Consume "pattern"
                            break;
                        }
                    }
                    pattern_parts.push(self.tokens.next().unwrap().clone());
                }
                Token::KeywordCreate => {
                    // Check for nested pattern creation
                    let mut tokens_clone = self.tokens.clone();
                    tokens_clone.next(); // Skip "create"
                    if let Some(next_token) = tokens_clone.next()
                        && next_token.token == Token::KeywordPattern
                    {
                        depth += 1;
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

        // Parse the pattern parts into the new PatternExpression AST structure
        let pattern_expr = Self::parse_pattern_tokens(&pattern_parts)?;

        Ok(Statement::PatternDefinition {
            name: pattern_name,
            pattern: pattern_expr,
            line: create_token.line,
            column: create_token.column,
        })
    }

    fn parse_extension_filter(&mut self) -> Result<Vec<Expression>, ParseError> {
        // Expect "extension", "extensions", or "pattern"
        if let Some(token) = self.tokens.peek() {
            match &token.token {
                Token::KeywordExtension => {
                    self.tokens.next(); // Consume "extension"
                    // Parse single extension
                    let ext = self.parse_primary_expression()?;
                    Ok(vec![ext])
                }
                Token::KeywordExtensions => {
                    self.tokens.next(); // Consume "extensions"
                    // Parse list of extensions
                    let has_bracket = if let Some(token) = self.tokens.peek() {
                        token.token == Token::LeftBracket
                    } else {
                        false
                    };

                    if has_bracket {
                        // Parse list literal
                        let list_expr = self.parse_primary_expression()?;
                        if let Expression::Literal(Literal::List(items), _, _) = list_expr {
                            Ok(items)
                        } else {
                            Err(ParseError::new(
                                "Expected list of extensions after 'extensions'".to_string(),
                                0,
                                0,
                            ))
                        }
                    } else {
                        // Allow a variable containing the extensions list
                        let expr = self.parse_primary_expression()?;
                        Ok(vec![expr])
                    }
                }
                Token::KeywordPattern => {
                    self.tokens.next(); // Consume "pattern"
                    // Parse pattern expression (e.g., "*.wfl")
                    let expr = self.parse_primary_expression()?;
                    Ok(vec![expr])
                }
                _ => Err(ParseError::new(
                    "Expected 'extension', 'extensions', or 'pattern' after 'with'".to_string(),
                    token.line,
                    token.column,
                )),
            }
        } else {
            Err(ParseError::new(
                "Expected 'extension', 'extensions', or 'pattern' after 'with'".to_string(),
                0,
                0,
            ))
        }
    }

    fn parse_list_element(&mut self) -> Result<Expression, ParseError> {
        // Parse a single list element without parsing binary operators
        // This prevents "and" from being interpreted as a boolean operator
        self.parse_primary_expression()
    }

    /// Parse tokens into the new PatternExpression AST structure
    fn parse_pattern_tokens(tokens: &[TokenWithPosition]) -> Result<PatternExpression, ParseError> {
        if tokens.is_empty() {
            return Err(ParseError::new(
                "Empty pattern definition".to_string(),
                0,
                0,
            ));
        }

        let mut i = 0;
        Self::parse_pattern_sequence(tokens, &mut i)
    }

    /// Parse a sequence of pattern elements (handles alternation with "or")
    fn parse_pattern_sequence(
        tokens: &[TokenWithPosition],
        i: &mut usize,
    ) -> Result<PatternExpression, ParseError> {
        let mut alternatives = vec![Self::parse_pattern_concatenation(tokens, i)?];

        while *i < tokens.len() {
            if let Token::KeywordOr = tokens[*i].token {
                *i += 1; // Skip "or"
                alternatives.push(Self::parse_pattern_concatenation(tokens, i)?);
            } else {
                break;
            }
        }

        if alternatives.len() == 1 {
            Ok(alternatives.into_iter().next().unwrap())
        } else {
            Ok(PatternExpression::Alternative(alternatives))
        }
    }

    /// Parse a concatenation of pattern elements (sequence)
    fn parse_pattern_concatenation(
        tokens: &[TokenWithPosition],
        i: &mut usize,
    ) -> Result<PatternExpression, ParseError> {
        let mut elements = Vec::new();

        while *i < tokens.len() {
            // Stop if we hit "or" (handled at a higher level)
            if let Token::KeywordOr = tokens[*i].token {
                break;
            }

            // Skip newlines
            if let Token::Newline = tokens[*i].token {
                *i += 1;
                continue;
            }

            // Skip "then" as it's just natural language syntax for sequencing
            if let Token::KeywordThen = tokens[*i].token {
                *i += 1;
                continue;
            }

            // Skip "followed by" as it's just natural language syntax
            if *i < tokens.len()
                && let Token::Identifier(s) = &tokens[*i].token
                && s == "followed"
                && *i + 1 < tokens.len()
                && let Token::KeywordBy = tokens[*i + 1].token
            {
                *i += 2; // Skip "followed by"
                continue;
            }

            // Debug: Print the current token before parsing
            if *i < tokens.len() {
                exec_trace!(
                    "Pattern concatenation: About to parse token {:?} at position {}",
                    tokens[*i].token,
                    *i
                );
            }

            elements.push(Self::parse_pattern_element(tokens, i)?);
        }

        if elements.is_empty() {
            return Err(ParseError::new(
                "Expected pattern element".to_string(),
                0,
                0,
            ));
        }

        if elements.len() == 1 {
            Ok(elements.into_iter().next().unwrap())
        } else {
            Ok(PatternExpression::Sequence(elements))
        }
    }

    /// Parse a single pattern element (literal, character class, quantified, etc.)
    fn parse_pattern_element(
        tokens: &[TokenWithPosition],
        i: &mut usize,
    ) -> Result<PatternExpression, ParseError> {
        if *i >= tokens.len() {
            return Err(ParseError::new(
                "Unexpected end of pattern".to_string(),
                0,
                0,
            ));
        }

        let token = &tokens[*i];
        let element = match &token.token {
            // String literals
            Token::StringLiteral(s) => {
                *i += 1;
                PatternExpression::Literal(s.clone())
            }

            // Character classes
            Token::KeywordAny => {
                *i += 1;
                if *i < tokens.len() {
                    match &tokens[*i].token {
                        Token::KeywordLetter => {
                            *i += 1;
                            PatternExpression::CharacterClass(CharClass::Letter)
                        }
                        Token::KeywordDigit => {
                            *i += 1;
                            PatternExpression::CharacterClass(CharClass::Digit)
                        }
                        Token::KeywordWhitespace => {
                            *i += 1;
                            PatternExpression::CharacterClass(CharClass::Whitespace)
                        }
                        Token::KeywordCharacter => {
                            *i += 1;
                            PatternExpression::CharacterClass(CharClass::Any)
                        }
                        _ => {
                            return Err(ParseError::new(
                                "Expected 'letter', 'digit', 'whitespace', or 'character' after 'any'"
                                    .to_string(),
                                tokens[*i].line,
                                tokens[*i].column,
                            ));
                        }
                    }
                } else {
                    return Err(ParseError::new(
                        "Expected character class after 'any'".to_string(),
                        token.line,
                        token.column,
                    ));
                }
            }

            // Handle quantifiers that start with specific keywords
            Token::KeywordOne => {
                if *i + 2 < tokens.len()
                    && tokens[*i + 1].token == Token::KeywordOr
                    && tokens[*i + 2].token == Token::KeywordMore
                {
                    // This is "one or more" which should be handled as a quantifier
                    // We need to parse the following element and then apply the quantifier
                    *i += 3; // Skip "one or more"

                    // Optionally consume "of" keyword
                    if *i < tokens.len() && tokens[*i].token == Token::KeywordOf {
                        *i += 1; // Skip "of"
                    }

                    let base_element = Self::parse_pattern_element(tokens, i)?;
                    PatternExpression::Quantified {
                        pattern: Box::new(base_element),
                        quantifier: Quantifier::OneOrMore,
                    }
                } else {
                    return Err(ParseError::new(
                        "Unexpected 'one' in pattern (did you mean 'one or more'?)".to_string(),
                        token.line,
                        token.column,
                    ));
                }
            }

            Token::KeywordZero => {
                if *i + 2 < tokens.len()
                    && tokens[*i + 1].token == Token::KeywordOr
                    && tokens[*i + 2].token == Token::KeywordMore
                {
                    // This is "zero or more" which should be handled as a quantifier
                    *i += 3; // Skip "zero or more"

                    // Optionally consume "of" keyword
                    if *i < tokens.len() && tokens[*i].token == Token::KeywordOf {
                        *i += 1; // Skip "of"
                    }

                    let base_element = Self::parse_pattern_element(tokens, i)?;
                    PatternExpression::Quantified {
                        pattern: Box::new(base_element),
                        quantifier: Quantifier::ZeroOrMore,
                    }
                } else {
                    return Err(ParseError::new(
                        "Unexpected 'zero' in pattern (did you mean 'zero or more'?)".to_string(),
                        token.line,
                        token.column,
                    ));
                }
            }

            Token::KeywordOptional => {
                // This is "optional" which should be handled as a quantifier
                *i += 1; // Skip "optional"
                let base_element = Self::parse_pattern_element(tokens, i)?;
                PatternExpression::Quantified {
                    pattern: Box::new(base_element),
                    quantifier: Quantifier::Optional,
                }
            }

            Token::KeywordExactly => {
                // Handle "exactly N element" syntax
                *i += 1; // Skip "exactly"
                if *i < tokens.len() {
                    if let Token::IntLiteral(n) = tokens[*i].token {
                        *i += 1; // Skip the number

                        // Optionally consume "of" keyword
                        if *i < tokens.len() && tokens[*i].token == Token::KeywordOf {
                            *i += 1; // Skip "of"
                        }

                        let base_element = Self::parse_pattern_element(tokens, i)?;
                        PatternExpression::Quantified {
                            pattern: Box::new(base_element),
                            quantifier: Quantifier::Exactly(n as u32),
                        }
                    } else {
                        return Err(ParseError::new(
                            "Expected number after 'exactly' in pattern".to_string(),
                            tokens[*i].line,
                            tokens[*i].column,
                        ));
                    }
                } else {
                    return Err(ParseError::new(
                        "Expected number after 'exactly' in pattern".to_string(),
                        token.line,
                        token.column,
                    ));
                }
            }

            Token::KeywordAt => {
                // Handle "at least N" or "at most N" syntax
                *i += 1; // Skip "at"
                if *i < tokens.len() {
                    match &tokens[*i].token {
                        Token::KeywordLeast => {
                            *i += 1; // Skip "least"
                            if *i < tokens.len() {
                                if let Token::IntLiteral(n) = tokens[*i].token {
                                    *i += 1; // Skip the number

                                    // Optionally consume "of" keyword
                                    if *i < tokens.len() && tokens[*i].token == Token::KeywordOf {
                                        *i += 1; // Skip "of"
                                    }

                                    let base_element = Self::parse_pattern_element(tokens, i)?;
                                    PatternExpression::Quantified {
                                        pattern: Box::new(base_element),
                                        quantifier: Quantifier::AtLeast(n as u32),
                                    }
                                } else {
                                    return Err(ParseError::new(
                                        "Expected number after 'at least' in pattern".to_string(),
                                        tokens[*i].line,
                                        tokens[*i].column,
                                    ));
                                }
                            } else {
                                return Err(ParseError::new(
                                    "Expected number after 'at least' in pattern".to_string(),
                                    token.line,
                                    token.column,
                                ));
                            }
                        }
                        Token::KeywordMost => {
                            *i += 1; // Skip "most"
                            if *i < tokens.len() {
                                if let Token::IntLiteral(n) = tokens[*i].token {
                                    *i += 1; // Skip the number

                                    // Optionally consume "of" keyword
                                    if *i < tokens.len() && tokens[*i].token == Token::KeywordOf {
                                        *i += 1; // Skip "of"
                                    }

                                    let base_element = Self::parse_pattern_element(tokens, i)?;
                                    PatternExpression::Quantified {
                                        pattern: Box::new(base_element),
                                        quantifier: Quantifier::AtMost(n as u32),
                                    }
                                } else {
                                    return Err(ParseError::new(
                                        "Expected number after 'at most' in pattern".to_string(),
                                        tokens[*i].line,
                                        tokens[*i].column,
                                    ));
                                }
                            } else {
                                return Err(ParseError::new(
                                    "Expected number after 'at most' in pattern".to_string(),
                                    token.line,
                                    token.column,
                                ));
                            }
                        }
                        _ => {
                            return Err(ParseError::new(
                                "Expected 'least' or 'most' after 'at' in pattern".to_string(),
                                tokens[*i].line,
                                tokens[*i].column,
                            ));
                        }
                    }
                } else {
                    return Err(ParseError::new(
                        "Expected 'least' or 'most' after 'at' in pattern".to_string(),
                        token.line,
                        token.column,
                    ));
                }
            }

            // Handle "N to M" syntax for numeric ranges
            Token::IntLiteral(min) => {
                let min_val = *min as u32;
                *i += 1; // Skip the number

                // Check if this is a range pattern "N to M"
                if *i + 1 < tokens.len() && tokens[*i].token == Token::KeywordTo {
                    *i += 1; // Skip "to"
                    if let Token::IntLiteral(max) = tokens[*i].token {
                        *i += 1; // Skip the max number

                        // Optionally consume "of" keyword
                        if *i < tokens.len() && tokens[*i].token == Token::KeywordOf {
                            *i += 1; // Skip "of"
                        }

                        let base_element = Self::parse_pattern_element(tokens, i)?;
                        PatternExpression::Quantified {
                            pattern: Box::new(base_element),
                            quantifier: Quantifier::Between(min_val, max as u32),
                        }
                    } else {
                        return Err(ParseError::new(
                            "Expected number after 'to' in pattern".to_string(),
                            tokens[*i].line,
                            tokens[*i].column,
                        ));
                    }
                } else {
                    // It's just a number literal, treat it as a literal pattern
                    PatternExpression::Literal(min.to_string())
                }
            }

            // Direct character classes
            Token::KeywordLetter => {
                *i += 1;
                PatternExpression::CharacterClass(CharClass::Letter)
            }
            Token::KeywordDigit => {
                *i += 1;
                PatternExpression::CharacterClass(CharClass::Digit)
            }
            Token::KeywordWhitespace => {
                *i += 1;
                PatternExpression::CharacterClass(CharClass::Whitespace)
            }

            // Handle plural forms of character classes
            Token::Identifier(s) if s == "letters" => {
                *i += 1;
                PatternExpression::CharacterClass(CharClass::Letter)
            }
            Token::Identifier(s) if s == "digits" => {
                *i += 1;
                PatternExpression::CharacterClass(CharClass::Digit)
            }

            // Unicode patterns
            Token::KeywordUnicode => {
                *i += 1;
                if *i < tokens.len() {
                    match &tokens[*i].token {
                        Token::KeywordLetter => {
                            *i += 1;
                            PatternExpression::CharacterClass(CharClass::UnicodeProperty(
                                "Alphabetic".to_string(),
                            ))
                        }
                        Token::KeywordDigit => {
                            *i += 1;
                            PatternExpression::CharacterClass(CharClass::UnicodeProperty(
                                "Numeric".to_string(),
                            ))
                        }
                        Token::KeywordCategory => {
                            *i += 1;
                            if *i < tokens.len() {
                                if let Token::StringLiteral(category) = &tokens[*i].token {
                                    *i += 1;
                                    PatternExpression::CharacterClass(CharClass::UnicodeCategory(
                                        category.clone(),
                                    ))
                                } else {
                                    return Err(ParseError::new(
                                        "Expected string literal after 'unicode category'"
                                            .to_string(),
                                        tokens[*i].line,
                                        tokens[*i].column,
                                    ));
                                }
                            } else {
                                return Err(ParseError::new(
                                    "Expected category name after 'unicode category'".to_string(),
                                    tokens[*i - 1].line,
                                    tokens[*i - 1].column,
                                ));
                            }
                        }
                        Token::KeywordScript => {
                            *i += 1;
                            if *i < tokens.len() {
                                if let Token::StringLiteral(script) = &tokens[*i].token {
                                    *i += 1;
                                    PatternExpression::CharacterClass(CharClass::UnicodeScript(
                                        script.clone(),
                                    ))
                                } else {
                                    return Err(ParseError::new(
                                        "Expected string literal after 'unicode script'"
                                            .to_string(),
                                        tokens[*i].line,
                                        tokens[*i].column,
                                    ));
                                }
                            } else {
                                return Err(ParseError::new(
                                    "Expected script name after 'unicode script'".to_string(),
                                    tokens[*i - 1].line,
                                    tokens[*i - 1].column,
                                ));
                            }
                        }
                        Token::Identifier(name) if name == "property" => {
                            *i += 1;
                            if *i < tokens.len() {
                                if let Token::StringLiteral(property) = &tokens[*i].token {
                                    *i += 1;
                                    PatternExpression::CharacterClass(CharClass::UnicodeProperty(
                                        property.clone(),
                                    ))
                                } else {
                                    return Err(ParseError::new(
                                        "Expected string literal after 'unicode property'"
                                            .to_string(),
                                        tokens[*i].line,
                                        tokens[*i].column,
                                    ));
                                }
                            } else {
                                return Err(ParseError::new(
                                    "Expected property name after 'unicode property'".to_string(),
                                    tokens[*i - 1].line,
                                    tokens[*i - 1].column,
                                ));
                            }
                        }
                        _ => {
                            return Err(ParseError::new(
                                "Expected 'letter', 'digit', 'category', 'script', or 'property' after 'unicode'".to_string(),
                                tokens[*i].line,
                                tokens[*i].column,
                            ));
                        }
                    }
                } else {
                    return Err(ParseError::new(
                        "Incomplete unicode pattern".to_string(),
                        tokens[*i - 1].line,
                        tokens[*i - 1].column,
                    ));
                }
            }

            // Anchors
            Token::KeywordStart => {
                if *i + 2 < tokens.len()
                    && tokens[*i + 1].token == Token::KeywordOf
                    && tokens[*i + 2].token == Token::KeywordText
                {
                    *i += 3;
                    PatternExpression::Anchor(Anchor::StartOfText)
                } else {
                    return Err(ParseError::new(
                        "Expected 'start of text'".to_string(),
                        token.line,
                        token.column,
                    ));
                }
            }

            // Capture groups
            Token::KeywordCapture => {
                *i += 1;
                if *i < tokens.len() && tokens[*i].token == Token::LeftBrace {
                    *i += 1; // Skip '{'

                    // Find the matching '}'
                    let start_pos = *i;
                    let mut brace_count = 1;
                    while *i < tokens.len() && brace_count > 0 {
                        match tokens[*i].token {
                            Token::LeftBrace => brace_count += 1,
                            Token::RightBrace => brace_count -= 1,
                            _ => {}
                        }
                        *i += 1;
                    }

                    if brace_count > 0 {
                        return Err(ParseError::new(
                            "Unclosed capture group".to_string(),
                            token.line,
                            token.column,
                        ));
                    }

                    let end_pos = *i - 1; // Before the closing '}'
                    let capture_tokens = &tokens[start_pos..end_pos];

                    // Expect "as" and capture name
                    if *i < tokens.len() && tokens[*i].token == Token::KeywordAs {
                        *i += 1;
                        if *i < tokens.len() {
                            if let Token::Identifier(name) = &tokens[*i].token {
                                *i += 1;
                                let mut inner_i = 0;
                                let inner_pattern =
                                    Self::parse_pattern_sequence(capture_tokens, &mut inner_i)?;
                                PatternExpression::Capture {
                                    name: name.clone(),
                                    pattern: Box::new(inner_pattern),
                                }
                            } else {
                                return Err(ParseError::new(
                                    "Expected identifier after 'as'".to_string(),
                                    tokens[*i].line,
                                    tokens[*i].column,
                                ));
                            }
                        } else {
                            return Err(ParseError::new(
                                "Expected capture name after 'as'".to_string(),
                                token.line,
                                token.column,
                            ));
                        }
                    } else {
                        return Err(ParseError::new(
                            "Expected 'as' after capture group".to_string(),
                            token.line,
                            token.column,
                        ));
                    }
                } else {
                    return Err(ParseError::new(
                        "Expected '{' after 'capture'".to_string(),
                        token.line,
                        token.column,
                    ));
                }
            }

            // Backreferences: "same as captured"
            Token::KeywordSame => {
                *i += 1;
                if *i + 1 < tokens.len()
                    && tokens[*i].token == Token::KeywordAs
                    && tokens[*i + 1].token == Token::KeywordCaptured
                {
                    *i += 2; // Skip "as captured"

                    if *i < tokens.len() {
                        if let Token::StringLiteral(name) = &tokens[*i].token {
                            *i += 1;
                            PatternExpression::Backreference(name.clone())
                        } else {
                            return Err(ParseError::new(
                                "Expected capture name (in quotes) after 'same as captured'"
                                    .to_string(),
                                tokens[*i].line,
                                tokens[*i].column,
                            ));
                        }
                    } else {
                        return Err(ParseError::new(
                            "Expected capture name after 'same as captured'".to_string(),
                            token.line,
                            token.column,
                        ));
                    }
                } else {
                    return Err(ParseError::new(
                        "Expected 'as captured' after 'same'".to_string(),
                        token.line,
                        token.column,
                    ));
                }
            }

            // Lookarounds: "check ahead for", "check not ahead for", "check behind for", "check not behind for"
            Token::KeywordCheck => {
                *i += 1;
                if *i >= tokens.len() {
                    return Err(ParseError::new(
                        "Expected 'ahead' or 'behind' after 'check'".to_string(),
                        token.line,
                        token.column,
                    ));
                }

                let is_negative = if tokens[*i].token == Token::KeywordNot {
                    *i += 1;
                    true
                } else {
                    false
                };

                if *i >= tokens.len() {
                    return Err(ParseError::new(
                        "Expected 'ahead' or 'behind' after 'check'".to_string(),
                        token.line,
                        token.column,
                    ));
                }

                match &tokens[*i].token {
                    Token::KeywordAhead => {
                        *i += 1;
                        if *i < tokens.len() && tokens[*i].token == Token::KeywordFor {
                            *i += 1; // Skip "for"

                            // Parse the pattern inside braces
                            if *i < tokens.len() && tokens[*i].token == Token::LeftBrace {
                                *i += 1; // Skip "{"
                                let pattern_start = *i;

                                // Find matching right brace
                                let mut brace_count = 1;
                                let mut pattern_end = *i;
                                while pattern_end < tokens.len() && brace_count > 0 {
                                    match &tokens[pattern_end].token {
                                        Token::LeftBrace => brace_count += 1,
                                        Token::RightBrace => brace_count -= 1,
                                        _ => {}
                                    }
                                    if brace_count > 0 {
                                        pattern_end += 1;
                                    }
                                }

                                if brace_count != 0 {
                                    return Err(ParseError::new(
                                        "Unmatched '{' in lookahead pattern".to_string(),
                                        tokens[pattern_start - 1].line,
                                        tokens[pattern_start - 1].column,
                                    ));
                                }

                                let pattern_tokens = &tokens[pattern_start..pattern_end];
                                *i = pattern_end + 1; // Skip past '}'

                                let inner_pattern = Self::parse_pattern_tokens(pattern_tokens)?;

                                if is_negative {
                                    PatternExpression::NegativeLookahead(Box::new(inner_pattern))
                                } else {
                                    PatternExpression::Lookahead(Box::new(inner_pattern))
                                }
                            } else {
                                return Err(ParseError::new(
                                    "Expected '{' after 'check ahead for'".to_string(),
                                    token.line,
                                    token.column,
                                ));
                            }
                        } else {
                            return Err(ParseError::new(
                                "Expected 'for' after 'check ahead'".to_string(),
                                token.line,
                                token.column,
                            ));
                        }
                    }
                    Token::KeywordBehind => {
                        *i += 1;
                        if *i < tokens.len() && tokens[*i].token == Token::KeywordFor {
                            *i += 1; // Skip "for"

                            // Parse the pattern inside braces
                            if *i < tokens.len() && tokens[*i].token == Token::LeftBrace {
                                *i += 1; // Skip "{"
                                let pattern_start = *i;

                                // Find matching right brace
                                let mut brace_count = 1;
                                let mut pattern_end = *i;
                                while pattern_end < tokens.len() && brace_count > 0 {
                                    match &tokens[pattern_end].token {
                                        Token::LeftBrace => brace_count += 1,
                                        Token::RightBrace => brace_count -= 1,
                                        _ => {}
                                    }
                                    if brace_count > 0 {
                                        pattern_end += 1;
                                    }
                                }

                                if brace_count != 0 {
                                    return Err(ParseError::new(
                                        "Unmatched '{' in lookbehind pattern".to_string(),
                                        tokens[pattern_start - 1].line,
                                        tokens[pattern_start - 1].column,
                                    ));
                                }

                                let pattern_tokens = &tokens[pattern_start..pattern_end];
                                *i = pattern_end + 1; // Skip past '}'

                                let inner_pattern = Self::parse_pattern_tokens(pattern_tokens)?;

                                if is_negative {
                                    PatternExpression::NegativeLookbehind(Box::new(inner_pattern))
                                } else {
                                    PatternExpression::Lookbehind(Box::new(inner_pattern))
                                }
                            } else {
                                return Err(ParseError::new(
                                    "Expected '{' after 'check behind for'".to_string(),
                                    token.line,
                                    token.column,
                                ));
                            }
                        } else {
                            return Err(ParseError::new(
                                "Expected 'for' after 'check behind'".to_string(),
                                token.line,
                                token.column,
                            ));
                        }
                    }
                    _ => {
                        return Err(ParseError::new(
                            "Expected 'ahead' or 'behind' after 'check'".to_string(),
                            tokens[*i].line,
                            tokens[*i].column,
                        ));
                    }
                }
            }

            // Handle "by" token after identifier - this happens when "followed" was consumed elsewhere
            Token::KeywordBy => {
                // This is likely a stray "by" after "followed" was consumed
                // Just return an error suggesting the issue
                return Err(ParseError::new(
                    "Found 'by' keyword - did you mean 'followed by'? Note: 'followed by' should be used between pattern elements".to_string(),
                    token.line,
                    token.column,
                ));
            }

            // Parentheses for grouping
            Token::LeftParen => {
                *i += 1; // Skip '('

                // Find the matching right parenthesis
                let pattern_start = *i;
                let mut paren_count = 1;
                let mut pattern_end = *i;

                while pattern_end < tokens.len() && paren_count > 0 {
                    match &tokens[pattern_end].token {
                        Token::LeftParen => paren_count += 1,
                        Token::RightParen => paren_count -= 1,
                        _ => {}
                    }
                    if paren_count > 0 {
                        pattern_end += 1;
                    }
                }

                if paren_count != 0 {
                    return Err(ParseError::new(
                        "Unmatched '(' in pattern".to_string(),
                        token.line,
                        token.column,
                    ));
                }

                // Parse the pattern inside parentheses
                let inner_tokens = &tokens[pattern_start..pattern_end];
                *i = pattern_end + 1; // Skip past ')'

                if inner_tokens.is_empty() {
                    return Err(ParseError::new(
                        "Empty parentheses in pattern".to_string(),
                        token.line,
                        token.column,
                    ));
                }

                // Parse the inner pattern as a sequence
                let mut inner_i = 0;
                Self::parse_pattern_sequence(inner_tokens, &mut inner_i)?
            }

            // List references - identifiers that reference list variables
            Token::Identifier(name) => {
                *i += 1;
                PatternExpression::ListReference(name.clone())
            }

            _ => {
                return Err(ParseError::new(
                    format!("Unexpected token in pattern: {:?}", token.token),
                    token.line,
                    token.column,
                ));
            }
        };

        // Check for quantifiers after the element
        Self::parse_quantifier(tokens, i, element)
    }

    /// Parse quantifiers that can appear after base elements (exactly, between)
    fn parse_quantifier(
        tokens: &[TokenWithPosition],
        i: &mut usize,
        base_pattern: PatternExpression,
    ) -> Result<PatternExpression, ParseError> {
        if *i >= tokens.len() {
            return Ok(base_pattern);
        }

        match &tokens[*i].token {
            Token::KeywordExactly => {
                if *i + 1 < tokens.len() {
                    if let Token::IntLiteral(n) = tokens[*i + 1].token {
                        *i += 2;
                        Ok(PatternExpression::Quantified {
                            pattern: Box::new(base_pattern),
                            quantifier: Quantifier::Exactly(n as u32),
                        })
                    } else {
                        Ok(base_pattern)
                    }
                } else {
                    Ok(base_pattern)
                }
            }
            Token::KeywordBetween => {
                if *i + 3 < tokens.len() && tokens[*i + 2].token == Token::KeywordAnd {
                    if let (Token::IntLiteral(min), Token::IntLiteral(max)) =
                        (&tokens[*i + 1].token, &tokens[*i + 3].token)
                    {
                        *i += 4;
                        Ok(PatternExpression::Quantified {
                            pattern: Box::new(base_pattern),
                            quantifier: Quantifier::Between(*min as u32, *max as u32),
                        })
                    } else {
                        Ok(base_pattern)
                    }
                } else {
                    Ok(base_pattern)
                }
            }
            _ => Ok(base_pattern),
        }
    }

    fn parse_create_list_statement(&mut self) -> Result<Statement, ParseError> {
        let create_token = self.tokens.next().unwrap(); // Consume "create"
        self.expect_token(Token::KeywordList, "Expected 'list' after 'create'")?;

        // Parse list name
        let name = if let Some(token) = self.tokens.peek() {
            match &token.token {
                Token::Identifier(n) => {
                    let name = n.clone();
                    self.tokens.next(); // Consume the identifier
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

        // Expect colon
        self.expect_token(Token::Colon, "Expected ':' after list name")?;

        // Parse list items
        let mut initial_values = Vec::new();

        while let Some(token) = self.tokens.peek().cloned() {
            match token.token {
                Token::KeywordEnd => {
                    self.tokens.next(); // Consume "end"
                    self.expect_token(Token::KeywordList, "Expected 'list' after 'end'")?;
                    break;
                }
                Token::KeywordAdd => {
                    self.tokens.next(); // Consume "add"
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

    /// Parses a `create date` statement, optionally with a custom date value.
    ///
    /// Returns a `CreateDateStatement` containing the variable name and an optional expression for the date value. If no custom value is provided, the date defaults to "today".
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Parses: create date my_date
    /// // Parses: create date my_date as some_expression
    /// let stmt = parser.parse_create_date_statement()?;
    /// ```
    fn parse_create_date_statement(&mut self) -> Result<Statement, ParseError> {
        let create_token = self.tokens.next().unwrap(); // Consume "create"
        self.expect_token(Token::KeywordDate, "Expected 'date' after 'create'")?;

        // Parse the date variable name
        let name = self.parse_variable_name_simple()?;

        // Check if there's an "as" clause for a custom date value
        let value = if let Some(token) = self.tokens.peek() {
            if token.token == Token::KeywordAs {
                self.tokens.next(); // Consume "as"
                Some(self.parse_expression()?)
            } else {
                None // Default to "today"
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

    /// Parses a `create time` statement, optionally with a custom time value.
    ///
    /// Recognizes the syntax `create time <name>` or `create time <name> as <expression>`.
    /// If the `as` clause is omitted, the time defaults to the current time.
    ///
    /// # Returns
    /// A `Statement::CreateTimeStatement` containing the variable name, optional value expression, and source position.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Parses: create time start_time
    /// let stmt = parser.parse_create_time_statement().unwrap();
    ///
    /// // Parses: create time deadline as some_expression
    /// let stmt = parser.parse_create_time_statement().unwrap();
    /// ```
    fn parse_create_time_statement(&mut self) -> Result<Statement, ParseError> {
        let create_token = self.tokens.next().unwrap(); // Consume "create"
        self.expect_token(Token::KeywordTime, "Expected 'time' after 'create'")?;

        // Parse the time variable name
        let name = self.parse_variable_name_simple()?;

        // Check if there's an "as" clause for a custom time value
        let value = if let Some(token) = self.tokens.peek() {
            if token.token == Token::KeywordAs {
                self.tokens.next(); // Consume "as"
                Some(self.parse_expression()?)
            } else {
                None // Default to "now"
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

    fn parse_map_creation(&mut self) -> Result<Statement, ParseError> {
        let create_token = self.tokens.next().unwrap(); // Consume "create"
        self.expect_token(Token::KeywordMap, "Expected 'map' after 'create'")?;

        // Parse map name
        let name = if let Some(token) = self.tokens.peek() {
            match &token.token {
                Token::Identifier(n) => {
                    let name = n.clone();
                    self.tokens.next(); // Consume the identifier
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

        // Expect colon
        self.expect_token(Token::Colon, "Expected ':' after map name")?;

        // Parse map entries
        let mut entries = Vec::new();

        while let Some(token) = self.tokens.peek().cloned() {
            match &token.token {
                Token::KeywordEnd => {
                    self.tokens.next(); // Consume "end"
                    self.expect_token(Token::KeywordMap, "Expected 'map' after 'end'")?;
                    break;
                }
                Token::Identifier(key) => {
                    let key = key.clone();
                    self.tokens.next(); // Consume the key

                    // Expect "is"
                    self.expect_token(Token::KeywordIs, "Expected 'is' after map key")?;

                    // Parse the value expression
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

    fn parse_add_operation(&mut self) -> Result<Statement, ParseError> {
        // We need to determine if this is:
        // 1. Arithmetic: "add 5 to variable" (adds 5 to a numeric variable)
        // 2. List operation: "add "item" to list" (appends to a list)

        // Save the position to potentially backtrack
        let _saved_position = self.tokens.clone();
        let add_token = self.tokens.next().unwrap(); // Consume "add"

        // Parse the value to add
        let value = self.parse_expression()?;

        // Check for "to" keyword
        if let Some(token) = self.tokens.peek() {
            if token.token == Token::KeywordTo {
                self.tokens.next(); // Consume "to"

                // Parse the target name
                let target_name = self.parse_variable_name_simple()?;

                // Try to determine the operation type
                // For now, we'll check if the value is numeric to decide
                // The interpreter will handle the actual type checking
                match &value {
                    Expression::Literal(Literal::Integer(_), _, _)
                    | Expression::Literal(Literal::Float(_), _, _) => {
                        // Likely arithmetic operation
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
                    _ => {
                        // Treat as list operation
                        Ok(Statement::AddToListStatement {
                            value,
                            list_name: target_name,
                            line: add_token.line,
                            column: add_token.column,
                        })
                    }
                }
            } else {
                // No "to" keyword, this is an error
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

    fn parse_remove_from_list_statement(&mut self) -> Result<Statement, ParseError> {
        let remove_token = self.tokens.next().unwrap(); // Consume "remove"

        // Parse the value to remove
        let value = self.parse_expression()?;

        // Expect "from"
        self.expect_token(Token::KeywordFrom, "Expected 'from' after value in remove")?;

        // Parse the list name
        let list_name = self.parse_variable_name_simple()?;

        Ok(Statement::RemoveFromListStatement {
            value,
            list_name,
            line: remove_token.line,
            column: remove_token.column,
        })
    }

    fn parse_clear_list_statement(&mut self) -> Result<Statement, ParseError> {
        let clear_token = self.tokens.next().unwrap(); // Consume "clear"

        // Parse the list name
        let list_name = self.parse_variable_name_simple()?;

        // Optionally consume "list" keyword if present
        if let Some(token) = self.tokens.peek()
            && token.token == Token::KeywordList
        {
            self.tokens.next(); // Consume "list"
        }

        Ok(Statement::ClearListStatement {
            list_name,
            line: clear_token.line,
            column: clear_token.column,
        })
    }
}
