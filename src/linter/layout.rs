//! Token-based source layout shared by linting and source-preserving fixes.

use crate::lexer::lex_wfl_with_positions;
use crate::lexer::token::{Token, TokenWithPosition};
use std::ops::Range;

pub(crate) struct LineLayout {
    pub line_number: usize,
    /// The line contents, excluding CR/LF terminators.
    pub content: Range<usize>,
    pub indentation: Range<usize>,
    /// None for blank/comment lines and lines that start inside a string.
    pub depth: Option<usize>,
}

pub(crate) struct SourceLayout {
    pub lines: Vec<LineLayout>,
    pub string_spans: Vec<Range<usize>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Block {
    Ordinary,
    PostconditionRepeat,
    Route,
    RouteArm,
}

impl SourceLayout {
    pub fn new(source: &str) -> Self {
        let tokens = lex_wfl_with_positions(source);
        let string_spans: Vec<_> = tokens
            .iter()
            .filter(|token| matches!(token.token, Token::StringLiteral(_)))
            .map(|token| token.byte_start..token.byte_end)
            .collect();
        let mut lines = Vec::new();
        let mut stack = Vec::new();
        let mut token_index = 0;
        let mut line_start = 0;
        let bytes = source.as_bytes();

        while line_start < source.len() {
            let mut line_end = line_start;
            while line_end < bytes.len() && !matches!(bytes[line_end], b'\r' | b'\n') {
                line_end += 1;
            }
            let mut next_line = line_end;
            if bytes.get(next_line) == Some(&b'\r') {
                next_line += 1;
            }
            if bytes.get(next_line) == Some(&b'\n') {
                next_line += 1;
            }

            let text = &source[line_start..line_end];
            let indent_end = line_start + text.len() - text.trim_start().len();
            while token_index < tokens.len() && tokens[token_index].byte_start < line_start {
                token_index += 1;
            }
            let start = token_index;
            while token_index < tokens.len() && tokens[token_index].byte_start < line_end {
                token_index += 1;
            }
            let line_tokens = &tokens[start..token_index];
            let string_index = string_spans.partition_point(|span| span.end <= line_start);
            let inside_string = string_spans
                .get(string_index)
                .is_some_and(|span| span.start < line_start && line_start < span.end);

            // A line starting inside a literal must keep its indentation, but
            // real tokens after its closing quote can still open/close blocks.
            let token_depth =
                (!line_tokens.is_empty()).then(|| line_depth(line_tokens, &mut stack));
            let depth = if inside_string { None } else { token_depth };
            lines.push(LineLayout {
                line_number: lines.len() + 1,
                content: line_start..line_end,
                indentation: line_start..indent_end,
                depth,
            });
            line_start = next_line;
        }
        Self {
            lines,
            string_spans,
        }
    }

    pub fn overlaps_string(&self, range: &Range<usize>) -> bool {
        let index = self
            .string_spans
            .partition_point(|span| span.end <= range.start);
        self.string_spans
            .get(index)
            .is_some_and(|span| span.start < range.end)
    }
}

/// Return this line's indentation depth and update the blocks for the next line.
/// Only body definitions open blocks; action exports and interface requirements
/// reference an existing action and leave the surrounding nesting unchanged.
fn line_depth(tokens: &[TokenWithPosition], stack: &mut Vec<Block>) -> usize {
    // Header lookups must stay linear even for very long or incomplete lines.
    let mut header_ends = vec![None; tokens.len() + 1];
    for index in (0..tokens.len()).rev() {
        header_ends[index] = if matches!(tokens[index].token, Token::Colon | Token::KeywordThen) {
            Some(index)
        } else {
            header_ends[index + 1]
        };
    }
    let mut depth = stack.len();
    let mut index = 0;
    while index < tokens.len() {
        let token = &tokens[index].token;
        if matches!(token, Token::KeywordUntil) && stack.last() == Some(&Block::PostconditionRepeat)
        {
            stack.pop();
            if index == 0 {
                depth = stack.len();
            }
            index += 1;
            continue;
        }
        if matches!(token, Token::KeywordEnd) {
            if stack.last() == Some(&Block::RouteArm) {
                stack.pop();
            }
            stack.pop();
            if index == 0 {
                depth = stack.len();
            }
            index += 1;
            // The keyword in `end check` closes a block, rather than opening
            // another one. Bare `end` (container methods) has no such suffix.
            if tokens
                .get(index)
                .is_some_and(|token| is_end_suffix(&token.token))
            {
                index += 1;
            }
            continue;
        }

        if matches!(
            token,
            Token::KeywordOtherwise
                | Token::KeywordWhen
                | Token::KeywordCatch
                | Token::KeywordFinally
        ) {
            let branch_depth = if matches!(token, Token::KeywordWhen | Token::KeywordOtherwise)
                && matches!(stack.last(), Some(Block::Route | Block::RouteArm))
            {
                if stack.last() == Some(&Block::RouteArm) {
                    stack.pop();
                }
                let branch_depth = stack.len();
                stack.push(Block::RouteArm);
                branch_depth
            } else {
                stack.len().saturating_sub(1)
            };
            if index == 0 {
                depth = branch_depth;
            }
            // Skip the branch condition. For `otherwise check if` this also
            // skips the chained check, which shares its owner's terminator;
            // `otherwise: check if` leaves the separately closed check next.
            index = header_ends[index].unwrap_or(index) + 1;
            continue;
        }

        if matches!(token, Token::KeywordExport) {
            index += 1;
            continue;
        }
        if matches!(token, Token::KeywordAction)
            && index > 0
            && matches!(
                tokens[index - 1].token,
                Token::KeywordRequires | Token::KeywordExport
            )
        {
            // Keep scanning: another statement may follow on the same line.
            index += 1;
            continue;
        }
        let has_colon = header_ends[index].is_some_and(|end| tokens[end].token == Token::Colon);
        if let Some(block) = opened_block(&tokens[index..], has_colon) {
            stack.push(block);
            if let Some(end) = header_ends[index] {
                index = end + 1;
            } else {
                // Optional-colon forms such as `check if yes` and `for each`
                // use the rest of this line as the block header.
                break;
            }
        } else {
            index += 1;
        }
    }
    depth
}

fn is_end_suffix(token: &Token) -> bool {
    matches!(
        token,
        Token::KeywordCheck
            | Token::KeywordIf
            | Token::KeywordFor
            | Token::KeywordCount
            | Token::KeywordRepeat
            | Token::KeywordLoop
            | Token::KeywordAction
            | Token::KeywordList
            | Token::KeywordMap
            | Token::KeywordPattern
            | Token::KeywordTry
            | Token::KeywordRoute
            | Token::KeywordDescribe
            | Token::KeywordTest
            | Token::KeywordSetup
            | Token::KeywordTeardown
            | Token::KeywordOn
    ) || matches!(token, Token::Identifier(name) if name.eq_ignore_ascii_case("transaction"))
}

/// Recognize body headers without borrowing a colon from a following statement.
fn opened_block(tokens: &[TokenWithPosition], has_colon: bool) -> Option<Block> {
    let first = &tokens.first()?.token;
    let second = tokens.get(1).map(|token| &token.token);
    match first {
        Token::KeywordRoute => Some(Block::Route),
        Token::KeywordRepeat if second == Some(&Token::Colon) => Some(Block::PostconditionRepeat),
        Token::KeywordCheck
        | Token::KeywordIf
        | Token::KeywordRepeat
        | Token::KeywordTry
        | Token::KeywordDescribe
        | Token::KeywordTest
        | Token::KeywordSetup
        | Token::KeywordTeardown
        | Token::KeywordAction => Some(Block::Ordinary),
        Token::KeywordFor if second == Some(&Token::KeywordEach) => Some(Block::Ordinary),
        Token::KeywordCount if second == Some(&Token::KeywordFrom) => Some(Block::Ordinary),
        Token::KeywordDefine if second == Some(&Token::KeywordAction) => Some(Block::Ordinary),
        Token::KeywordStatic if second == Some(&Token::KeywordAction) => Some(Block::Ordinary),
        Token::KeywordCreate if second == Some(&Token::KeywordNew) => {
            // Container initialization requires `new Type as name:`. The
            // supported legacy `new constant name as value` form is a variable
            // declaration, even when its value contains named-argument colons.
            let mut header = tokens.iter().skip(2).map(|token| &token.token);
            matches!(
                (header.next(), header.next(), header.next(), header.next()),
                (
                    Some(Token::Identifier(_)),
                    Some(Token::KeywordAs),
                    Some(Token::Identifier(_)),
                    Some(Token::Colon)
                )
            )
            .then_some(Block::Ordinary)
        }
        Token::KeywordCreate if second == Some(&Token::KeywordInterface) => {
            interface_has_body(tokens).then_some(Block::Ordinary)
        }
        Token::KeywordCreate
            if matches!(
                second,
                Some(Token::KeywordList | Token::KeywordMap | Token::KeywordPattern)
            ) =>
        {
            // `create list` is also an expression, and `create`, `map`, and
            // `pattern` can be contextual variable operands in expressions.
            // A supported declaration requires its own name and colon.
            matches!(
                (
                    tokens.get(2).map(|token| &token.token),
                    tokens.get(3).map(|token| &token.token)
                ),
                (Some(Token::Identifier(_)), Some(Token::Colon))
            )
            .then_some(Block::Ordinary)
        }
        Token::KeywordCreate if second == Some(&Token::KeywordContainer) && has_colon => {
            Some(Block::Ordinary)
        }
        Token::KeywordIn if matches!(second, Some(Token::Identifier(name)) if name.eq_ignore_ascii_case("transaction")) => {
            Some(Block::Ordinary)
        }
        Token::Identifier(name) if name == "main" && second == Some(&Token::KeywordLoop) => {
            Some(Block::Ordinary)
        }
        Token::KeywordOn if has_colon => Some(Block::Ordinary),
        _ => None,
    }
}

/// An interface body starts only at the colon immediately after its name or
/// comma-separated `extends` names. A bare interface can have a later statement
/// on the same line, including one with a named-argument colon.
fn interface_has_body(tokens: &[TokenWithPosition]) -> bool {
    let mut header = tokens.iter().skip(2).map(|token| &token.token);
    if !matches!(header.next(), Some(Token::Identifier(_))) {
        return false;
    }
    let mut next = header.next();
    if next == Some(&Token::KeywordExtends) {
        loop {
            if !matches!(header.next(), Some(Token::Identifier(_))) {
                return false;
            }
            next = header.next();
            if next != Some(&Token::Comma) {
                break;
            }
        }
    }
    next == Some(&Token::Colon)
}
