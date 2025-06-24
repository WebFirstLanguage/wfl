#[cfg(test)]
mod tests {
    use crate::interpreter::value::Value;
    use crate::stdlib::pattern::{
        AnchorType, CharClass, PatternNode, exec_match, native_pattern_find,
        native_pattern_matches, native_pattern_replace, native_pattern_split, parse_ir,
    };
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
    fn test_ir_parse_letter_class() {
        let pattern = parse_ir("class(letter)").unwrap();
        assert_eq!(pattern.root, PatternNode::CharClass(CharClass::Letter));
    }

    #[test]
    fn test_ir_parse_whitespace_class() {
        let pattern = parse_ir("class(whitespace)").unwrap();
        assert_eq!(pattern.root, PatternNode::CharClass(CharClass::Whitespace));
    }

    #[test]
    fn test_match_literal_abc() {
        let pattern = parse_ir("lit(\"abc\")").unwrap();
        let result = exec_match(&pattern, "abc").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().matched_text, "abc");
    }

    #[test]
    fn test_match_literal_abc_fail() {
        let pattern = parse_ir("lit(\"abc\")").unwrap();
        let result = exec_match(&pattern, "def").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_match_digit() {
        let pattern = parse_ir("class(digit)").unwrap();
        let result = exec_match(&pattern, "5").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().matched_text, "5");
    }

    #[test]
    fn test_match_digit_fail() {
        let pattern = parse_ir("class(digit)").unwrap();
        let result = exec_match(&pattern, "a").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_match_letter() {
        let pattern = parse_ir("class(letter)").unwrap();
        let result = exec_match(&pattern, "x").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().matched_text, "x");
    }

    #[test]
    fn test_match_letter_fail() {
        let pattern = parse_ir("class(letter)").unwrap();
        let result = exec_match(&pattern, "9").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_match_whitespace() {
        let pattern = parse_ir("class(whitespace)").unwrap();
        let result = exec_match(&pattern, " ").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().matched_text, " ");
    }

    #[test]
    fn test_match_whitespace_tab() {
        let pattern = parse_ir("class(whitespace)").unwrap();
        let result = exec_match(&pattern, "\t").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().matched_text, "\t");
    }

    #[test]
    fn test_match_whitespace_fail() {
        let pattern = parse_ir("class(whitespace)").unwrap();
        let result = exec_match(&pattern, "a").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_ir_parse_one_or_more() {
        let pattern = parse_ir("rep(1,inf,class(digit))").unwrap();
        if let PatternNode::Rep { min, max, child } = &pattern.root {
            assert_eq!(*min, 1);
            assert_eq!(*max, u32::MAX);
            assert_eq!(**child, PatternNode::CharClass(CharClass::Digit));
        } else {
            panic!("Expected Rep node");
        }
    }

    #[test]
    fn test_ir_parse_optional() {
        let pattern = parse_ir("rep(0,1,class(digit))").unwrap();
        if let PatternNode::Rep { min, max, child } = &pattern.root {
            assert_eq!(*min, 0);
            assert_eq!(*max, 1);
            assert_eq!(**child, PatternNode::CharClass(CharClass::Digit));
        } else {
            panic!("Expected Rep node");
        }
    }

    #[test]
    fn test_ir_parse_between() {
        let pattern = parse_ir("rep(2,5,class(letter))").unwrap();
        if let PatternNode::Rep { min, max, child } = &pattern.root {
            assert_eq!(*min, 2);
            assert_eq!(*max, 5);
            assert_eq!(**child, PatternNode::CharClass(CharClass::Letter));
        } else {
            panic!("Expected Rep node");
        }
    }

    #[test]
    fn test_ir_parse_alternation() {
        let pattern = parse_ir("alt(class(digit),class(letter))").unwrap();
        if let PatternNode::Alt { alternatives } = &pattern.root {
            assert_eq!(alternatives.len(), 2);
            assert_eq!(alternatives[0], PatternNode::CharClass(CharClass::Digit));
            assert_eq!(alternatives[1], PatternNode::CharClass(CharClass::Letter));
        } else {
            panic!("Expected Alt node");
        }
    }

    #[test]
    fn test_match_one_or_more_digits() {
        let pattern = parse_ir("rep(1,inf,class(digit))").unwrap();
        let result = exec_match(&pattern, "123").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().matched_text, "123");
    }

    #[test]
    fn test_match_one_or_more_digits_fail() {
        let pattern = parse_ir("rep(1,inf,class(digit))").unwrap();
        let result = exec_match(&pattern, "").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_match_optional_digit() {
        let pattern = parse_ir("rep(0,1,class(digit))").unwrap();
        let result = exec_match(&pattern, "5").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().matched_text, "5");
    }

    #[test]
    fn test_match_optional_digit_empty() {
        let pattern = parse_ir("rep(0,1,class(digit))").unwrap();
        let result = exec_match(&pattern, "").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().matched_text, "");
    }

    #[test]
    fn test_match_between_2_and_4_letters() {
        let pattern = parse_ir("rep(2,4,class(letter))").unwrap();
        let result = exec_match(&pattern, "abc").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().matched_text, "abc");
    }

    #[test]
    fn test_match_between_2_and_4_letters_fail_too_few() {
        let pattern = parse_ir("rep(2,4,class(letter))").unwrap();
        let result = exec_match(&pattern, "a").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_match_between_2_and_4_letters_fail_too_many() {
        let pattern = parse_ir("rep(2,4,class(letter))").unwrap();
        let result = exec_match(&pattern, "abcde").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_match_alternation_digit() {
        let pattern = parse_ir("alt(class(digit),class(letter))").unwrap();
        let result = exec_match(&pattern, "5").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().matched_text, "5");
    }

    #[test]
    fn test_match_alternation_letter() {
        let pattern = parse_ir("alt(class(digit),class(letter))").unwrap();
        let result = exec_match(&pattern, "x").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().matched_text, "x");
    }

    #[test]
    fn test_match_alternation_fail() {
        let pattern = parse_ir("alt(class(digit),class(letter))").unwrap();
        let result = exec_match(&pattern, " ").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_match_sequence() {
        let pattern = parse_ir("seq(class(digit),class(letter))").unwrap();
        let result = exec_match(&pattern, "5x").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().matched_text, "5x");
    }

    #[test]
    fn test_match_sequence_fail() {
        let pattern = parse_ir("seq(class(digit),class(letter))").unwrap();
        let result = exec_match(&pattern, "55").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_ir_parse_capture() {
        let pattern = parse_ir("cap(\"name\",class(digit))").unwrap();
        if let PatternNode::Capture { name, child } = &pattern.root {
            assert_eq!(name, "name");
            assert_eq!(**child, PatternNode::CharClass(CharClass::Digit));
        } else {
            panic!("Expected Capture node");
        }
    }

    #[test]
    fn test_match_capture() {
        let pattern = parse_ir("cap(\"digit\",class(digit))").unwrap();
        let result = exec_match(&pattern, "7").unwrap();
        assert!(result.is_some());
        let match_result = result.unwrap();
        assert_eq!(match_result.matched_text, "7");
        assert_eq!(match_result.captures.get("digit"), Some(&"7".to_string()));
    }

    #[test]
    fn test_match_multiple_captures() {
        let pattern =
            parse_ir("seq(cap(\"first\",class(digit)),cap(\"second\",class(letter)))").unwrap();
        let result = exec_match(&pattern, "5x").unwrap();
        assert!(result.is_some());
        let match_result = result.unwrap();
        assert_eq!(match_result.matched_text, "5x");
        assert_eq!(match_result.captures.get("first"), Some(&"5".to_string()));
        assert_eq!(match_result.captures.get("second"), Some(&"x".to_string()));
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
    fn test_native_pattern_matches_fail() {
        let args = vec![
            Value::Text(Rc::from("def")),
            Value::Pattern(Rc::new(parse_ir("lit(\"abc\")").unwrap())),
        ];
        let result = native_pattern_matches(args, 0, 0).unwrap();
        assert_eq!(result, Value::Bool(false));
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
            if let Value::Text(letter) = obj.get("letter").unwrap() {
                assert_eq!(letter.to_string(), "x");
            } else {
                panic!("Expected letter to be a text value");
            }
        } else {
            panic!("Expected result to be an object");
        }
    }

    #[test]
    fn test_native_pattern_find_no_match() {
        let args = vec![
            Value::Text(Rc::from("xyz")),
            Value::Pattern(Rc::new(parse_ir("lit(\"abc\")").unwrap())),
        ];
        let result = native_pattern_find(args, 0, 0).unwrap();
        assert_eq!(result, Value::Null);
    }

    #[test]
    fn test_native_pattern_replace_basic() {
        let args = vec![
            Value::Text(Rc::from("hello abc world")),
            Value::Pattern(Rc::new(parse_ir("lit(\"abc\")").unwrap())),
            Value::Text(Rc::from("XYZ")),
        ];
        let result = native_pattern_replace(args, 0, 0).unwrap();
        if let Value::Text(text) = result {
            assert_eq!(text.to_string(), "hello XYZ world");
        } else {
            panic!("Expected result to be a text value");
        }
    }

    #[test]
    fn test_native_pattern_replace_no_match() {
        let args = vec![
            Value::Text(Rc::from("hello world")),
            Value::Pattern(Rc::new(parse_ir("lit(\"abc\")").unwrap())),
            Value::Text(Rc::from("XYZ")),
        ];
        let result = native_pattern_replace(args, 0, 0).unwrap();
        if let Value::Text(text) = result {
            assert_eq!(text.to_string(), "hello world");
        } else {
            panic!("Expected result to be a text value");
        }
    }

    #[test]
    fn test_native_pattern_split_basic() {
        let args = vec![
            Value::Text(Rc::from("a,b,c")),
            Value::Pattern(Rc::new(parse_ir("lit(\",\")").unwrap())),
        ];
        let result = native_pattern_split(args, 0, 0).unwrap();

        if let Value::List(list_rc) = result {
            let list = list_rc.borrow();
            assert_eq!(list.len(), 3);
            if let Value::Text(text) = &list[0] {
                assert_eq!(text.to_string(), "a");
            } else {
                panic!("Expected list item to be a text value");
            }
            if let Value::Text(text) = &list[1] {
                assert_eq!(text.to_string(), "b");
            } else {
                panic!("Expected list item to be a text value");
            }
            if let Value::Text(text) = &list[2] {
                assert_eq!(text.to_string(), "c");
            } else {
                panic!("Expected list item to be a text value");
            }
        } else {
            panic!("Expected result to be a list");
        }
    }

    #[test]
    fn test_native_pattern_split_no_match() {
        let args = vec![
            Value::Text(Rc::from("abc")),
            Value::Pattern(Rc::new(parse_ir("lit(\",\")").unwrap())),
        ];
        let result = native_pattern_split(args, 0, 0).unwrap();

        if let Value::List(list_rc) = result {
            let list = list_rc.borrow();
            assert_eq!(list.len(), 1);
            if let Value::Text(text) = &list[0] {
                assert_eq!(text.to_string(), "abc");
            } else {
                panic!("Expected list item to be a text value");
            }
        } else {
            panic!("Expected result to be a list");
        }
    }

    #[test]
    fn test_ir_parse_start_anchor() {
        let pattern = parse_ir("anchor(start)").unwrap();
        assert_eq!(pattern.root, PatternNode::Anchor(AnchorType::Start));
    }

    #[test]
    fn test_ir_parse_end_anchor() {
        let pattern = parse_ir("anchor(end)").unwrap();
        assert_eq!(pattern.root, PatternNode::Anchor(AnchorType::End));
    }

    #[test]
    fn test_match_start_anchor() {
        let pattern = parse_ir("seq(anchor(start),lit(\"abc\"))").unwrap();
        let result = exec_match(&pattern, "abc").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().matched_text, "abc");
    }

    #[test]
    fn test_match_start_anchor_fail() {
        let pattern = parse_ir("seq(anchor(start),lit(\"abc\"))").unwrap();
        let result = exec_match(&pattern, "xabc").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_match_end_anchor() {
        let pattern = parse_ir("seq(lit(\"abc\"),anchor(end))").unwrap();
        let result = exec_match(&pattern, "abc").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().matched_text, "abc");
    }

    #[test]
    fn test_match_end_anchor_fail() {
        let pattern = parse_ir("seq(lit(\"abc\"),anchor(end))").unwrap();
        let result = exec_match(&pattern, "abcx").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_performance_limit_exceeded() {
        let pattern_ir = "rep(0,inf,rep(0,1,class(letter)))";
        let pattern = parse_ir(pattern_ir).unwrap();

        let large_input = "a".repeat(1000);

        let _result = exec_match(&pattern, &large_input).unwrap();
    }

    #[test]
    fn test_performance_regression_20_optional_groups() {
        use std::time::Instant;

        let mut pattern_parts = Vec::new();
        for i in 0..20 {
            pattern_parts.push(format!("rep(0,1,lit(\"{}\"))", i));
        }
        let pattern_ir = format!("seq({})", pattern_parts.join(","));
        let pattern = parse_ir(&pattern_ir).unwrap();

        let input = "0123456789".repeat(200); // 2000 chars ≈ 2KB

        let start = Instant::now();
        let _result = exec_match(&pattern, &input).unwrap();
        let duration = start.elapsed();

        assert!(
            duration.as_millis() < 200,
            "Pattern matching took {}ms, expected < 200ms",
            duration.as_millis()
        );
    }

    #[test]
    fn test_invalid_ir_syntax() {
        let result = parse_ir("invalid(syntax)");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unknown IR function")
        );
    }

    #[test]
    fn test_invalid_range_quantifier() {
        let result = parse_ir("rep(5,2,class(digit))");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid range"));
    }

    #[test]
    fn test_unclosed_group_error() {
        let result = parse_ir("seq(class(digit)");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Expected closing parenthesis")
        );
    }

    #[test]
    fn test_complex_pattern_with_captures_and_quantifiers() {
        let pattern = parse_ir("seq(cap(\"prefix\",rep(1,3,class(letter))),lit(\"-\"),cap(\"suffix\",rep(2,4,class(digit))))").unwrap();
        let result = exec_match(&pattern, "abc-123").unwrap();
        assert!(result.is_some());
        let match_result = result.unwrap();
        assert_eq!(match_result.matched_text, "abc-123");
        assert_eq!(
            match_result.captures.get("prefix"),
            Some(&"abc".to_string())
        );
        assert_eq!(
            match_result.captures.get("suffix"),
            Some(&"123".to_string())
        );
    }

    #[test]
    fn test_nested_alternation_and_repetition() {
        let pattern = parse_ir("rep(1,inf,alt(class(digit),class(letter)))").unwrap();
        let result = exec_match(&pattern, "a1b2c3").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().matched_text, "a1b2c3");
    }

    #[test]
    fn test_anchored_pattern_with_captures() {
        let pattern =
            parse_ir("seq(anchor(start),cap(\"word\",rep(1,inf,class(letter))),anchor(end))")
                .unwrap();
        let result = exec_match(&pattern, "hello").unwrap();
        assert!(result.is_some());
        let match_result = result.unwrap();
        assert_eq!(match_result.matched_text, "hello");
        assert_eq!(
            match_result.captures.get("word"),
            Some(&"hello".to_string())
        );
    }

    #[test]
    fn test_empty_input_with_optional_pattern() {
        let pattern = parse_ir("rep(0,inf,class(digit))").unwrap();
        let result = exec_match(&pattern, "").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().matched_text, "");
    }
}
