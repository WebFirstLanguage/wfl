use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

const MAX_STEPS: u32 = 100_000;
const MAX_RECURSION_DEPTH: usize = 1000;

#[derive(Debug, Clone)]
pub enum PatternError {
    ParseError(String),
    RuntimeError(String),
    StepLimitExceeded,
    RecursionLimitExceeded,
}

impl std::fmt::Display for PatternError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatternError::ParseError(msg) => write!(f, "Pattern parse error: {msg}"),
            PatternError::RuntimeError(msg) => write!(f, "Pattern runtime error: {msg}"),
            PatternError::StepLimitExceeded => write!(f, "Pattern execution step limit exceeded"),
            PatternError::RecursionLimitExceeded => write!(f, "Pattern recursion limit exceeded"),
        }
    }
}

impl std::error::Error for PatternError {}

#[derive(Debug, Clone, PartialEq)]
pub enum CharClass {
    Digit,
    Letter,
    Whitespace,
    Any,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AnchorType {
    Start,
    End,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatternNode {
    Literal(String),
    CharClass(CharClass),
    Sequence(Vec<PatternNode>), // Critical: needed for proper IR structure
    Alt {
        alternatives: Vec<PatternNode>,
    },
    Rep {
        min: u32,
        max: u32,
        child: Box<PatternNode>,
    }, // min, max (u32::MAX = infinite), pattern
    Capture {
        name: String,
        child: Box<PatternNode>,
    }, // capture name, pattern
    Anchor(AnchorType),
}

#[derive(Debug, Clone)]
pub struct CompiledPattern {
    pub root: PatternNode,
    pub captures: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MatchResult {
    pub matched_text: String,
    pub captures: HashMap<String, String>,
    pub start: usize,
    pub end: usize,
}

impl CompiledPattern {
    pub fn new(root: PatternNode) -> Self {
        let captures = Self::extract_captures(&root);
        Self { root, captures }
    }

    fn extract_captures(node: &PatternNode) -> Vec<String> {
        let mut captures = Vec::new();
        Self::collect_captures(node, &mut captures);
        captures
    }

    fn collect_captures(node: &PatternNode, captures: &mut Vec<String>) {
        match node {
            PatternNode::Capture { name, child } => {
                captures.push(name.clone());
                Self::collect_captures(child, captures);
            }
            PatternNode::Sequence(nodes) => {
                for node in nodes {
                    Self::collect_captures(node, captures);
                }
            }
            PatternNode::Alt { alternatives } => {
                for alt in alternatives {
                    Self::collect_captures(alt, captures);
                }
            }
            PatternNode::Rep { child, .. } => {
                Self::collect_captures(child, captures);
            }
            _ => {}
        }
    }
}

pub fn parse_ir(ir_string: &str) -> Result<CompiledPattern, PatternError> {
    let root = parse_ir_node(ir_string.trim())?;
    Ok(CompiledPattern::new(root))
}

fn parse_ir_node(ir: &str) -> Result<PatternNode, PatternError> {
    if ir.starts_with("lit(") && ir.ends_with(')') {
        let content = &ir[4..ir.len() - 1];
        if content.starts_with('"') && content.ends_with('"') {
            let literal = content[1..content.len() - 1].replace("\\\"", "\"");
            Ok(PatternNode::Literal(literal))
        } else {
            Err(PatternError::ParseError(format!(
                "Invalid literal format: {ir}"
            )))
        }
    } else if ir.starts_with("class(") && ir.ends_with(')') {
        let class_name = &ir[6..ir.len() - 1];
        match class_name {
            "digit" => Ok(PatternNode::CharClass(CharClass::Digit)),
            "letter" => Ok(PatternNode::CharClass(CharClass::Letter)),
            "whitespace" => Ok(PatternNode::CharClass(CharClass::Whitespace)),
            "any" => Ok(PatternNode::CharClass(CharClass::Any)),
            _ => Err(PatternError::ParseError(format!(
                "Unknown character class: {class_name}"
            ))),
        }
    } else if ir.starts_with("seq(") && ir.ends_with(')') {
        let content = &ir[4..ir.len() - 1];
        let parts = parse_comma_separated(content)?;
        let mut nodes = Vec::new();
        for part in parts {
            nodes.push(parse_ir_node(&part)?);
        }
        Ok(PatternNode::Sequence(nodes))
    } else if ir.starts_with("alt(") && ir.ends_with(')') {
        let content = &ir[4..ir.len() - 1];
        let parts = parse_comma_separated(content)?;
        if parts.len() < 2 {
            return Err(PatternError::ParseError(
                "Alt requires at least 2 arguments".to_string(),
            ));
        }
        let mut alternatives = Vec::new();
        for part in parts {
            alternatives.push(parse_ir_node(&part)?);
        }
        Ok(PatternNode::Alt { alternatives })
    } else if ir.starts_with("rep(") && ir.ends_with(')') {
        let content = &ir[4..ir.len() - 1];
        let parts = parse_comma_separated(content)?;
        if parts.len() != 3 {
            return Err(PatternError::ParseError(
                "Rep requires exactly 3 arguments".to_string(),
            ));
        }

        let min: u32 = parts[0]
            .parse()
            .map_err(|_| PatternError::ParseError(format!("Invalid min count: {}", parts[0])))?;

        let max = if parts[1] == "inf" {
            u32::MAX
        } else {
            parts[1]
                .parse()
                .map_err(|_| PatternError::ParseError(format!("Invalid max count: {}", parts[1])))?
        };

        if min > max && max != u32::MAX {
            return Err(PatternError::ParseError(format!(
                "Invalid range: min {min} > max {max}"
            )));
        }

        let child = Box::new(parse_ir_node(&parts[2])?);
        Ok(PatternNode::Rep { min, max, child })
    } else if ir.starts_with("cap(") && ir.ends_with(')') {
        let content = &ir[4..ir.len() - 1];
        let parts = parse_comma_separated(content)?;
        if parts.len() != 2 {
            return Err(PatternError::ParseError(
                "Capture requires exactly 2 arguments".to_string(),
            ));
        }

        let name = if parts[0].starts_with('"') && parts[0].ends_with('"') {
            parts[0][1..parts[0].len() - 1].to_string()
        } else {
            return Err(PatternError::ParseError(format!(
                "Capture name must be quoted: {}",
                parts[0]
            )));
        };

        let child = Box::new(parse_ir_node(&parts[1])?);
        Ok(PatternNode::Capture { name, child })
    } else if ir.starts_with("opt(") && ir.ends_with(')') {
        let content = &ir[4..ir.len() - 1];
        let child = Box::new(parse_ir_node(content)?);
        Ok(PatternNode::Rep {
            min: 0,
            max: 1,
            child,
        })
    } else if ir.starts_with("anchor(") && ir.ends_with(')') {
        let anchor_type = &ir[7..ir.len() - 1];
        match anchor_type {
            "start" => Ok(PatternNode::Anchor(AnchorType::Start)),
            "end" => Ok(PatternNode::Anchor(AnchorType::End)),
            _ => Err(PatternError::ParseError(format!(
                "Unknown anchor type: {anchor_type}"
            ))),
        }
    } else {
        Err(PatternError::ParseError(format!(
            "Unknown IR function: {}",
            ir.split('(').next().unwrap_or(ir)
        )))
    }
}

fn parse_comma_separated(content: &str) -> Result<Vec<String>, PatternError> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut paren_depth = 0;
    let mut in_quotes = false;
    let mut escape_next = false;

    for ch in content.chars() {
        if escape_next {
            current.push(ch);
            escape_next = false;
            continue;
        }

        match ch {
            '\\' if in_quotes => {
                escape_next = true;
                current.push(ch);
            }
            '"' => {
                in_quotes = !in_quotes;
                current.push(ch);
            }
            '(' if !in_quotes => {
                paren_depth += 1;
                current.push(ch);
            }
            ')' if !in_quotes => {
                paren_depth -= 1;
                current.push(ch);
            }
            ',' if !in_quotes && paren_depth == 0 => {
                parts.push(current.trim().to_string());
                current.clear();
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if paren_depth > 0 {
        return Err(PatternError::ParseError(
            "Expected closing parenthesis".to_string(),
        ));
    }

    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }

    Ok(parts)
}

pub fn exec_match(
    pattern: &CompiledPattern,
    text: &str,
) -> Result<Option<MatchResult>, PatternError> {
    let mut steps = 0;
    let result = exec_match_with_steps(pattern, text, &mut steps);

    if let Some(ref match_result) = result {
        if should_match_entire_input(&pattern.root) && match_result.end != text.len() {
            return Ok(None);
        }
    }

    Ok(result)
}

fn should_match_entire_input(node: &PatternNode) -> bool {
    match node {
        PatternNode::Rep { max, .. } => *max != u32::MAX, // Bounded repetitions should match exactly
        PatternNode::Sequence(nodes) => nodes.iter().any(should_match_entire_input),
        PatternNode::Capture { child, .. } => should_match_entire_input(child),
        _ => false,
    }
}

pub fn exec_match_with_steps(
    pattern: &CompiledPattern,
    text: &str,
    steps: &mut u32,
) -> Option<MatchResult> {
    let mut captures = HashMap::new();
    for capture_name in &pattern.captures {
        captures.insert(capture_name.clone(), None);
    }

    for start_pos in 0..=text.len() {
        *steps += 1;
        if *steps > MAX_STEPS {
            return None; // Step limit exceeded
        }

        let mut local_captures = captures.clone();
        let mut recursion_stack = Vec::new();

        if match_at_position(
            &pattern.root,
            text,
            start_pos,
            &mut local_captures,
            steps,
            &mut recursion_stack,
        ) {
            let end_pos = find_match_end(&pattern.root, text, start_pos);
            return Some(MatchResult {
                matched_text: text[start_pos..end_pos].to_string(),
                captures: local_captures
                    .into_iter()
                    .filter_map(|(k, v)| v.map(|val| (k, val)))
                    .collect(),
                start: start_pos,
                end: end_pos,
            });
        }
    }

    None
}

fn match_at_position(
    node: &PatternNode,
    text: &str,
    pos: usize,
    captures: &mut HashMap<String, Option<String>>,
    steps: &mut u32,
    recursion_stack: &mut Vec<String>,
) -> bool {
    *steps += 1;
    if *steps > MAX_STEPS {
        return false;
    }

    if recursion_stack.len() > MAX_RECURSION_DEPTH {
        return false;
    }

    match node {
        PatternNode::Literal(literal) => {
            if pos + literal.len() <= text.len() {
                &text[pos..pos + literal.len()] == literal
            } else {
                false
            }
        }
        PatternNode::CharClass(class) => {
            if pos < text.len() {
                let ch = text.chars().nth(pos).unwrap();
                match class {
                    CharClass::Digit => ch.is_ascii_digit(),
                    CharClass::Letter => ch.is_alphabetic(),
                    CharClass::Whitespace => ch.is_whitespace(),
                    CharClass::Any => true,
                }
            } else {
                false
            }
        }
        PatternNode::Sequence(nodes) => {
            let mut current_pos = pos;
            for node in nodes {
                if !match_at_position(node, text, current_pos, captures, steps, recursion_stack) {
                    return false;
                }
                current_pos = find_match_end(node, text, current_pos);
            }
            true
        }
        PatternNode::Alt { alternatives } => {
            for alternative in alternatives {
                let mut alt_captures = captures.clone();
                let mut alt_stack = recursion_stack.clone();

                if match_at_position(
                    alternative,
                    text,
                    pos,
                    &mut alt_captures,
                    steps,
                    &mut alt_stack,
                ) {
                    *captures = alt_captures;
                    *recursion_stack = alt_stack;
                    return true;
                }
            }
            false
        }
        PatternNode::Rep { min, max, child } => {
            let mut match_count = 0;
            let mut current_pos = pos;

            loop {
                if *max != u32::MAX && match_count >= *max {
                    break;
                }

                if current_pos >= text.len() {
                    break;
                }

                if !match_at_position(child, text, current_pos, captures, steps, recursion_stack) {
                    break;
                }

                match_count += 1;
                let next_pos = find_match_end(child, text, current_pos);

                if next_pos == current_pos {
                    break; // Prevent infinite loop on zero-width matches
                }
                current_pos = next_pos;
            }

            match_count >= *min
        }
        PatternNode::Capture { name, child } => {
            recursion_stack.push(format!("capture:{name}"));
            let start_pos = pos;
            let result = match_at_position(child, text, pos, captures, steps, recursion_stack);

            if result {
                let end_pos = find_match_end(child, text, start_pos);
                if end_pos <= text.len() {
                    captures.insert(name.clone(), Some(text[start_pos..end_pos].to_string()));
                }
            }

            recursion_stack.pop();
            result
        }
        PatternNode::Anchor(anchor_type) => match anchor_type {
            AnchorType::Start => pos == 0,
            AnchorType::End => pos == text.len(),
        },
    }
}

fn find_match_end(node: &PatternNode, text: &str, start_pos: usize) -> usize {
    match node {
        PatternNode::Literal(literal) => start_pos + literal.len(),
        PatternNode::CharClass(_) => start_pos + 1,
        PatternNode::Sequence(nodes) => {
            let mut pos = start_pos;
            for node in nodes {
                pos = find_match_end(node, text, pos);
            }
            pos
        }
        PatternNode::Alt { alternatives } => {
            for alt in alternatives {
                let mut captures = HashMap::new();
                let mut steps = 0;
                let mut recursion_stack = Vec::new();
                if match_at_position(
                    alt,
                    text,
                    start_pos,
                    &mut captures,
                    &mut steps,
                    &mut recursion_stack,
                ) {
                    return find_match_end(alt, text, start_pos);
                }
            }
            start_pos
        }
        PatternNode::Rep { min, max, child } => {
            let mut pos = start_pos;

            let mut temp_pos = start_pos;
            let mut actual_matches = 0;

            while actual_matches < *max && temp_pos < text.len() {
                let mut captures = HashMap::new();
                let mut steps = 0;
                let mut recursion_stack = Vec::new();

                if match_at_position(
                    child,
                    text,
                    temp_pos,
                    &mut captures,
                    &mut steps,
                    &mut recursion_stack,
                ) {
                    let next_pos = find_match_end(child, text, temp_pos);
                    if next_pos == temp_pos {
                        break; // Prevent infinite loop on zero-width matches
                    }
                    temp_pos = next_pos;
                    actual_matches += 1;
                } else {
                    break;
                }
            }

            if actual_matches >= *min {
                pos = temp_pos;
            }

            pos
        }
        PatternNode::Capture { child, .. } => find_match_end(child, text, start_pos),
        PatternNode::Anchor(_) => start_pos,
    }
}

pub fn native_pattern_matches(
    args: Vec<Value>,
    line: usize,
    column: usize,
) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::new(
            "pattern_matches requires exactly 2 arguments".to_string(),
            line,
            column,
        ));
    }

    let text = match &args[0] {
        Value::Text(t) => t.as_ref(),
        _ => {
            return Err(RuntimeError::new(
                "First argument must be text".to_string(),
                line,
                column,
            ));
        }
    };

    let pattern = match &args[1] {
        Value::Pattern(p) => p.as_ref(),
        _ => {
            return Err(RuntimeError::new(
                "Second argument must be a pattern".to_string(),
                line,
                column,
            ));
        }
    };

    match exec_match(pattern, text) {
        Ok(Some(_)) => Ok(Value::Bool(true)),
        Ok(None) => Ok(Value::Bool(false)),
        Err(e) => Err(RuntimeError::new(
            format!("Pattern execution error: {e}"),
            line,
            column,
        )),
    }
}

pub fn native_pattern_find(
    args: Vec<Value>,
    line: usize,
    column: usize,
) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::new(
            "pattern_find requires exactly 2 arguments".to_string(),
            line,
            column,
        ));
    }

    let text = match &args[0] {
        Value::Text(t) => t.as_ref(),
        _ => {
            return Err(RuntimeError::new(
                "First argument must be text".to_string(),
                line,
                column,
            ));
        }
    };

    let pattern = match &args[1] {
        Value::Pattern(p) => p.as_ref(),
        _ => {
            return Err(RuntimeError::new(
                "Second argument must be a pattern".to_string(),
                line,
                column,
            ));
        }
    };

    match exec_match(pattern, text) {
        Ok(Some(result)) => {
            use std::rc::Rc;
            let mut map = HashMap::new();
            for (key, value) in result.captures {
                map.insert(key, Value::Text(Rc::from(value)));
            }
            Ok(Value::Object(Rc::new(RefCell::new(map))))
        }
        Ok(None) => Ok(Value::Null),
        Err(e) => Err(RuntimeError::new(
            format!("Pattern execution error: {e}"),
            line,
            column,
        )),
    }
}

pub fn native_pattern_replace(
    args: Vec<Value>,
    line: usize,
    column: usize,
) -> Result<Value, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::new(
            "pattern_replace requires exactly 3 arguments".to_string(),
            line,
            column,
        ));
    }

    let text = match &args[0] {
        Value::Text(t) => t.as_ref(),
        _ => {
            return Err(RuntimeError::new(
                "First argument must be text".to_string(),
                line,
                column,
            ));
        }
    };

    let pattern = match &args[1] {
        Value::Pattern(p) => p.as_ref(),
        _ => {
            return Err(RuntimeError::new(
                "Second argument must be a pattern".to_string(),
                line,
                column,
            ));
        }
    };

    let replacement = match &args[2] {
        Value::Text(t) => t.as_ref(),
        _ => {
            return Err(RuntimeError::new(
                "Third argument must be text".to_string(),
                line,
                column,
            ));
        }
    };

    match exec_match(pattern, text) {
        Ok(Some(result)) => {
            let mut new_text = text.to_string();
            new_text.replace_range(result.start..result.end, replacement);
            Ok(Value::Text(Rc::from(new_text)))
        }
        Ok(None) => Ok(Value::Text(Rc::from(text))),
        Err(e) => Err(RuntimeError::new(
            format!("Pattern execution error: {e}"),
            line,
            column,
        )),
    }
}

pub fn native_pattern_split(
    args: Vec<Value>,
    line: usize,
    column: usize,
) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::new(
            "pattern_split requires exactly 2 arguments".to_string(),
            line,
            column,
        ));
    }

    let text = match &args[0] {
        Value::Text(t) => t.as_ref(),
        _ => {
            return Err(RuntimeError::new(
                "First argument must be text".to_string(),
                line,
                column,
            ));
        }
    };

    let pattern = match &args[1] {
        Value::Pattern(p) => p.as_ref(),
        _ => {
            return Err(RuntimeError::new(
                "Second argument must be a pattern".to_string(),
                line,
                column,
            ));
        }
    };

    use std::rc::Rc;
    let mut parts = Vec::new();
    let mut last_end = 0;
    let mut search_pos = 0;

    while search_pos < text.len() {
        let remaining_text = &text[search_pos..];
        match exec_match(pattern, remaining_text) {
            Ok(Some(result)) => {
                let actual_start = search_pos + result.start;
                let actual_end = search_pos + result.end;

                if actual_start > last_end {
                    parts.push(Value::Text(Rc::from(&text[last_end..actual_start])));
                }

                last_end = actual_end;
                search_pos = actual_end;

                // Prevent infinite loop on zero-width matches
                if result.start == result.end {
                    search_pos += 1;
                }
            }
            Ok(None) => break,
            Err(e) => {
                return Err(RuntimeError::new(
                    format!("Pattern execution error: {e}"),
                    line,
                    column,
                ));
            }
        }
    }

    if last_end < text.len() {
        parts.push(Value::Text(Rc::from(&text[last_end..])));
    }

    if parts.is_empty() {
        parts.push(Value::Text(Rc::from(text)));
    }

    Ok(Value::List(Rc::new(RefCell::new(parts))))
}

pub fn register(env: &mut Environment) {
    crate::stdlib::legacy_pattern::register(env);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::value::Value;
    use std::rc::Rc;

    #[test]
    fn test_ir_parse_literal() {
        let pattern = parse_ir("lit(\"abc\")").unwrap();
        assert_eq!(pattern.root, PatternNode::Literal("abc".to_string()));
    }

    #[test]
    fn test_ir_parse_digit_class() {
        let pattern = parse_ir("class(digit)").unwrap();
        assert_eq!(pattern.root, PatternNode::CharClass(CharClass::Digit));
    }

    #[test]
    fn test_match_literal_abc() {
        let pattern = parse_ir("lit(\"abc\")").unwrap();
        let result = exec_match(&pattern, "abc").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().matched_text, "abc");
    }

    #[test]
    fn test_match_digit() {
        let pattern = parse_ir("class(digit)").unwrap();
        let result = exec_match(&pattern, "5").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().matched_text, "5");
    }

    #[test]
    fn test_native_pattern_matches_basic() {
        let args = vec![
            Value::Text(Rc::from("abc")),
            Value::Pattern(Rc::new(parse_ir("lit(\"abc\")").unwrap())),
        ];
        let result = native_pattern_matches(args, 0, 0).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn test_native_pattern_find_with_captures() {
        let args = vec![
            Value::Text(Rc::from("5x")),
            Value::Pattern(Rc::new(
                parse_ir("seq(cap(\"digit\",class(digit)),cap(\"letter\",class(letter)))").unwrap(),
            )),
        ];
        let result = native_pattern_find(args, 0, 0).unwrap();

        if let Value::Object(obj_rc) = result {
            let obj = obj_rc.borrow();
            if let Value::Text(digit) = obj.get("digit").unwrap() {
                assert_eq!(digit.to_string(), "5");
            } else {
                panic!("Expected digit to be a text value");
            }
        } else {
            panic!("Expected result to be an object");
        }
    }

    #[test]
    fn test_performance_regression_20_optional_groups() {
        use std::time::Instant;

        let mut pattern_ir = "seq(".to_string();
        for i in 0..20 {
            if i > 0 {
                pattern_ir.push(',');
            }
            pattern_ir.push_str("opt(class(letter))");
        }
        pattern_ir.push(')');

        let pattern = parse_ir(&pattern_ir).unwrap();

        let large_input = "a".repeat(2048);

        let start = Instant::now();
        let _result = exec_match(&pattern, &large_input).unwrap();
        let duration = start.elapsed();

        assert!(
            duration.as_millis() < 200,
            "Pattern matching took {}ms, expected < 200ms",
            duration.as_millis()
        );
    }

    #[test]
    fn test_quantifier_one_or_more() {
        let pattern = parse_ir("rep(1,inf,class(digit))").unwrap();
        let result = exec_match(&pattern, "123").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().matched_text, "123");
    }

    #[test]
    fn test_quantifier_optional() {
        let pattern = parse_ir("opt(class(digit))").unwrap();
        let result = exec_match(&pattern, "").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().matched_text, "");
    }

    #[test]
    fn test_alternation() {
        let pattern = parse_ir("alt(class(digit),class(letter))").unwrap();
        let result = exec_match(&pattern, "5").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().matched_text, "5");
    }

    #[test]
    fn test_anchors() {
        let pattern = parse_ir("seq(anchor(start),lit(\"abc\"))").unwrap();
        let result = exec_match(&pattern, "abc").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().matched_text, "abc");
    }
}
