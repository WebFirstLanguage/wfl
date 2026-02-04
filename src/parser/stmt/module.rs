//! Module import and loading statement parsing

use super::super::{ExportType, ParseError, Parser, Statement};
use crate::lexer::token::Token;
use crate::parser::expr::ExprParser;

pub(crate) trait ModuleParser<'a>: ExprParser<'a> {
    fn parse_load_module_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_include_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_export_statement(&mut self) -> Result<Statement, ParseError>;
}

impl<'a> ModuleParser<'a> for Parser<'a> {
    fn parse_load_module_statement(&mut self) -> Result<Statement, ParseError> {
        // Capture the position of the 'load' token before consuming it
        let load_token = self.cursor.peek().ok_or_else(|| {
            ParseError::from_span(
                "Unexpected end of input while parsing load module statement".to_string(),
                self.cursor.current_span(),
                self.cursor.current_line(),
                1, // Column fallback when at EOF
            )
        })?;
        let (line, column) = (load_token.line, load_token.column);

        // Validate and consume the 'load' token
        self.expect_token(Token::KeywordLoad, "Expected 'load' keyword")?;

        self.expect_token(Token::KeywordModule, "Expected 'module' after 'load'")?;
        self.expect_token(Token::KeywordFrom, "Expected 'from' after 'module'")?;

        let path = self.parse_expression()?;

        // V1: Reject unsupported alias syntax with helpful error
        if let Some(next_token) = self.cursor.peek()
            && matches!(&next_token.token, Token::KeywordAs)
        {
            return Err(ParseError::from_token(
                "Module aliases are not yet supported. Use: load module from \"path\"".to_string(),
                next_token,
            ));
        }

        Ok(Statement::LoadModuleStatement {
            path,
            alias: None,
            line,
            column,
        })
    }

    fn parse_include_statement(&mut self) -> Result<Statement, ParseError> {
        // Capture the position of the 'include' token before consuming it
        let include_token = self.cursor.peek().ok_or_else(|| {
            ParseError::from_span(
                "Unexpected end of input while parsing include statement".to_string(),
                self.cursor.current_span(),
                self.cursor.current_line(),
                1, // Column fallback when at EOF
            )
        })?;
        let (line, column) = (include_token.line, include_token.column);

        // Validate and consume the 'include' token
        self.expect_token(Token::KeywordInclude, "Expected 'include' keyword")?;

        self.expect_token(Token::KeywordFrom, "Expected 'from' after 'include'")?;

        let path = self.parse_expression()?;

        Ok(Statement::IncludeStatement { path, line, column })
    }

    fn parse_export_statement(&mut self) -> Result<Statement, ParseError> {
        // Capture the position of the 'export' token before consuming it
        let export_token = self.cursor.peek().ok_or_else(|| {
            ParseError::from_span(
                "Unexpected end of input while parsing export statement".to_string(),
                self.cursor.current_span(),
                self.cursor.current_line(),
                1, // Column fallback when at EOF
            )
        })?;
        let (line, column) = (export_token.line, export_token.column);

        // Validate and consume the 'export' token
        self.expect_token(Token::KeywordExport, "Expected 'export' keyword")?;

        // Parse the export type
        let export_type = if let Some(token) = self.cursor.peek() {
            match &token.token {
                Token::KeywordContainer => {
                    self.expect_token(Token::KeywordContainer, "Expected 'container'")?;
                    ExportType::Container
                }
                Token::KeywordAction => {
                    self.expect_token(Token::KeywordAction, "Expected 'action'")?;
                    ExportType::Action
                }
                Token::KeywordConstant => {
                    self.expect_token(Token::KeywordConstant, "Expected 'constant'")?;
                    ExportType::Constant
                }
                _ => {
                    return Err(ParseError::from_token(
                        "Expected 'container', 'action', or 'constant' after 'export'".to_string(),
                        token,
                    ));
                }
            }
        } else {
            return Err(ParseError::from_span(
                "Expected export type after 'export'".to_string(),
                self.cursor.current_span(),
                line,
                column,
            ));
        };

        // Parse the name of the item to export
        let name = if let Some(token) = self.cursor.peek() {
            match &token.token {
                Token::Identifier(name) => {
                    let name = name.clone();
                    self.bump_sync(); // consume the identifier
                    name
                }
                _ => {
                    return Err(ParseError::from_token(
                        "Expected identifier after export type".to_string(),
                        token,
                    ));
                }
            }
        } else {
            return Err(ParseError::from_span(
                "Expected identifier after export type".to_string(),
                self.cursor.current_span(),
                line,
                column,
            ));
        };

        Ok(Statement::ExportStatement {
            export_type,
            name,
            line,
            column,
        })
    }
}
