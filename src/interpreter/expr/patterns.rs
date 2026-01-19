use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::interpreter::environment::Environment;
use crate::interpreter::error::{ErrorKind, RuntimeError};
use crate::interpreter::value::Value;
use crate::interpreter::Interpreter;
use crate::parser::ast::Expression;
use crate::pattern::CompiledPattern;

pub trait PatternExpressionEvaluator {
    async fn evaluate_pattern_match(
        &self,
        text: &Expression,
        pattern: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError>;

    async fn evaluate_pattern_find(
        &self,
        text: &Expression,
        pattern: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError>;

    async fn evaluate_pattern_replace(
        &self,
        text: &Expression,
        pattern: &Expression,
        replacement: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError>;

    async fn evaluate_pattern_split(
        &self,
        text: &Expression,
        pattern: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError>;

    async fn evaluate_string_split(
        &self,
        text: &Expression,
        delimiter: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError>;
}

impl PatternExpressionEvaluator for Interpreter {
    async fn evaluate_pattern_match(
        &self,
        text: &Expression,
        pattern: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError> {
        let text_val = self.evaluate_expression(text, Rc::clone(&env)).await?;
        let pattern_val = self.evaluate_expression(pattern, Rc::clone(&env)).await?;

        let text_str = match &text_val {
            Value::Text(s) => s.as_ref(),
            _ => {
                return Err(RuntimeError::new(
                    "Text to match must be a string".to_string(),
                    line,
                    column,
                ));
            }
        };

        // Handle both compiled patterns and simple string patterns
        match &pattern_val {
            Value::Pattern(compiled_pattern) => {
                let is_match = compiled_pattern.matches(text_str);
                Ok(Value::Bool(is_match))
            }
            Value::Text(pattern_str) => {
                // Simple string match
                Ok(Value::Bool(text_str == pattern_str.as_ref()))
            }
            _ => Err(RuntimeError::new(
                "Pattern must be a pattern object or string".to_string(),
                line,
                column,
            )),
        }
    }

    async fn evaluate_pattern_find(
        &self,
        text: &Expression,
        pattern: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError> {
        let text_val = self.evaluate_expression(text, Rc::clone(&env)).await?;
        let pattern_val = self.evaluate_expression(pattern, Rc::clone(&env)).await?;

        let text_str = match &text_val {
            Value::Text(s) => s.as_ref(),
            _ => {
                return Err(RuntimeError::new(
                    "Text to match must be a string".to_string(),
                    line,
                    column,
                ));
            }
        };

        match &pattern_val {
            Value::Pattern(compiled_pattern) => {
                match compiled_pattern.find(text_str) {
                    Some(match_result) => {
                        // Convert match result to WFL object
                        let mut map = HashMap::new();
                        map.insert(
                            "match".to_string(),
                            Value::Text(Rc::from(match_result.matched_text)),
                        );
                        map.insert(
                            "index".to_string(),
                            Value::Number(match_result.start as f64),
                        );
                        map.insert(
                            "length".to_string(),
                            Value::Number((match_result.end - match_result.start) as f64),
                        );

                        // Add captures
                        let mut captures_map = HashMap::new();
                        for (name, value) in match_result.captures {
                            captures_map.insert(name, Value::Text(Rc::from(value)));
                        }
                        map.insert(
                            "captures".to_string(),
                            Value::Object(Rc::new(RefCell::new(captures_map))),
                        );

                        Ok(Value::Object(Rc::new(RefCell::new(map))))
                    }
                    None => Ok(Value::Nothing),
                }
            }
            Value::Text(pattern_str) => {
                // Simple string find
                if let Some(idx) = text_str.find(pattern_str.as_ref()) {
                    let mut map = HashMap::new();
                    map.insert(
                        "match".to_string(),
                        Value::Text(Rc::clone(pattern_str)),
                    );
                    map.insert("index".to_string(), Value::Number(idx as f64));
                    map.insert(
                        "length".to_string(),
                        Value::Number(pattern_str.len() as f64),
                    );
                    map.insert(
                        "captures".to_string(),
                        Value::Object(Rc::new(RefCell::new(HashMap::new()))),
                    );

                    Ok(Value::Object(Rc::new(RefCell::new(map))))
                } else {
                    Ok(Value::Nothing)
                }
            }
            _ => Err(RuntimeError::new(
                "Pattern must be a pattern object or string".to_string(),
                line,
                column,
            )),
        }
    }

    async fn evaluate_pattern_replace(
        &self,
        text: &Expression,
        pattern: &Expression,
        replacement: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError> {
        let text_val = self.evaluate_expression(text, Rc::clone(&env)).await?;
        let pattern_val = self.evaluate_expression(pattern, Rc::clone(&env)).await?;
        let replacement_val = self.evaluate_expression(replacement, Rc::clone(&env)).await?;

        let text_str = match &text_val {
            Value::Text(s) => s.as_ref(),
            _ => {
                return Err(RuntimeError::new(
                    "Text to replace must be a string".to_string(),
                    line,
                    column,
                ));
            }
        };

        let replacement_str = match &replacement_val {
            Value::Text(s) => s.as_ref(),
            _ => {
                return Err(RuntimeError::new(
                    "Replacement must be a string".to_string(),
                    line,
                    column,
                ));
            }
        };

        match &pattern_val {
            Value::Pattern(compiled_pattern) => {
                let matches = compiled_pattern.find_all(text_str);
                if matches.is_empty() {
                    return Ok(Value::Text(Rc::from(text_str)));
                }

                let mut result = String::new();
                let mut last_end = 0;

                for m in matches {
                    result.push_str(&text_str[last_end..m.start]);
                    // TODO: Support capture group references in replacement (e.g., $1)
                    // For now, just literal replacement
                    result.push_str(replacement_str);
                    last_end = m.end;
                }
                result.push_str(&text_str[last_end..]);

                Ok(Value::Text(Rc::from(result)))
            }
            Value::Text(pattern_str) => {
                // Simple string replace
                Ok(Value::Text(Rc::from(text_str.replace(pattern_str.as_ref(), replacement_str))))
            }
            _ => Err(RuntimeError::new(
                "Pattern must be a pattern object or string".to_string(),
                line,
                column,
            )),
        }
    }

    async fn evaluate_pattern_split(
        &self,
        text: &Expression,
        pattern: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError> {
        let text_val = self.evaluate_expression(text, Rc::clone(&env)).await?;
        let pattern_val = self.evaluate_expression(pattern, Rc::clone(&env)).await?;

        let text_str = match &text_val {
            Value::Text(s) => s.as_ref(),
            _ => {
                return Err(RuntimeError::new(
                    "Text to split must be a string".to_string(),
                    line,
                    column,
                ));
            }
        };

        match &pattern_val {
            Value::Pattern(compiled_pattern) => {
                let matches = compiled_pattern.find_all(text_str);
                let mut parts_vec = Vec::new();
                let mut last_end = 0;

                for m in matches {
                    parts_vec.push(Value::Text(Rc::from(&text_str[last_end..m.start])));
                    last_end = m.end;
                }
                parts_vec.push(Value::Text(Rc::from(&text_str[last_end..])));

                Ok(Value::List(Rc::new(RefCell::new(parts_vec))))
            }
            Value::Text(pattern_str) => {
                // Simple string split
                let parts: Vec<Value> = text_str
                    .split(pattern_str.as_ref())
                    .map(|s| Value::Text(Rc::from(s)))
                    .collect();
                Ok(Value::List(Rc::new(RefCell::new(parts))))
            }
            _ => Err(RuntimeError::new(
                "Pattern must be a pattern object or string".to_string(),
                line,
                column,
            )),
        }
    }

    async fn evaluate_string_split(
        &self,
        text: &Expression,
        delimiter: &Expression,
        line: usize,
        column: usize,
        env: Rc<RefCell<Environment>>,
    ) -> Result<Value, RuntimeError> {
         let text_val = self.evaluate_expression(text, Rc::clone(&env)).await?;
        let delimiter_val = self.evaluate_expression(delimiter, Rc::clone(&env)).await?;

        let text_str = match &text_val {
            Value::Text(s) => s.as_ref(),
            _ => {
                return Err(RuntimeError::new(
                    "Text to split must be a string".to_string(),
                    line,
                    column,
                ));
            }
        };

        let delimiter_str = match &delimiter_val {
            Value::Text(s) => s.as_ref(),
            _ => {
                return Err(RuntimeError::new(
                    "Delimiter must be a string".to_string(),
                    line,
                    column,
                ));
            }
        };

        let parts: Vec<Value> = text_str
            .split(delimiter_str)
            .map(|s| Value::Text(Rc::from(s)))
            .collect();

        Ok(Value::List(Rc::new(RefCell::new(parts))))
    }
}
