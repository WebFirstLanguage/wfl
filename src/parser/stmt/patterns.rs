//! Pattern matching and pattern definition statement parsing

use super::super::{Parser, ParseError, Statement, Expression, PatternExpression, Literal, CharClass, Anchor, Quantifier};
use crate::exec_trace;
use crate::lexer::token::{Token, TokenWithPosition};
use crate::parser::expr::{ExprParser, PrimaryExprParser};
use crate::parser::helpers::is_reserved_pattern_name;

pub(crate) trait PatternParser<'a>: ExprParser<'a> {
    fn parse_create_pattern_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_pattern_tokens(tokens: &[TokenWithPosition]) -> Result<PatternExpression, ParseError>;
    fn parse_pattern_sequence(
        tokens: &[TokenWithPosition],
        i: &mut usize,
    ) -> Result<PatternExpression, ParseError>;
    fn parse_pattern_concatenation(
        tokens: &[TokenWithPosition],
        i: &mut usize,
    ) -> Result<PatternExpression, ParseError>;
    fn parse_pattern_element(
        tokens: &[TokenWithPosition],
        i: &mut usize,
    ) -> Result<PatternExpression, ParseError>;
    fn parse_quantifier(
        tokens: &[TokenWithPosition],
        i: &mut usize,
        base_pattern: PatternExpression,
    ) -> Result<PatternExpression, ParseError>;
    fn parse_extension_filter(&mut self) -> Result<Vec<Expression>, ParseError>;
}

impl<'a> PatternParser<'a> for Parser<'a> {
    fn parse_create_pattern_statement(&mut self) -> Result<Statement, ParseError> {
        let create_token = self.bump_sync().unwrap(); // Consume "create"
        self.expect_token(Token::KeywordPattern, "Expected 'pattern' after 'create'")?;

        let (pattern_name, pattern_token) = if let Some(token) = self.bump_sync() {
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
                format!(
                    "'{}' is a predefined pattern in WFL. Please choose a different name.",
                    pattern_name
                ),
                pattern_token.line,
                pattern_token.column,
            ));
        }

        self.expect_token(Token::Colon, "Expected ':' after pattern name")?;

        let mut pattern_parts = Vec::new();
        let mut depth = 1; // Track nesting depth for proper end matching

        while let Some(token) = self.cursor.peek() {
            match &token.token {
                Token::KeywordEnd => {
                    // Check if this is "end pattern"
                    if let Some(next_token) = self.cursor.peek_next() {
                        if next_token.token == Token::KeywordPattern {
                            depth -= 1;
                            if depth == 0 {
                                self.bump_sync(); // Consume "end"
                                self.bump_sync(); // Consume "pattern"
                                break;
                            }
                        }
                    }
                    pattern_parts.push(self.bump_sync().unwrap().clone());
                }
                Token::KeywordCreate => {
                    // Check for nested pattern creation
                    if let Some(next_token) = self.cursor.peek_next() {
                        if next_token.token == Token::KeywordPattern {
                            depth += 1;
                        }
                    }
                    pattern_parts.push(self.bump_sync().unwrap().clone());
                }
                _ => {
                    pattern_parts.push(self.bump_sync().unwrap().clone());
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
        if let Some(token) = self.cursor.peek() {
            match &token.token {
                Token::KeywordExtension => {
                    self.bump_sync(); // Consume "extension"
                    // Parse single extension
                    let ext = self.parse_primary_expression()?;
                    Ok(vec![ext])
                }
                Token::KeywordExtensions => {
                    self.bump_sync(); // Consume "extensions"
                    // Parse list of extensions
                    let has_bracket = if let Some(token) = self.cursor.peek() {
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
                    self.bump_sync(); // Consume "pattern"
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

    fn parse_pattern_tokens(tokens: &[TokenWithPosition]) -> Result<PatternExpression, ParseError> {
        // Filter out Eol tokens from pattern definition
        let filtered_tokens: Vec<TokenWithPosition> = tokens
            .iter()
            .filter(|t| !matches!(t.token, Token::Eol))
            .cloned()
            .collect();

        if filtered_tokens.is_empty() {
            return Err(ParseError::new(
                "Empty pattern definition".to_string(),
                0,
                0,
            ));
        }

        let mut i = 0;
        Self::parse_pattern_sequence(&filtered_tokens, &mut i)
    }

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
}
