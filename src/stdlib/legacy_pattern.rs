use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use regex::Regex;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum PatternPart {
    Literal(String),
    Digits {
        min: usize,
        max: Option<usize>,
    },
    Letters {
        min: usize,
        max: Option<usize>,
    },
    Whitespace {
        min: usize,
        max: Option<usize>,
    },
    Placeholder(String),
    Optional(Box<PatternPart>),
    OneOrMore(Box<PatternPart>),
    Exactly {
        count: usize,
        part: Box<PatternPart>,
    },
    Between {
        min: usize,
        max: usize,
        part: Box<PatternPart>,
    },
    Sequence(Vec<PatternPart>),
    Alternation(Vec<PatternPart>),
    BeginsWith(Box<PatternPart>),
    EndsWith(Box<PatternPart>),
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Pattern {
    parts: Vec<PatternPart>,
    source: String,
    regex: Option<Regex>,
    capture_names: Vec<String>,
}

impl Pattern {
    pub fn parse(pattern_str: &str) -> Result<Self, String> {
        let mut parts = Vec::new();
        let mut capture_names = Vec::new();
        let mut regex_str = String::new();
        let mut current_pos = 0;

        while current_pos < pattern_str.len() {
            let remaining = &pattern_str[current_pos..];

            if remaining.starts_with('{') {
                let end_pos = remaining
                    .find('}')
                    .ok_or_else(|| "Unclosed placeholder".to_string())?;

                let placeholder_content = &remaining[1..end_pos];
                let placeholder_name = placeholder_content.trim();

                capture_names.push(placeholder_name.to_string());

                regex_str.push_str(&format!("(?P<{}>.*?)", placeholder_name));

                parts.push(PatternPart::Placeholder(placeholder_name.to_string()));
                current_pos += end_pos + 1;
                continue;
            }

            if remaining.starts_with("digit") {
                current_pos += 5;

                let is_plural = current_pos < pattern_str.len()
                    && &pattern_str[current_pos..current_pos + 1] == "s";
                if is_plural {
                    current_pos += 1;
                }

                if let Some((min, max, new_pos)) = parse_quantifier(pattern_str, current_pos) {
                    current_pos = new_pos;

                    parts.push(PatternPart::Digits { min, max });
                    regex_str.push_str(&format!(
                        "\\d{{{},{}}}",
                        min,
                        max.map_or("".to_string(), |m| m.to_string())
                    ));
                } else if is_plural {
                    parts.push(PatternPart::Digits { min: 1, max: None });
                    regex_str.push_str("\\d+");
                } else {
                    parts.push(PatternPart::Digits {
                        min: 1,
                        max: Some(1),
                    });
                    regex_str.push_str("\\d");
                }
                continue;
            }

            if remaining.starts_with("letter") {
                current_pos += 6;

                let is_plural = current_pos < pattern_str.len()
                    && &pattern_str[current_pos..current_pos + 1] == "s";
                if is_plural {
                    current_pos += 1;
                }

                if let Some((min, max, new_pos)) = parse_quantifier(pattern_str, current_pos) {
                    current_pos = new_pos;

                    parts.push(PatternPart::Letters { min, max });
                    regex_str.push_str(&format!(
                        "[a-zA-Z]{{{},{}}}",
                        min,
                        max.map_or("".to_string(), |m| m.to_string())
                    ));
                } else if is_plural {
                    parts.push(PatternPart::Letters { min: 1, max: None });
                    regex_str.push_str("[a-zA-Z]+");
                } else {
                    parts.push(PatternPart::Letters {
                        min: 1,
                        max: Some(1),
                    });
                    regex_str.push_str("[a-zA-Z]");
                }
                continue;
            }

            if remaining.starts_with("whitespace") {
                current_pos += 10;

                if let Some((min, max, new_pos)) = parse_quantifier(pattern_str, current_pos) {
                    current_pos = new_pos;

                    parts.push(PatternPart::Whitespace { min, max });
                    regex_str.push_str(&format!(
                        "\\s{{{},{}}}",
                        min,
                        max.map_or("".to_string(), |m| m.to_string())
                    ));
                } else {
                    parts.push(PatternPart::Whitespace { min: 1, max: None });
                    regex_str.push_str("\\s+");
                }
                continue;
            }

            if remaining.starts_with("optional ") {
                current_pos += 9;

                while current_pos < pattern_str.len()
                    && pattern_str[current_pos..current_pos + 1].trim().is_empty()
                {
                    current_pos += 1;
                }

                let (part, part_regex, new_pos) = parse_part(pattern_str, current_pos)?;
                current_pos = new_pos;

                parts.push(PatternPart::Optional(Box::new(part)));
                regex_str.push_str(&format!("(?:{})?", part_regex));
                continue;
            }

            if remaining.starts_with("one or more ") {
                current_pos += 12;

                while current_pos < pattern_str.len()
                    && pattern_str[current_pos..current_pos + 1].trim().is_empty()
                {
                    current_pos += 1;
                }

                let (part, part_regex, new_pos) = parse_part(pattern_str, current_pos)?;
                current_pos = new_pos;

                parts.push(PatternPart::OneOrMore(Box::new(part)));
                regex_str.push_str(&format!("(?:{})+", part_regex));
                continue;
            }

            if remaining.starts_with("exactly ") {
                current_pos += 8;

                let num_start = current_pos;
                while current_pos < pattern_str.len()
                    && pattern_str[current_pos..current_pos + 1]
                        .chars()
                        .next()
                        .unwrap()
                        .is_ascii_digit()
                {
                    current_pos += 1;
                }

                if num_start == current_pos {
                    return Err("Expected number after 'exactly'".to_string());
                }

                let count = pattern_str[num_start..current_pos]
                    .parse::<usize>()
                    .map_err(|_| "Invalid number after 'exactly'".to_string())?;

                while current_pos < pattern_str.len()
                    && pattern_str[current_pos..current_pos + 1].trim().is_empty()
                {
                    current_pos += 1;
                }

                let (part, part_regex, new_pos) = parse_part(pattern_str, current_pos)?;
                current_pos = new_pos;

                parts.push(PatternPart::Exactly {
                    count,
                    part: Box::new(part),
                });
                regex_str.push_str(&format!("(?:{}){{{}}}", part_regex, count));
                continue;
            }

            if remaining.starts_with("between ") {
                current_pos += 8;

                let num_start = current_pos;
                while current_pos < pattern_str.len()
                    && pattern_str[current_pos..current_pos + 1]
                        .chars()
                        .next()
                        .unwrap()
                        .is_ascii_digit()
                {
                    current_pos += 1;
                }

                if num_start == current_pos {
                    return Err("Expected number after 'between'".to_string());
                }

                let min = pattern_str[num_start..current_pos]
                    .parse::<usize>()
                    .map_err(|_| "Invalid number after 'between'".to_string())?;

                if !remaining[current_pos - num_start..].starts_with(" and ") {
                    return Err("Expected 'and' after first number in 'between'".to_string());
                }
                current_pos += 5; // Skip " and "

                let num_start = current_pos;
                while current_pos < pattern_str.len()
                    && pattern_str[current_pos..current_pos + 1]
                        .chars()
                        .next()
                        .unwrap()
                        .is_ascii_digit()
                {
                    current_pos += 1;
                }

                if num_start == current_pos {
                    return Err("Expected number after 'and' in 'between'".to_string());
                }

                let max = pattern_str[num_start..current_pos]
                    .parse::<usize>()
                    .map_err(|_| "Invalid number after 'and' in 'between'".to_string())?;

                while current_pos < pattern_str.len()
                    && pattern_str[current_pos..current_pos + 1].trim().is_empty()
                {
                    current_pos += 1;
                }

                let (part, part_regex, new_pos) = parse_part(pattern_str, current_pos)?;
                current_pos = new_pos;

                parts.push(PatternPart::Between {
                    min,
                    max,
                    part: Box::new(part),
                });
                regex_str.push_str(&format!("(?:{}){{{},{}}}", part_regex, min, max));
                continue;
            }

            if remaining.starts_with("begins with ") {
                current_pos += 12;

                while current_pos < pattern_str.len()
                    && pattern_str[current_pos..current_pos + 1].trim().is_empty()
                {
                    current_pos += 1;
                }

                let (part, part_regex, new_pos) = parse_part(pattern_str, current_pos)?;
                current_pos = new_pos;

                parts.push(PatternPart::BeginsWith(Box::new(part)));
                regex_str = format!("^{}{}", part_regex, regex_str);
                continue;
            }

            if remaining.starts_with("ends with ") {
                current_pos += 10;

                while current_pos < pattern_str.len()
                    && pattern_str[current_pos..current_pos + 1].trim().is_empty()
                {
                    current_pos += 1;
                }

                let (part, part_regex, new_pos) = parse_part(pattern_str, current_pos)?;
                current_pos = new_pos;

                parts.push(PatternPart::EndsWith(Box::new(part)));
                regex_str.push_str(&format!("{}$", part_regex));
                continue;
            }

            if remaining.starts_with("or ") {
                current_pos += 3;

                while current_pos < pattern_str.len()
                    && pattern_str[current_pos..current_pos + 1].trim().is_empty()
                {
                    current_pos += 1;
                }

                let (part, part_regex, new_pos) = parse_part(pattern_str, current_pos)?;
                current_pos = new_pos;

                if let Some(PatternPart::Alternation(alts)) = parts.last_mut() {
                    alts.push(part);

                    regex_str.push_str(&format!("|{}", part_regex));
                } else if let Some(prev_part) = parts.pop() {
                    let prev_regex = regex_str.clone();
                    regex_str = format!("(?:{}|{})", prev_regex, part_regex);

                    parts.push(PatternPart::Alternation(vec![prev_part, part]));
                } else {
                    return Err("'or' without preceding pattern part".to_string());
                }
                continue;
            }

            let c = pattern_str[current_pos..current_pos + 1]
                .chars()
                .next()
                .unwrap();
            parts.push(PatternPart::Literal(c.to_string()));

            if "\\.*+?()[]{}|^$".contains(c) {
                regex_str.push('\\');
            }
            regex_str.push(c);

            current_pos += 1;
        }

        let regex = Regex::new(&regex_str).map_err(|e| format!("Invalid regex: {}", e))?;

        Ok(Pattern {
            parts,
            source: pattern_str.to_string(),
            regex: Some(regex),
            capture_names,
        })
    }

    pub fn matches(&self, text: &str) -> bool {
        if let Some(regex) = &self.regex {
            regex.is_match(text)
        } else {
            false
        }
    }

    pub fn find(&self, text: &str) -> Option<HashMap<String, String>> {
        if let Some(regex) = &self.regex {
            if let Some(captures) = regex.captures(text) {
                let mut result = HashMap::new();

                for name in &self.capture_names {
                    if let Some(m) = captures.name(name) {
                        result.insert(name.clone(), m.as_str().to_string());
                    }
                }

                if !result.is_empty() {
                    return Some(result);
                }
            }
        }

        None
    }

    pub fn replace(&self, text: &str, replacement: &str) -> String {
        if let Some(regex) = &self.regex {
            regex.replace_all(text, replacement).to_string()
        } else {
            text.to_string()
        }
    }

    pub fn split(&self, text: &str) -> Vec<String> {
        if let Some(regex) = &self.regex {
            regex.split(text).map(|s| s.to_string()).collect()
        } else {
            vec![text.to_string()]
        }
    }
}

fn parse_quantifier(pattern_str: &str, pos: usize) -> Option<(usize, Option<usize>, usize)> {
    if pos >= pattern_str.len() {
        return None;
    }

    let mut current_pos = pos;
    while current_pos < pattern_str.len()
        && pattern_str[current_pos..current_pos + 1].trim().is_empty()
    {
        current_pos += 1;
    }

    let num_start = current_pos;
    while current_pos < pattern_str.len()
        && pattern_str[current_pos..current_pos + 1]
            .chars()
            .next()
            .unwrap()
            .is_ascii_digit()
    {
        current_pos += 1;
    }

    if num_start == current_pos {
        return None;
    }

    let min = pattern_str[num_start..current_pos].parse::<usize>().ok()?;

    while current_pos < pattern_str.len()
        && pattern_str[current_pos..current_pos + 1].trim().is_empty()
    {
        current_pos += 1;
    }

    if current_pos + 7 <= pattern_str.len()
        && &pattern_str[current_pos..current_pos + 7] == "or more"
    {
        current_pos += 7;
        return Some((min, None, current_pos));
    }

    Some((min, Some(min), current_pos))
}

fn parse_part(pattern_str: &str, pos: usize) -> Result<(PatternPart, String, usize), String> {
    if pos >= pattern_str.len() {
        return Err("Unexpected end of pattern".to_string());
    }

    let remaining = &pattern_str[pos..];

    if remaining.starts_with("digit") {
        let mut current_pos = pos + 5;

        let is_plural =
            current_pos < pattern_str.len() && &pattern_str[current_pos..current_pos + 1] == "s";
        if is_plural {
            current_pos += 1;
        }

        if let Some((min, max, new_pos)) = parse_quantifier(pattern_str, current_pos) {
            current_pos = new_pos;

            let part = PatternPart::Digits { min, max };
            let regex = format!(
                "\\d{{{},{}}}",
                min,
                max.map_or("".to_string(), |m| m.to_string())
            );

            return Ok((part, regex, current_pos));
        } else {
            let part = if is_plural {
                PatternPart::Digits { min: 1, max: None }
            } else {
                PatternPart::Digits {
                    min: 1,
                    max: Some(1),
                }
            };

            let regex = if is_plural { "\\d+" } else { "\\d" };

            return Ok((part, regex.to_string(), current_pos));
        }
    }

    if remaining.starts_with("letter") {
        let mut current_pos = pos + 6;

        let is_plural =
            current_pos < pattern_str.len() && &pattern_str[current_pos..current_pos + 1] == "s";
        if is_plural {
            current_pos += 1;
        }

        if let Some((min, max, new_pos)) = parse_quantifier(pattern_str, current_pos) {
            current_pos = new_pos;

            let part = PatternPart::Letters { min, max };
            let regex = format!(
                "[a-zA-Z]{{{},{}}}",
                min,
                max.map_or("".to_string(), |m| m.to_string())
            );

            return Ok((part, regex, current_pos));
        } else {
            let part = if is_plural {
                PatternPart::Letters { min: 1, max: None }
            } else {
                PatternPart::Letters {
                    min: 1,
                    max: Some(1),
                }
            };

            let regex = if is_plural { "[a-zA-Z]+" } else { "[a-zA-Z]" };

            return Ok((part, regex.to_string(), current_pos));
        }
    }

    if remaining.starts_with("whitespace") {
        let mut current_pos = pos + 10;

        if let Some((min, max, new_pos)) = parse_quantifier(pattern_str, current_pos) {
            current_pos = new_pos;

            let part = PatternPart::Whitespace { min, max };
            let regex = format!(
                "\\s{{{},{}}}",
                min,
                max.map_or("".to_string(), |m| m.to_string())
            );

            return Ok((part, regex, current_pos));
        } else {
            let part = PatternPart::Whitespace { min: 1, max: None };
            let regex = "\\s+";

            return Ok((part, regex.to_string(), current_pos));
        }
    }

    let c = pattern_str[pos..pos + 1].chars().next().unwrap();
    let part = PatternPart::Literal(c.to_string());

    let mut regex = String::new();
    if "\\.*+?()[]{}|^$".contains(c) {
        regex.push('\\');
    }
    regex.push(c);

    Ok((part, regex, pos + 1))
}

pub fn native_pattern_matches(args: Vec<Value>) -> Result<Value, RuntimeError> {
    let line = 0;
    let column = 0;
    if args.len() != 2 {
        return Err(RuntimeError::new(
            format!("matches pattern expects 2 arguments, got {}", args.len()),
            line,
            column,
        ));
    }

    let text = match &args[0] {
        Value::Text(s) => s.to_string(),
        _ => {
            return Err(RuntimeError::new(
                format!("Expected text to match, got {}", args[0].type_name()),
                line,
                column,
            ));
        }
    };

    let pattern_str = match &args[1] {
        Value::Text(s) => s.to_string(),
        _ => {
            return Err(RuntimeError::new(
                format!("Expected text pattern, got {}", args[1].type_name()),
                line,
                column,
            ));
        }
    };

    match Pattern::parse(&pattern_str) {
        Ok(pattern) => {
            let result = pattern.matches(&text);
            Ok(Value::Bool(result))
        }
        Err(err) => Err(RuntimeError::new(
            format!("Error parsing pattern: {}", err),
            line,
            column,
        )),
    }
}

pub fn native_pattern_find(args: Vec<Value>) -> Result<Value, RuntimeError> {
    let line = 0;
    let column = 0;
    if args.len() != 2 {
        return Err(RuntimeError::new(
            format!("find pattern expects 2 arguments, got {}", args.len()),
            line,
            column,
        ));
    }

    let pattern_str = match &args[0] {
        Value::Text(s) => s.to_string(),
        _ => {
            return Err(RuntimeError::new(
                format!("Expected text pattern, got {}", args[0].type_name()),
                line,
                column,
            ));
        }
    };

    let text = match &args[1] {
        Value::Text(s) => s.to_string(),
        _ => {
            return Err(RuntimeError::new(
                format!("Expected text to search in, got {}", args[1].type_name()),
                line,
                column,
            ));
        }
    };

    match Pattern::parse(&pattern_str) {
        Ok(pattern) => {
            if let Some(captures) = pattern.find(&text) {
                let mut map = HashMap::new();
                for (key, value) in captures {
                    map.insert(key, Value::Text(Rc::from(value.as_str())));
                }
                Ok(Value::Object(Rc::new(RefCell::new(map))))
            } else {
                Ok(Value::Null)
            }
        }
        Err(err) => Err(RuntimeError::new(
            format!("Error parsing pattern: {}", err),
            line,
            column,
        )),
    }
}

pub fn native_pattern_replace(args: Vec<Value>) -> Result<Value, RuntimeError> {
    let line = 0;
    let column = 0;
    if args.len() != 3 {
        return Err(RuntimeError::new(
            format!("replace pattern expects 3 arguments, got {}", args.len()),
            line,
            column,
        ));
    }

    let pattern_str = match &args[0] {
        Value::Text(s) => s.to_string(),
        _ => {
            return Err(RuntimeError::new(
                format!("Expected text pattern, got {}", args[0].type_name()),
                line,
                column,
            ));
        }
    };

    let replacement = match &args[1] {
        Value::Text(s) => s.to_string(),
        _ => {
            return Err(RuntimeError::new(
                format!("Expected text replacement, got {}", args[1].type_name()),
                line,
                column,
            ));
        }
    };

    let text = match &args[2] {
        Value::Text(s) => s.to_string(),
        _ => {
            return Err(RuntimeError::new(
                format!("Expected text to replace in, got {}", args[2].type_name()),
                line,
                column,
            ));
        }
    };

    match Pattern::parse(&pattern_str) {
        Ok(pattern) => {
            let result = pattern.replace(&text, &replacement);
            Ok(Value::Text(Rc::from(result.as_str())))
        }
        Err(err) => Err(RuntimeError::new(
            format!("Error parsing pattern: {}", err),
            line,
            column,
        )),
    }
}

pub fn native_pattern_split(args: Vec<Value>) -> Result<Value, RuntimeError> {
    let line = 0;
    let column = 0;
    if args.len() != 2 {
        return Err(RuntimeError::new(
            format!("split by pattern expects 2 arguments, got {}", args.len()),
            line,
            column,
        ));
    }

    let text = match &args[0] {
        Value::Text(s) => s.to_string(),
        _ => {
            return Err(RuntimeError::new(
                format!("Expected text to split, got {}", args[0].type_name()),
                line,
                column,
            ));
        }
    };

    let pattern_str = match &args[1] {
        Value::Text(s) => s.to_string(),
        _ => {
            return Err(RuntimeError::new(
                format!("Expected text pattern, got {}", args[1].type_name()),
                line,
                column,
            ));
        }
    };

    match Pattern::parse(&pattern_str) {
        Ok(pattern) => {
            let parts = pattern.split(&text);
            let values: Vec<Value> = parts
                .into_iter()
                .map(|s| Value::Text(Rc::from(s.as_str())))
                .collect();

            Ok(Value::List(Rc::new(RefCell::new(values))))
        }
        Err(err) => Err(RuntimeError::new(
            format!("Error parsing pattern: {}", err),
            line,
            column,
        )),
    }
}

pub fn register(env: &mut Environment) {
    env.define(
        "matches_pattern",
        Value::NativeFunction("matches_pattern", native_pattern_matches),
    );
    env.define(
        "find_pattern",
        Value::NativeFunction("find_pattern", native_pattern_find),
    );
    env.define(
        "replace_pattern",
        Value::NativeFunction("replace_pattern", native_pattern_replace),
    );
    env.define(
        "split_by_pattern",
        Value::NativeFunction("split_by_pattern", native_pattern_split),
    );
}
