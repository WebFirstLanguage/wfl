//! User-defined action overloading tests, consolidated into a single
//! integration-test crate: each additional tests/*.rs file links its own copy
//! of the (debug-info-heavy) wfl library, and the extra binaries pushed CI
//! runners into out-of-disk linker failures (ld signal 7 / Bus error).
//! Analyzer, typechecker, and interpreter coverage live in the modules below.

// Analyzer tests for user-defined action overloading.
//
// Overloading rules (definition time):
// - Two same-scope actions with the same name are legal when they differ in
//   parameter count, or in at least one position where BOTH declare concrete,
//   different parameter types (`as number` vs `as text`).
// - Exact duplicates (same arity, same normalized type vector) are rejected.
// - Indistinguishable same-arity pairs (e.g. `f(x)` vs `f(x as number)`) are
//   rejected, because an untyped parameter accepts numbers too.
//
// Call-site rules:
// - Candidates are filtered by arity, then by static argument types.
// - No arity match / no type match are errors listing the candidates.
// - Statically ambiguous calls (Unknown-typed arguments) defer to runtime
//   dispatch with no analyzer error.
mod analyzer {
    use wfl::analyzer::Analyzer;
    use wfl::lexer::lex_wfl_with_positions;
    use wfl::parser::Parser;

    fn analyze_errors(code: &str) -> Vec<String> {
        let tokens = lex_wfl_with_positions(code);
        let mut parser = Parser::new(&tokens);
        let program = parser
            .parse()
            .unwrap_or_else(|e| panic!("Failed to parse {code:?}: {e:?}"));

        let mut analyzer = Analyzer::new();
        match analyzer.analyze(&program) {
            Ok(()) => Vec::new(),
            Err(errors) => errors.into_iter().map(|e| e.message).collect(),
        }
    }

    // --- Definition-time rules --------------------------------------------------

    #[test]
    fn overload_by_arity_accepted() {
        let errors = analyze_errors(
            r#"
    define action called greet with parameters name:
        display name
    end action

    define action called greet with parameters first and last:
        display first with last
    end action

    call greet with "a"
    call greet with "a" and "b"
    "#,
        );
        assert!(
            errors.is_empty(),
            "arity-distinct overloads should be accepted: {errors:?}"
        );
    }

    #[test]
    fn typed_same_arity_accepted() {
        let errors = analyze_errors(
            r#"
    define action called depict with parameters value as number:
        display value
    end action

    define action called depict with parameters value as text:
        display value
    end action

    call depict with 5
    call depict with "hello"
    "#,
        );
        assert!(
            errors.is_empty(),
            "type-distinct same-arity overloads should be accepted: {errors:?}"
        );
    }

    #[test]
    fn exact_duplicate_rejected() {
        let errors = analyze_errors(
            r#"
    define action called greet with parameters name as text:
        display name
    end action

    define action called greet with parameters other as text:
        display other
    end action
    "#,
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains("greet") && e.contains("same parameters")),
            "exact duplicate signature must be rejected: {errors:?}"
        );
    }

    #[test]
    fn untyped_duplicate_rejected() {
        let errors = analyze_errors(
            r#"
    define action called greet with parameters name:
        display name
    end action

    define action called greet with parameters other:
        display other
    end action
    "#,
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains("greet") && e.contains("same parameters")),
            "two untyped same-arity definitions must be rejected: {errors:?}"
        );
    }

    #[test]
    fn ambiguous_same_arity_rejected() {
        let errors = analyze_errors(
            r#"
    define action called f with parameters x:
        display x
    end action

    define action called f with parameters x as number:
        display x
    end action
    "#,
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains("f") && e.contains("cannot be told apart")),
            "untyped vs typed same-arity pair must be rejected as ambiguous: {errors:?}"
        );
    }

    #[test]
    fn variable_function_collision_still_rejected() {
        let errors = analyze_errors(
            r#"
    store greet as 5

    define action called greet with parameters name:
        display name
    end action
    "#,
        );
        assert!(
            errors.iter().any(|e| e.contains("already been defined")),
            "variable/function name collision must remain an error: {errors:?}"
        );
    }

    // --- Call-site resolution ---------------------------------------------------

    #[test]
    fn call_no_arity_match_lists_arities() {
        let errors = analyze_errors(
            r#"
    define action called f with parameters x:
        display x
    end action

    define action called f with parameters x and y:
        display x with y
    end action

    call f with 1 and 2 and 3
    "#,
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains('f') && e.contains('3') && e.contains('1') && e.contains('2')),
            "wrong-arity call must list available arities: {errors:?}"
        );
    }

    #[test]
    fn call_no_type_match_lists_candidates() {
        let errors = analyze_errors(
            r#"
    define action called f with parameters x as text:
        display x
    end action

    define action called f with parameters x as boolean:
        display x
    end action

    call f with 5
    "#,
        );
        assert!(
            errors.iter().any(|e| e.contains("No version of 'f'")),
            "no-type-match call must produce a candidate-listing error: {errors:?}"
        );
    }

    #[test]
    fn call_dynamic_arg_deferred_without_error() {
        let errors = analyze_errors(
            r#"
    define action called f with parameters x as number:
        display x
    end action

    define action called f with parameters x as text:
        display x
    end action

    define action called g with parameters v:
        call f with v
    end action
    "#,
        );
        assert!(
            errors.is_empty(),
            "a dynamically-typed argument must defer to runtime, not error: {errors:?}"
        );
    }

    #[test]
    fn of_form_and_call_form_agree() {
        let of_form = analyze_errors(
            r#"
    define action called f with parameters x as text:
        display x
    end action

    define action called f with parameters x as boolean:
        display x
    end action

    store r as f of 5
    "#,
        );
        let call_form = analyze_errors(
            r#"
    define action called f with parameters x as text:
        display x
    end action

    define action called f with parameters x as boolean:
        display x
    end action

    store r as call f with 5
    "#,
        );
        assert!(
            of_form.iter().any(|e| e.contains("No version of 'f'")),
            "of-form must reject a no-type-match call: {of_form:?}"
        );
        assert!(
            call_form.iter().any(|e| e.contains("No version of 'f'")),
            "call-form must reject a no-type-match call: {call_form:?}"
        );
    }

    #[test]
    fn nothing_and_pattern_type_annotations_parse() {
        // `nothing` lexes as NothingLiteral and `pattern`/`text` as keywords, so
        // type positions must accept those tokens, not just identifiers
        // (PR #639 review).
        let errors = analyze_errors(
            r#"
    define action called f with parameters x as nothing:
        display "nothing"
    end action

    define action called f with parameters x as number:
        display x
    end action
    "#,
        );
        assert!(
            errors.is_empty(),
            "'as nothing' must parse and overload against 'as number': {errors:?}"
        );
    }

    #[test]
    fn any_type_annotation_parses_but_cannot_distinguish() {
        // `as any` parses (KeywordAny in type position), but Any accepts every
        // value, so it cannot separate two same-count overloads — same rule as
        // untyped parameters (PR #639 review).
        let errors = analyze_errors(
            r#"
    define action called f with parameters x as any:
        display x
    end action

    define action called f with parameters x as number:
        display x
    end action
    "#,
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains('f') && e.contains("cannot be told apart")),
            "'as any' vs 'as number' same-arity pair must be rejected as ambiguous: {errors:?}"
        );
    }

    #[test]
    fn single_signature_behavior_unchanged() {
        // The classic single-definition path must keep its existing diagnostics.
        let errors = analyze_errors(
            r#"
    define action called greet with parameters name:
        display name
    end action

    call greet with "a" and "b"
    "#,
        );
        assert!(
            !errors.is_empty(),
            "wrong arity against a single signature must still error"
        );
    }
}

// Typechecker tests for user-defined action overloading: each overload's
// inferred return type must flow to call sites that statically resolve to
// it, and statically ambiguous (deferred) calls must not produce spurious
// errors.
//
// Return types are always inferred from action bodies — WFL has no working
// return-type annotation syntax (the lexer folds `name returns text` into a
// single multi-word identifier), so the overload machinery leans on the
// post-body inference from issue #575.
mod typechecker {
    use wfl::lexer::lex_wfl_with_positions;
    use wfl::parser::Parser;
    use wfl::typechecker::TypeChecker;

    fn typecheck_errors(code: &str) -> Vec<String> {
        let tokens = lex_wfl_with_positions(code);
        let mut parser = Parser::new(&tokens);
        let program = parser
            .parse()
            .unwrap_or_else(|e| panic!("Failed to parse {code:?}: {e:?}"));

        let mut checker = TypeChecker::new();
        match checker.check_types(&program) {
            Ok(()) => Vec::new(),
            Err(err) => err
                .into_diagnostics()
                .into_iter()
                .map(|e| e.message)
                .collect(),
        }
    }

    fn assert_clean(code: &str) {
        let errors = typecheck_errors(code);
        assert!(
            errors.is_empty(),
            "expected clean typecheck, got: {errors:?}"
        );
    }

    #[test]
    fn return_type_per_overload_number_path() {
        // `f of 1` resolves to the number overload (inferred return: Number), so
        // arithmetic on its result typechecks.
        assert_clean(
            r#"
    define action called f with parameters x as number:
        return x plus 1
    end action

    define action called f with parameters x as text:
        return "text result"
    end action

    store a as f of 1
    store b as a plus 1
    display b
    "#,
        );
    }

    #[test]
    fn return_type_per_overload_text_misuse_rejected() {
        // `f of "s"` resolves to the text overload (inferred return: Text);
        // multiplying its result must be a type error.
        let errors = typecheck_errors(
            r#"
    define action called f with parameters x as number:
        return x plus 1
    end action

    define action called f with parameters x as text:
        return "text result"
    end action

    store c as f of "s"
    store d as c times 2
    display d
    "#,
        );
        assert!(
            !errors.is_empty(),
            "using the text overload's result as a number must be rejected"
        );
    }

    #[test]
    fn common_return_when_deferred() {
        // Both overloads return text, so even a deferred (dynamic-arg) call has
        // a known Text result that concatenation accepts.
        assert_clean(
            r#"
    define action called f with parameters x as number:
        return "n"
    end action

    define action called f with parameters x as text:
        return "t"
    end action

    define action called g with parameters v:
        store r as f of v
        store s as r with "!"
        display s
    end action
    "#,
        );
    }

    #[test]
    fn divergent_return_when_deferred_does_not_cascade() {
        // Overloads disagree on return type; a deferred call infers Unknown and
        // any downstream use stays diagnostic-free.
        assert_clean(
            r#"
    define action called f with parameters x as number:
        return 1
    end action

    define action called f with parameters x as text:
        return "t"
    end action

    define action called g with parameters v:
        store r as f of v
        display r
    end action
    "#,
        );
    }

    #[test]
    fn forward_reference_to_later_overload() {
        // PASS 1 registers all top-level signatures before checking, so a call
        // that statically resolves to a later-defined overload is clean.
        assert_clean(
            r#"
    define action called f with parameters x as number:
        return 1
    end action

    define action called probe:
        return f of "hello"
    end action

    define action called f with parameters x as text:
        return "t"
    end action
    "#,
        );
    }
}

// Interpreter tests for user-defined action overloading: runtime dispatch by
// arity, then by declared parameter types against the runtime argument
// values, picking the most specific match (ties resolve to definition order).
mod interpreter {
    use wfl::interpreter::Interpreter;
    use wfl::interpreter::value::Value;
    use wfl::lexer::lex_wfl_with_positions;
    use wfl::parser::Parser;

    async fn run(code: &str) -> Result<Interpreter, String> {
        let tokens = lex_wfl_with_positions(code);
        let mut parser = Parser::new(&tokens);
        let program = parser
            .parse()
            .unwrap_or_else(|e| panic!("Failed to parse {code:?}: {e:?}"));

        let mut interpreter = Interpreter::new();
        match interpreter.interpret(&program).await {
            Ok(_) => Ok(interpreter),
            Err(errors) => Err(errors
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("; ")),
        }
    }

    async fn run_err(code: &str, why: &str) -> String {
        match run(code).await {
            Ok(_) => panic!("{why}"),
            Err(e) => e,
        }
    }

    fn global_text(interpreter: &Interpreter, name: &str) -> String {
        match interpreter.global_env().borrow().get(name) {
            Some(Value::Text(t)) => t.to_string(),
            other => panic!("expected Text in '{name}', got {other:?}"),
        }
    }

    fn global_number(interpreter: &Interpreter, name: &str) -> f64 {
        match interpreter.global_env().borrow().get(name) {
            Some(Value::Number(n)) => n,
            other => panic!("expected Number in '{name}', got {other:?}"),
        }
    }

    #[tokio::test]
    async fn arity_dispatch_runtime() {
        let interp = run(r#"
    define action called f with parameters x:
        return "one"
    end action

    define action called f with parameters x and y:
        return "two"
    end action

    store r1 as f of 1
    store r2 as f of 1 and 2
    "#)
        .await
        .expect("program should run");
        assert_eq!(global_text(&interp, "r1"), "one");
        assert_eq!(global_text(&interp, "r2"), "two");
    }

    #[tokio::test]
    async fn type_dispatch_runtime() {
        let interp = run(r#"
    define action called f with parameters x as number:
        return "num"
    end action

    define action called f with parameters x as text:
        return "text"
    end action

    store r1 as f of 5
    store r2 as f of "hello"

    store v as 7
    store r3 as f of v
    "#)
        .await
        .expect("program should run");
        assert_eq!(global_text(&interp, "r1"), "num");
        assert_eq!(global_text(&interp, "r2"), "text");
        assert_eq!(global_text(&interp, "r3"), "num");
    }

    #[tokio::test]
    async fn capitalized_primitive_annotations_dispatch() {
        // `as Number` / `as Text` normalize to the primitive types, so
        // runtime dispatch treats them like the lowercase spellings instead
        // of container types named "Number"/"Text" (PR #639 review).
        let interp = run(r#"
define action called f with parameters x as Number:
    return "num"
end action

define action called f with parameters x as Text:
    return "text"
end action

store r1 as f of 5
store r2 as f of "hello"
"#)
        .await
        .expect("program should run");
        assert_eq!(global_text(&interp, "r1"), "num");
        assert_eq!(global_text(&interp, "r2"), "text");
    }

    #[tokio::test]
    async fn both_call_forms_dispatch() {
        let interp = run(r#"
    define action called f with parameters x as number:
        return "num"
    end action

    define action called f with parameters x as text:
        return "text"
    end action

    store r1 as call f with 5
    store r2 as call f with "hello"
    "#)
        .await
        .expect("program should run");
        assert_eq!(global_text(&interp, "r1"), "num");
        assert_eq!(global_text(&interp, "r2"), "text");
    }

    #[tokio::test]
    async fn partially_annotated_same_arity_rejected() {
        // No parameter position where BOTH overloads declare concrete, different
        // types — some calls (e.g. `f of 1 and "b"`) would match both, so the
        // pair is rejected at definition time, mirroring the analyzer's rule.
        let err = run_err(
            r#"
    define action called f with parameters x as number and y:
        return "first"
    end action

    define action called f with parameters x and y as text:
        return "second"
    end action
    "#,
            "a partially-annotated indistinguishable pair must be rejected",
        )
        .await;
        assert!(
            err.contains('f') && err.contains("parameter"),
            "ambiguity error should explain the rule: {err}"
        );
    }

    #[tokio::test]
    async fn cross_annotated_same_arity_dispatch() {
        let interp = run(r#"
    define action called f with parameters x as number and y as text:
        return "first"
    end action

    define action called f with parameters x as text and y as number:
        return "second"
    end action

    store r1 as f of 1 and "b"
    store r2 as f of "a" and 2
    "#)
        .await
        .expect("program should run");
        assert_eq!(global_text(&interp, "r1"), "first");
        assert_eq!(global_text(&interp, "r2"), "second");
    }

    #[tokio::test]
    async fn no_match_runtime_error_lists_candidates() {
        let err = run_err(
            r#"
    define action called f with parameters x as number:
        return "num"
    end action

    define action called f with parameters x as text:
        return "text"
    end action

    store r as f of yes
    "#,
            "boolean argument should not match any overload",
        )
        .await;
        assert!(
            err.contains("No version of 'f'"),
            "runtime no-match error should list candidates: {err}"
        );
    }

    #[tokio::test]
    async fn wrong_arity_runtime_error() {
        let err = run_err(
            r#"
    define action called f with parameters x:
        return "one"
    end action

    define action called f with parameters x and y:
        return "two"
    end action

    store r as f of 1 and 2 and 3
    "#,
            "3 arguments should not match 1- or 2-parameter overloads",
        )
        .await;
        assert!(
            err.contains('f') && err.contains('3'),
            "wrong-arity error should mention the count: {err}"
        );
    }

    #[tokio::test]
    async fn overloaded_name_as_first_class_value() {
        let interp = run(r#"
    define action called f with parameters x as number:
        return "num"
    end action

    define action called f with parameters x as text:
        return "text"
    end action

    store h as f
    store r as h of 5
    "#)
        .await
        .expect("program should run");
        assert_eq!(global_text(&interp, "r"), "num");
    }

    #[tokio::test]
    async fn zero_arg_overload_auto_calls_on_bare_reference() {
        let interp = run(r#"
    define action called g:
        return 42
    end action

    define action called g with parameters x:
        return 0
    end action

    store r as g
    "#)
        .await
        .expect("program should run");
        assert_eq!(global_number(&interp, "r"), 42.0);
    }

    #[tokio::test]
    async fn of_form_call_dispatches_when_zero_arg_overload_present() {
        // Regression (PR #639 review): evaluating the callee of `g of 5` must not
        // auto-call the zero-argument overload and then try to call its result —
        // the overload set itself is the call target. Bare references still
        // auto-call the zero-argument version.
        let interp = run(r#"
    define action called g:
        return 1
    end action

    define action called g with parameters x:
        return x
    end action

    store r1 as g of 5
    store r2 as g
    "#)
        .await
        .expect("program should run");
        assert_eq!(global_number(&interp, "r1"), 5.0);
        assert_eq!(global_number(&interp, "r2"), 1.0);
    }

    #[tokio::test]
    async fn overload_closures_capture_definitions() {
        let interp = run(r#"
    store base as 10

    define action called f with parameters x as number:
        return base plus x
    end action

    define action called f with parameters x as text:
        return x
    end action

    store r as f of 5
    "#)
        .await
        .expect("program should run");
        assert_eq!(global_number(&interp, "r"), 15.0);
    }

    /// Interpreter call frames are very large in debug builds, so even shallow
    /// WFL recursion can overflow the default test-thread stack; give this test a
    /// generous one (the release binary used by TestPrograms has no such issue).
    #[test]
    fn recursion_across_sibling_overloads() {
        std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .spawn(|| {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime")
                    .block_on(async {
                        let interp = run(r#"
    define action called fact with parameters n:
        return fact of n and 1
    end action

    define action called fact with parameters n and acc:
        check if n is less than 2:
            return acc
        end check
        return fact of (n minus 1) and (acc times n)
    end action

    store r as fact of 5
    "#)
                        .await
                        .expect("program should run");
                        assert_eq!(global_number(&interp, "r"), 120.0);
                    })
            })
            .expect("spawn test thread")
            .join()
            .expect("test thread panicked");
    }

    #[tokio::test]
    async fn exact_duplicate_still_rejected_at_runtime() {
        let err = run_err(
            r#"
    define action called f with parameters x:
        return 1
    end action

    define action called f with parameters y:
        return 2
    end action
    "#,
            "an exact duplicate definition must stay an error",
        )
        .await;
        assert!(
            err.contains('f'),
            "duplicate-definition error should name the action: {err}"
        );
    }

    #[tokio::test]
    async fn nested_scope_shadowing_still_rejected() {
        let err = run_err(
            r#"
    define action called outer:
        define action called outer:
            return 1
        end action
        return 2
    end action

    store r as outer
    "#,
            "shadowing an outer action in a nested scope must stay an error",
        )
        .await;
        assert!(
            err.contains("already been defined"),
            "nested shadowing should keep today's error: {err}"
        );
    }
}

// Full-pipeline coverage (parse -> analyze -> typecheck -> interpret) for the
// maintainer deep-review findings on PR #639: container-typed overloads,
// `nothing` arguments, stored action aliases, temporal dispatch, and
// overload-set identity.
mod full_pipeline {
    use wfl::analyzer::Analyzer;
    use wfl::interpreter::Interpreter;
    use wfl::interpreter::value::Value;
    use wfl::lexer::lex_wfl_with_positions;
    use wfl::parser::Parser;
    use wfl::typechecker::TypeChecker;

    /// Runs the whole static pipeline and then the interpreter. Errors from
    /// any stage come back as a joined string naming the stage.
    async fn run_pipeline(code: &str) -> Result<Interpreter, String> {
        let tokens = lex_wfl_with_positions(code);
        let mut parser = Parser::new(&tokens);
        let program = parser.parse().map_err(|e| format!("parse: {e:?}"))?;

        let mut analyzer = Analyzer::new();
        if let Err(errors) = analyzer.analyze(&program) {
            return Err(format!(
                "analyze: {}",
                errors
                    .iter()
                    .map(|e| e.message.clone())
                    .collect::<Vec<_>>()
                    .join("; ")
            ));
        }

        let mut checker = TypeChecker::new();
        if let Err(err) = checker.check_types(&program) {
            return Err(format!(
                "typecheck: {}",
                err.into_diagnostics()
                    .iter()
                    .map(|e| e.message.clone())
                    .collect::<Vec<_>>()
                    .join("; ")
            ));
        }

        let mut interpreter = Interpreter::new();
        match interpreter.interpret(&program).await {
            Ok(_) => Ok(interpreter),
            Err(errors) => Err(format!(
                "runtime: {}",
                errors
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("; ")
            )),
        }
    }

    fn global_text(interpreter: &Interpreter, name: &str) -> String {
        match interpreter.global_env().borrow().get(name) {
            Some(Value::Text(t)) => t.to_string(),
            other => panic!("expected Text in '{name}', got {other:?}"),
        }
    }

    fn global_number(interpreter: &Interpreter, name: &str) -> f64 {
        match interpreter.global_env().borrow().get(name) {
            Some(Value::Number(n)) => n,
            other => panic!("expected Number in '{name}', got {other:?}"),
        }
    }

    fn global_bool(interpreter: &Interpreter, name: &str) -> bool {
        match interpreter.global_env().borrow().get(name) {
            Some(Value::Bool(b)) => b,
            other => panic!("expected Bool in '{name}', got {other:?}"),
        }
    }

    #[tokio::test]
    async fn container_typed_overloads_dispatch() {
        let interp = run_pipeline(
            r#"
create container Dog:
    property tag: Text
end

create container Cat:
    property tag: Text
end

define action called depict with parameters value as Dog:
    return "dog"
end action

define action called depict with parameters value as Cat:
    return "cat"
end action

create new Dog as rover:
    tag is "rover"
end

create new Cat as felix:
    tag is "felix"
end

store r1 as depict of rover
store r2 as depict of felix
"#,
        )
        .await
        .expect("container-typed overloads must pass the whole pipeline");
        assert_eq!(global_text(&interp, "r1"), "dog");
        assert_eq!(global_text(&interp, "r2"), "cat");
    }

    #[tokio::test]
    async fn inherited_container_instance_matches_parent_param() {
        let interp = run_pipeline(
            r#"
create container Dog:
    property tag: Text
end

create container Puppy extends Dog:
    property toy: Text
end

define action called depict with parameters value as Dog:
    return "dog"
end action

define action called depict with parameters value as number:
    return "number"
end action

create new Puppy as pip:
    tag is "pip"
    toy is "ball"
end

store r as depict of pip
"#,
        )
        .await
        .expect("a descendant instance must match a parent-typed parameter");
        assert_eq!(global_text(&interp, "r"), "dog");
    }

    #[tokio::test]
    async fn nothing_argument_dispatches_by_definition_order() {
        let interp = run_pipeline(
            r#"
define action called f with parameters x as number:
    return "number body"
end action

define action called f with parameters x as text:
    return "text body"
end action

store r as f of nothing
"#,
        )
        .await
        .expect("`nothing` must stay compatible with every overload at runtime");
        assert_eq!(global_text(&interp, "r"), "number body");
    }

    #[tokio::test]
    async fn stored_action_alias_full_pipeline() {
        let interp = run_pipeline(
            r#"
define action called f with parameters x as number:
    return x plus 1
end action

define action called f with parameters x as text:
    return "text"
end action

store h as f
store r as h of 5
store s as r plus 1
"#,
        )
        .await
        .expect("calling an overload set through a stored variable must pass the whole pipeline");
        assert_eq!(global_number(&interp, "r"), 6.0);
        assert_eq!(global_number(&interp, "s"), 7.0);
    }

    #[tokio::test]
    async fn alias_invalidated_on_reassignment() {
        let code = r#"
define action called f with parameters x as number:
    return 1
end action

define action called f with parameters x as text:
    return "t"
end action

store h as f
change h to 5
store r as h of 1
"#;
        let tokens = lex_wfl_with_positions(code);
        let mut parser = Parser::new(&tokens);
        let program = parser.parse().expect("parse");
        let mut analyzer = Analyzer::new();
        let result = analyzer.analyze(&program);
        let errors: Vec<String> = match result {
            Ok(()) => Vec::new(),
            Err(errors) => errors.into_iter().map(|e| e.message).collect(),
        };
        assert!(
            errors.iter().any(|e| e.contains("not a function")),
            "a reassigned alias must stop being callable: {errors:?}"
        );
    }

    #[tokio::test]
    async fn interleaved_call_rejected_at_runtime() {
        // Statically the call resolves to the later text overload, but at
        // runtime only the number overload exists yet — the call must be
        // rejected, never silently run the number body with a text argument.
        let err = run_pipeline(
            r#"
define action called choose with parameters value as number:
    return "number body"
end action

store selected as choose of "hello"

define action called choose with parameters value as text:
    return "text body"
end action
"#,
        )
        .await
        .err()
        .expect("an interleaved call against the wrong lone overload must error");
        assert!(
            err.starts_with("runtime:") && err.contains("expects"),
            "the rejection should be a runtime type error: {err}"
        );
    }

    #[tokio::test]
    async fn call_after_both_definitions_dispatches_correctly() {
        let interp = run_pipeline(
            r#"
define action called choose with parameters value as number:
    return "number body"
end action

define action called choose with parameters value as text:
    return "text body"
end action

store selected as choose of "hello"
"#,
        )
        .await
        .expect("the same call after both definitions must dispatch");
        assert_eq!(global_text(&interp, "selected"), "text body");
    }

    #[tokio::test]
    async fn overloaded_value_self_identity() {
        let interp = run_pipeline(
            r#"
define action called f with parameters x as number:
    return "number"
end action

define action called f with parameters x as text:
    return "text"
end action

define action called g with parameters x as number:
    return "g-number"
end action

define action called g with parameters x as text:
    return "g-text"
end action

store first_alias as f
store second_alias as g
store identical as first_alias is equal to first_alias
store distinct as first_alias is equal to second_alias
"#,
        )
        .await
        .expect("overload-set identity comparisons must run");
        assert!(
            global_bool(&interp, "identical"),
            "an overload set must be equal to itself"
        );
        assert!(
            !global_bool(&interp, "distinct"),
            "distinct overload sets must not be equal"
        );
    }
}
