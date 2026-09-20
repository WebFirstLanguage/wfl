//! Token-based source layout shared by linting and source-preserving fixes.

use crate::lexer::lex_wfl_with_positions;
use crate::lexer::token::{Token, TokenWithPosition};
use crate::parser::ast::{Assertion, Expression, Literal, Program, Statement};
use std::collections::HashSet;
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
    /// A validated body whose `end` consumes a following grammar keyword.
    SuffixedEnd,
    /// Container methods, containers, interfaces, and instance initializers
    /// consume only `end`; a following keyword belongs to the next statement.
    BareEnd,
    Check,
    PostconditionRepeat,
    Route,
    RouteArm,
}

/// The parsed program distinguishes expression operands from identically
/// spelled block words. Keep the token scanner for physical layout, since some
/// statement locations are legacy end positions and cannot identify headers.
#[derive(Default)]
struct SourceRoles {
    operands: HashSet<usize>,
    body_headers: HashSet<usize>,
}

impl SourceRoles {
    /// Traverse the existing AST without reparsing or recursing on user input.
    /// A source-spelling check excludes synthetic route helper variables whose
    /// locations point at real `when` headers instead of variable references.
    fn new(source: &str, tokens: &[TokenWithPosition], program: &Program) -> Self {
        let mut roles = Self::default();
        let mut statements: Vec<_> = program.statements.iter().collect();
        let mut expressions = Vec::new();
        while let Some(statement) = statements.pop() {
            statements.extend(super::statement_children(statement));
            if let Statement::WaitForStatement { inner, .. } = statement {
                statements.push(inner);
            }
            // Ordinary `on source event` registers a bodyless event handler;
            // only the websocket form consumes a body and `end on` today.
            if let Statement::WebSocketHandlerStatement { line, column, .. } = statement
                && let Some(token) = token_at(tokens, *line, *column)
                && token.token == Token::KeywordOn
            {
                roles.body_headers.insert(token.byte_start);
            }
            // `main` can also name an action or member in an expression. This
            // statement's location reliably identifies the actual loop header.
            if let Statement::MainLoop { line, column, .. } = statement
                && let Some(token) = token_at(tokens, *line, *column)
                && matches!(&token.token, Token::Identifier(name) if name == "main")
            {
                roles.body_headers.insert(token.byte_start);
            }
            // Pattern find/replace expressions may contain `in transaction`;
            // only this parsed statement's opening `in` introduces a body.
            if let Statement::TransactionStatement { line, column, .. } = statement
                && let Some(token) = token_at(tokens, *line, *column)
                && token.token == Token::KeywordIn
            {
                roles.body_headers.insert(token.byte_start);
            }
            // Pattern lookarounds use `check` too. Source-token verification
            // also excludes conditional nodes synthesized from route arms.
            if let Statement::IfStatement { line, column, .. } = statement
                && let Some(token) = token_at(tokens, *line, *column)
                && token.token == Token::KeywordCheck
            {
                roles.body_headers.insert(token.byte_start);
            }
            statement_expressions(statement, &mut expressions);
        }
        while let Some(expression) = expressions.pop() {
            if let Expression::Variable(name, line, column) = expression
                && let Some(token) = token_at(tokens, *line, *column)
            {
                let matches_source = match &token.token {
                    Token::Identifier(identifier) => identifier == name,
                    _ => source[token.byte_start..token.byte_end].eq_ignore_ascii_case(name),
                };
                if matches_source {
                    roles.operands.insert(token.byte_start);
                }
            }
            expression_children(expression, &mut expressions);
        }
        roles
    }
}

/// AST and token locations come from the same positioned lexer. Binary search
/// avoids repeatedly scanning the source for each expression in large programs.
fn token_at(
    tokens: &[TokenWithPosition],
    line: usize,
    column: usize,
) -> Option<&TokenWithPosition> {
    tokens
        .binary_search_by_key(&(line, column), |token| (token.line, token.column))
        .ok()
        .map(|index| &tokens[index])
}

/// Add all expression roots carried by a statement, including metadata defaults
/// and validation expressions. Exhaustive variants make new AST forms visible
/// to this visitor at compile time; nested statements are handled separately.
fn statement_expressions<'a>(statement: &'a Statement, pending: &mut Vec<&'a Expression>) {
    match statement {
        Statement::VariableDeclaration { value, .. }
        | Statement::Assignment { value, .. }
        | Statement::DisplayStatement { value, .. }
        | Statement::AddToListStatement { value, .. }
        | Statement::RemoveFromListStatement { value, .. } => pending.push(value),
        Statement::IfStatement { condition, .. }
        | Statement::SingleLineIf { condition, .. }
        | Statement::WhileLoop { condition, .. }
        | Statement::RepeatWhileLoop { condition, .. }
        | Statement::RepeatUntilLoop { condition, .. } => pending.push(condition),
        Statement::ForEachLoop { collection, .. } => pending.push(collection),
        Statement::CountLoop {
            start, end, step, ..
        } => {
            pending.extend([start, end]);
            pending.extend(step);
        }
        Statement::ActionDefinition { parameters, .. }
        | Statement::EventDefinition { parameters, .. } => {
            pending.extend(
                parameters
                    .iter()
                    .filter_map(|parameter| parameter.default_value.as_ref()),
            );
        }
        Statement::ReturnStatement { value, .. }
        | Statement::CreateDateStatement { value, .. }
        | Statement::CreateTimeStatement { value, .. } => pending.extend(value),
        Statement::ExpressionStatement { expression, .. } => pending.push(expression),
        Statement::OpenFileStatement { path, .. }
        | Statement::ReadFileStatement { path, .. }
        | Statement::CreateDirectoryStatement { path, .. }
        | Statement::DeleteFileStatement { path, .. }
        | Statement::DeleteDirectoryStatement { path, .. }
        | Statement::LoadModuleStatement { path, .. }
        | Statement::IncludeStatement { path, .. } => pending.push(path),
        Statement::WriteFileStatement { file, content, .. }
        | Statement::WriteToStatement { file, content, .. } => pending.extend([file, content]),
        Statement::CloseFileStatement { file, .. } => pending.push(file),
        Statement::OpenDatabaseStatement { url, .. } | Statement::HttpGetStatement { url, .. } => {
            pending.push(url)
        }
        Statement::DatabaseQueryStatement {
            db,
            sql,
            parameters,
            ..
        } => {
            pending.extend([db, sql]);
            pending.extend(parameters);
        }
        Statement::CloseDatabaseStatement { db, .. }
        | Statement::TransactionStatement { db, .. } => pending.push(db),
        Statement::CreateFileStatement { path, content, .. } => pending.extend([path, content]),
        Statement::ExecuteCommandStatement {
            command,
            arguments,
            directory,
            ..
        }
        | Statement::SpawnProcessStatement {
            command,
            arguments,
            directory,
            ..
        } => {
            pending.push(command);
            pending.extend(arguments);
            pending.extend(directory);
        }
        Statement::ExecuteFileStatement { path, request, .. } => {
            pending.push(path);
            pending.extend(request);
        }
        Statement::ReadProcessOutputStatement { process_id, .. }
        | Statement::KillProcessStatement { process_id, .. } => pending.push(process_id),
        Statement::WaitForProcessStatement {
            process_id,
            timeout,
            ..
        } => {
            pending.push(process_id);
            pending.extend(timeout);
        }
        Statement::WaitForDurationStatement { duration, .. } => pending.push(duration),
        Statement::HttpPostStatement { url, data, .. } => pending.extend([url, data]),
        Statement::HttpRequestStatement {
            url,
            method,
            headers,
            body,
            ..
        }
        | Statement::HttpStreamStatement {
            url,
            method,
            headers,
            body,
            ..
        } => {
            pending.push(url);
            pending.extend(method.iter().chain(headers).chain(body));
        }
        Statement::WaitForNextChunkStatement { source, .. }
        | Statement::WaitForNextLineStatement { source, .. } => pending.push(source),
        Statement::PushStatement { list, value, .. } => pending.extend([list, value]),
        Statement::CreateListStatement { initial_values, .. } => pending.extend(initial_values),
        Statement::MapCreation { entries, .. } => {
            pending.extend(entries.iter().map(|(_, value)| value))
        }
        Statement::ContainerDefinition {
            properties,
            static_properties,
            events,
            ..
        } => {
            for property in properties.iter().chain(static_properties) {
                pending.extend(&property.default_value);
                pending.extend(
                    property
                        .validation_rules
                        .iter()
                        .flat_map(|rule| &rule.parameters),
                );
            }
            pending.extend(
                events
                    .iter()
                    .flat_map(|event| &event.parameters)
                    .filter_map(|parameter| parameter.default_value.as_ref()),
            );
        }
        Statement::ContainerInstantiation {
            arguments,
            property_initializers,
            ..
        } => {
            pending.extend(arguments.iter().map(|argument| &argument.value));
            pending.extend(property_initializers.iter().map(|property| &property.value));
        }
        Statement::InterfaceDefinition {
            required_actions, ..
        } => {
            pending.extend(
                required_actions
                    .iter()
                    .flat_map(|action| &action.parameters)
                    .filter_map(|parameter| parameter.default_value.as_ref()),
            );
        }
        Statement::EventTrigger { arguments, .. }
        | Statement::ParentMethodCall { arguments, .. } => {
            pending.extend(arguments.iter().map(|argument| &argument.value))
        }
        Statement::EventHandler { event_source, .. } => pending.push(event_source),
        Statement::ListenStatement {
            port,
            tls,
            redirect_to_port,
            ..
        } => {
            pending.push(port);
            pending.extend(redirect_to_port);
            if let Some(tls) = tls {
                pending.extend(tls.cert_path.iter().chain(&tls.key_path));
            }
        }
        Statement::WaitForRequestStatement {
            server, timeout, ..
        } => {
            pending.push(server);
            pending.extend(timeout);
        }
        Statement::RespondStatement {
            request,
            content,
            status,
            content_type,
            headers,
            ..
        } => {
            pending.extend([request, content]);
            pending.extend(status.iter().chain(content_type).chain(headers));
        }
        Statement::StartStreamingResponseStatement {
            request,
            status,
            content_type,
            headers,
            ..
        } => {
            pending.push(request);
            pending.extend(status.iter().chain(content_type).chain(headers));
        }
        Statement::StreamWriteStatement {
            value,
            target,
            fallback_content,
            ..
        } => {
            pending.extend([value, target]);
            pending.extend(fallback_content.as_deref());
        }
        Statement::FlushStreamStatement {
            target,
            action_fallback,
            ..
        } => {
            pending.push(target);
            pending.extend(action_fallback);
        }
        Statement::ListenWebSocketStatement { port, .. } => pending.push(port),
        Statement::WebSocketHandlerStatement { server, .. }
        | Statement::StopAcceptingConnectionsStatement { server, .. }
        | Statement::CloseServerStatement { server, .. } => pending.push(server),
        Statement::SendWebSocketMessageStatement {
            message, target, ..
        } => pending.extend([message, target]),
        Statement::BroadcastWebSocketMessageStatement {
            message, server, ..
        } => pending.extend([message, server]),
        Statement::WriteContentStatement {
            content, target, ..
        }
        | Statement::WriteBinaryStatement {
            content, target, ..
        } => pending.extend([content, target]),
        Statement::ExpectStatement {
            subject, assertion, ..
        } => {
            pending.push(subject);
            match assertion {
                Assertion::Equal(value)
                | Assertion::Be(value)
                | Assertion::GreaterThan(value)
                | Assertion::LessThan(value)
                | Assertion::Contain(value)
                | Assertion::HaveLength(value) => pending.push(value),
                Assertion::BeYes
                | Assertion::BeNo
                | Assertion::Exist
                | Assertion::BeEmpty
                | Assertion::BeOfType(_) => {}
            }
        }
        Statement::ExitStatement { code, .. } => pending.extend(code),
        Statement::ForeverLoop { .. }
        | Statement::MainLoop { .. }
        | Statement::BreakStatement { .. }
        | Statement::ContinueStatement { .. }
        | Statement::ExportStatement { .. }
        | Statement::WaitForStatement { .. }
        | Statement::TryStatement { .. }
        | Statement::ClearListStatement { .. }
        | Statement::PatternDefinition { .. }
        | Statement::RegisterSignalHandlerStatement { .. }
        | Statement::DescribeBlock { .. }
        | Statement::TestBlock { .. } => {}
    }
}

/// Add immediate expression children to an explicit worklist, preserving
/// operand positions inside lists, calls, operators, and nested I/O forms.
fn expression_children<'a>(expression: &'a Expression, pending: &mut Vec<&'a Expression>) {
    match expression {
        Expression::Literal(Literal::List(values), ..) => pending.extend(values),
        Expression::Literal(..)
        | Expression::Variable(..)
        | Expression::StaticMemberAccess { .. }
        | Expression::CurrentTimeMilliseconds { .. }
        | Expression::CurrentTimeFormatted { .. } => {}
        Expression::BinaryOperation { left, right, .. }
        | Expression::Concatenation { left, right, .. } => {
            pending.extend([left.as_ref(), right.as_ref()])
        }
        Expression::UnaryOperation { expression, .. }
        | Expression::AwaitExpression { expression, .. } => pending.push(expression),
        Expression::FunctionCall {
            function,
            arguments,
            ..
        } => {
            pending.push(function);
            pending.extend(arguments.iter().map(|argument| &argument.value));
        }
        Expression::MemberAccess { object, .. } | Expression::PropertyAccess { object, .. } => {
            pending.push(object)
        }
        Expression::ActionCall { arguments, .. } => {
            pending.extend(arguments.iter().map(|argument| &argument.value))
        }
        Expression::IndexAccess {
            collection, index, ..
        } => pending.extend([collection.as_ref(), index.as_ref()]),
        Expression::PatternMatch { text, pattern, .. }
        | Expression::PatternFind { text, pattern, .. }
        | Expression::PatternSplit { text, pattern, .. } => {
            pending.extend([text.as_ref(), pattern.as_ref()])
        }
        Expression::PatternReplace {
            text,
            pattern,
            replacement,
            ..
        } => pending.extend([text.as_ref(), pattern.as_ref(), replacement.as_ref()]),
        Expression::StringSplit {
            text, delimiter, ..
        } => pending.extend([text.as_ref(), delimiter.as_ref()]),
        Expression::MethodCall {
            object, arguments, ..
        } => {
            pending.push(object);
            pending.extend(arguments.iter().map(|argument| &argument.value));
        }
        Expression::HeaderAccess { request, .. } => pending.push(request),
        Expression::FileExists { path, .. }
        | Expression::DirectoryExists { path, .. }
        | Expression::ListFiles { path, .. } => pending.push(path),
        Expression::ReadContent { file_handle, .. }
        | Expression::ReadBinaryContent { file_handle, .. }
        | Expression::FileSizeOf { file_handle, .. } => pending.push(file_handle),
        Expression::ReadBinaryN {
            file_handle, count, ..
        } => pending.extend([file_handle.as_ref(), count.as_ref()]),
        Expression::ListFilesRecursive {
            path, extensions, ..
        } => {
            pending.push(path);
            pending.extend(extensions.iter().flatten());
        }
        Expression::ListFilesFiltered {
            path, extensions, ..
        } => {
            pending.push(path);
            pending.extend(extensions);
        }
        Expression::ProcessRunning { process_id, .. } => pending.push(process_id),
        Expression::DatabaseQuery {
            db,
            sql,
            parameters,
            ..
        } => {
            pending.extend([db.as_ref(), sql.as_ref()]);
            pending.extend(parameters.as_deref());
        }
    }
}

impl SourceLayout {
    /// Derive physical layout using the AST parsed from this same source to
    /// distinguish expression operands from block and branch keywords.
    pub fn new(source: &str, program: &Program) -> Self {
        let tokens = lex_wfl_with_positions(source);
        let roles = SourceRoles::new(source, &tokens, program);
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
                (!line_tokens.is_empty()).then(|| line_depth(line_tokens, &mut stack, &roles));
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

    /// Check whether a proposed source edit intersects an original string token.
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
fn line_depth(tokens: &[TokenWithPosition], stack: &mut Vec<Block>, roles: &SourceRoles) -> usize {
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
        if roles.operands.contains(&tokens[index].byte_start) {
            index += 1;
            continue;
        }
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
            let closed = stack.pop();
            if index == 0 {
                depth = stack.len();
            }
            index += 1;
            // Only the closed owner can claim a suffix. For example, after
            // a method's bare `end`, `action second:` opens the next method.
            if matches!(
                closed,
                Some(Block::SuffixedEnd | Block::Check | Block::Route)
            ) && tokens
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
            let is_check_branch = stack.last() == Some(&Block::Check);
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
            // Only a check-owned `otherwise check if` shares its terminator.
            // Bare-if and route branches instead contain a separately closed
            // check. Scan onward so a later colon cannot hide an inline body.
            let is_chained_check = is_check_branch
                && matches!(token, Token::KeywordOtherwise)
                && tokens.get(index + 1).map(|token| &token.token) == Some(&Token::KeywordCheck)
                && tokens.get(index + 2).map(|token| &token.token) == Some(&Token::KeywordIf);
            index += if is_chained_check { 3 } else { 1 };
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
        let needs_body_role = matches!(
            token,
            Token::KeywordOn | Token::KeywordIn | Token::KeywordCheck
        ) || matches!(token, Token::Identifier(name) if name == "main");
        let is_body_header =
            !needs_body_role || roles.body_headers.contains(&tokens[index].byte_start);
        if let Some(block) = is_body_header
            .then(|| opened_block(&tokens[index..], has_colon))
            .flatten()
        {
            stack.push(block);
            // Consume compound opener words only. Operand roles disambiguate
            // the remaining header, and an actual body can start on this line.
            index += if matches!(
                token,
                Token::KeywordCheck | Token::KeywordDefine | Token::KeywordStatic
            ) {
                2
            } else {
                1
            };
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
        Token::KeywordCheck => Some(Block::Check),
        Token::KeywordRepeat if second == Some(&Token::Colon) => Some(Block::PostconditionRepeat),
        Token::KeywordIf
        | Token::KeywordRepeat
        | Token::KeywordTry
        | Token::KeywordDescribe
        | Token::KeywordTest
        | Token::KeywordSetup
        | Token::KeywordTeardown => Some(Block::SuffixedEnd),
        Token::KeywordAction => Some(Block::BareEnd),
        Token::KeywordFor if second == Some(&Token::KeywordEach) => Some(Block::SuffixedEnd),
        Token::KeywordCount if second == Some(&Token::KeywordFrom) => Some(Block::SuffixedEnd),
        Token::KeywordDefine if second == Some(&Token::KeywordAction) => Some(Block::SuffixedEnd),
        Token::KeywordStatic if second == Some(&Token::KeywordAction) => Some(Block::BareEnd),
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
            .then_some(Block::BareEnd)
        }
        Token::KeywordCreate if second == Some(&Token::KeywordInterface) => {
            interface_has_body(tokens).then_some(Block::BareEnd)
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
            .then_some(Block::SuffixedEnd)
        }
        Token::KeywordCreate if second == Some(&Token::KeywordContainer) && has_colon => {
            Some(Block::BareEnd)
        }
        Token::KeywordIn if matches!(second, Some(Token::Identifier(name)) if name.eq_ignore_ascii_case("transaction")) => {
            Some(Block::SuffixedEnd)
        }
        Token::Identifier(name) if name == "main" && second == Some(&Token::KeywordLoop) => {
            Some(Block::SuffixedEnd)
        }
        Token::KeywordOn if has_colon => Some(Block::SuffixedEnd),
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
