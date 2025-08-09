use super::Parser;
use super::ast::*;
use super::error::ParseError;
use crate::lexer::token::{Token, TokenWithPosition};

impl<'a> Parser<'a> {
    pub(crate) fn parse_create_pattern_statement(&mut self) -> Result<Statement, ParseError> {
        let create_token = self.tokens.next().unwrap();
        self.expect_token(Token::KeywordPattern, "Expected 'pattern' after 'create'")?;

        let pattern_name = self.parse_variable_name_simple()?;
        self.expect_token(Token::Colon, "Expected ':' after pattern name")?;

        let mut depth = 0usize;
        let mut pattern_parts: Vec<TokenWithPosition> = Vec::new();

        while let Some(token) = self.tokens.peek().cloned() {
            match &token.token {
                Token::KeywordEnd => {
                    let _ = self.tokens.next();
                    if let Some(next_token) = self.tokens.peek() {
                        if next_token.token == Token::KeywordPattern {
                            if depth == 0 {
                                let _ = self.tokens.next();
                                break;
                            } else {
                                depth -= 1;
                                let _ = self.tokens.next();
                                continue;
                            }
                        } else {
                            pattern_parts.push(token.clone());
                        }
                    } else {
                        pattern_parts.push(token.clone());
                    }
                }
                Token::KeywordCreate => {
                    if let Some(next_token) = self.tokens.clone().nth(1) {
                        if matches!(next_token.token, Token::KeywordPattern) {
                            depth += 1;
                        }
                        pattern_parts.push(self.tokens.next().unwrap().clone());
                    } else {
                        pattern_parts.push(self.tokens.next().unwrap().clone());
                    }
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

        let pattern_expr = Self::parse_pattern_tokens(&pattern_parts)?;

        Ok(Statement::PatternDefinition {
            name: pattern_name,
            pattern: pattern_expr,
            line: create_token.line,
            column: create_token.column,
        })
    }

    pub(crate) fn parse_pattern_tokens(
        tokens: &[TokenWithPosition],
    ) -> Result<PatternExpression, ParseError> {
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

    fn parse_pattern_sequence(
        tokens: &[TokenWithPosition],
        i: &mut usize,
    ) -> Result<PatternExpression, ParseError> {
        let mut alternatives = vec![Self::parse_pattern_concatenation(tokens, i)?];

        while *i < tokens.len() {
            if let Token::KeywordOr = tokens[*i].token {
                *i += 1;
                alternatives.push(Self::parse_pattern_concatenation(tokens, i)?);
            } else {
                break;
            }
        }

        if alternatives.len() == 1 {
            Ok(alternatives.remove(0))
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
            match &tokens[*i].token {
                Token::KeywordOr | Token::KeywordEnd | Token::Colon => break,
                _ => elements.push(Self::parse_pattern_element(tokens, i)?),
            }
        }

        if elements.len() == 1 {
            Ok(elements.remove(0))
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
        *i += 1;

        let mut element = match &token.token {
            Token::StringLiteral(s) => PatternExpression::Literal(s.clone()),
            Token::Identifier(id) => PatternExpression::Literal(id.clone()),
            Token::LeftParen => {
                let inner = Self::parse_pattern_sequence(tokens, i)?;
                if *i >= tokens.len() || tokens[*i].token != Token::RightParen {
                    return Err(ParseError::new(
                        "Expected ')' to close pattern group".to_string(),
                        token.line,
                        token.column,
                    ));
                }
                *i += 1;
                PatternExpression::Sequence(vec![inner])
            }
            Token::KeywordNot => {
                let inner = Self::parse_pattern_element(tokens, i)?;
                PatternExpression::NegativeLookahead(Box::new(inner))
            }
            Token::KeywordDigit => PatternExpression::CharacterClass(CharClass::Digit),
            Token::KeywordLetter => PatternExpression::CharacterClass(CharClass::Letter),
            Token::KeywordWhitespace => PatternExpression::CharacterClass(CharClass::Whitespace),
            Token::KeywordOne => {
                if *i + 1 < tokens.len()
                    && tokens[*i].token == Token::KeywordOr
                    && tokens[*i + 1].token == Token::KeywordMore
                {
                    *i += 2; // consumed 'or more' (we already moved past 'one')
                    if *i < tokens.len() && tokens[*i].token == Token::KeywordOf {
                        *i += 1; // optionally skip 'of'
                    }
                    let base = Self::parse_pattern_element(tokens, i)?;
                    PatternExpression::Quantified {
                        pattern: Box::new(base),
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
            _ => {
                return Err(ParseError::new(
                    format!("Unexpected token in pattern: {:?}", token.token),
                    token.line,
                    token.column,
                ));
            }
        };

        if *i < tokens.len()
            && let Some(q) = Self::parse_quantifier(tokens, i)?
        {
            element = PatternExpression::Quantified {
                pattern: Box::new(element),
                quantifier: q,
            };
        }

        Ok(element)
    }

    fn parse_quantifier(
        tokens: &[TokenWithPosition],
        i: &mut usize,
    ) -> Result<Option<Quantifier>, ParseError> {
        if *i >= tokens.len() {
            return Ok(None);
        }

        let token = &tokens[*i];

        let quant = match &token.token {
            Token::Plus => Some(Quantifier::OneOrMore),
            _ => None,
        };

        if quant.is_some() {
            *i += 1;
        }

        Ok(quant)
    }
}
