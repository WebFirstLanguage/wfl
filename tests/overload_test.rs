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

    // Round 6, finding 2: block-entry arming is speculative — if the run
    // fails before the merging definition executes, the still-single action
    // must return to legacy dynamic-call leniency for later runs (a REPL
    // interpreter is reused across snippets).
    #[tokio::test]
    async fn speculative_arming_reverts_when_merge_never_executes() {
        async fn run_snippet(interpreter: &mut Interpreter, code: &str) -> Result<(), String> {
            let tokens = lex_wfl_with_positions(code);
            let mut parser = Parser::new(&tokens);
            let program = parser
                .parse()
                .unwrap_or_else(|e| panic!("Failed to parse {code:?}: {e:?}"));
            interpreter
                .interpret(&program)
                .await
                .map(|_| ())
                .map_err(|errors| {
                    errors
                        .iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<_>>()
                        .join("; ")
                })
        }

        let mut interpreter = Interpreter::new();
        run_snippet(
            &mut interpreter,
            r#"
define action called f with parameters x as number:
    return "num body"
end action
"#,
        )
        .await
        .expect("snippet 1 defines a lone typed action");

        run_snippet(
            &mut interpreter,
            r#"
store bad as 10 divided by 0
define action called f with parameters t as text:
    return "text body"
end action
"#,
        )
        .await
        .expect_err("snippet 2 fails before the merge executes");

        run_snippet(
            &mut interpreter,
            r#"
define action called caller with parameters passthrough:
    return f of passthrough
end action

store out as caller of "lenient"
"#,
        )
        .await
        .expect("a never-merged action must keep legacy dynamic-call leniency");
        match interpreter.global_env().borrow().get("out") {
            Some(Value::Text(t)) => assert_eq!(t.to_string(), "num body"),
            other => panic!("expected Text in 'out', got {other:?}"),
        }
    }

    // Round 4, finding 3: runtime enforcement must be scoped to the block
    // that actually overloads the name. A sibling block's lone typed action
    // keeps legacy dynamic-call leniency even though another block overloads
    // the same name. (Interpreter-only: static analysis of nested action
    // bodies is out of scope here — this pins the runtime rule.)
    #[tokio::test]
    async fn sibling_block_overload_does_not_guard_lone_action() {
        let interp = run(r#"
define action called block_a:
    define action called helper with parameters x as number:
        return x plus 1
    end action
    define action called helper with parameters t as text:
        return t
    end action
    return helper of 5
end action

define action called block_b with parameters raw_input:
    define action called helper with parameters x as number:
        return x
    end action
    return helper of raw_input
end action

store outcome as block_b of "lenient"
"#)
        .await
        .expect("a lone typed action in a sibling block must keep dynamic-call leniency");
        assert_eq!(global_text(&interp, "outcome"), "lenient");
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

    // A `nothing` argument is accepted by every parameter type but must not
    // earn specificity credit for a typed parameter — otherwise a version
    // with more annotations beats the documented definition-order rule.
    #[tokio::test]
    async fn nothing_specificity_falls_to_definition_order() {
        let interp = run_pipeline(
            r#"
define action called pick with parameters a as number and b:
    return "first"
end action

define action called pick with parameters a as text and b as number:
    return "second"
end action

store r as pick of nothing and nothing
"#,
        )
        .await
        .expect("nothing must be accepted by every version");
        assert_eq!(
            global_text(&interp, "r"),
            "first",
            "nothing must not add specificity; definition order decides"
        );
    }

    // The one parameter type a `nothing` argument matches *exactly* is an
    // explicit `as nothing` — that version is more specific for it.
    #[tokio::test]
    async fn explicit_nothing_overload_wins_specificity() {
        let interp = run_pipeline(
            r#"
define action called pick with parameters a as number:
    return "num"
end action

define action called pick with parameters a as nothing:
    return "none"
end action

store r as pick of nothing
"#,
        )
        .await
        .expect("an explicit nothing overload must be callable");
        assert_eq!(
            global_text(&interp, "r"),
            "none",
            "an explicit 'as nothing' parameter matches nothing exactly"
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

    // --- Round-3 deep-review regressions ---------------------------------------

    // Finding 1: an alias stored between two overload definitions captures a
    // snapshot — statically it must see only the overloads defined before the
    // `store`, matching the runtime value.
    #[tokio::test]
    async fn alias_snapshot_between_definitions_rejects_later_overload() {
        let err = match run_pipeline(
            r#"
define action called choose with parameters value as number:
    return "number"
end action

store saved as choose

define action called choose with parameters value as text:
    return "text"
end action

store result as saved of "hello"
"#,
        )
        .await
        {
            Ok(_) => panic!("a text call through a number-only snapshot must be rejected"),
            Err(err) => err,
        };
        assert!(
            err.starts_with("analyze:") || err.starts_with("typecheck:"),
            "rejection must come from static analysis, not the runtime: {err}"
        );
    }

    #[tokio::test]
    async fn alias_snapshot_between_definitions_accepts_visible_overload() {
        let interp = run_pipeline(
            r#"
define action called choose with parameters value as number:
    return "number"
end action

store saved as choose

define action called choose with parameters value as text:
    return "text"
end action

store result as saved of 5
"#,
        )
        .await
        .expect("a call matching the snapshot overload must pass the whole pipeline");
        assert_eq!(global_text(&interp, "result"), "number");
    }

    // Finding 2: the typechecker must resolve aliases in the `call ... with`
    // form, not just the `of` form.
    #[tokio::test]
    async fn alias_call_with_form_typechecks() {
        let interp = run_pipeline(
            r#"
define action called f with parameters x as number:
    return x plus 1
end action

define action called f with parameters x as text:
    return "text"
end action

store h as f
store r as call h with 5
store s as r plus 1
"#,
        )
        .await
        .expect("call-with through an alias must analyze, typecheck, and run");
        assert_eq!(global_number(&interp, "s"), 7.0);
    }

    // Finding 2 companion: the same alias resolution must also *reject*
    // statically-invalid `call ... with` arguments, not just accept valid ones.
    #[tokio::test]
    async fn alias_call_with_form_rejects_type_mismatch() {
        let err = match run_pipeline(
            r#"
define action called f with parameters x as number:
    return x plus 1
end action

store h as f
store r as call h with "hello"
"#,
        )
        .await
        {
            Ok(_) => panic!("call-with through an alias must reject a mismatched argument"),
            Err(err) => err,
        };
        assert!(
            err.starts_with("analyze:") || err.starts_with("typecheck:"),
            "the rejection should come from static analysis: {err}"
        );
    }

    // Finding 3: alias state must not be mutated by code that has not
    // executed. (Reassigning to another action is used here because
    // `change h to 0` on a function-typed variable is a pre-existing static
    // type error unrelated to aliases.)
    #[tokio::test]
    async fn uncalled_body_does_not_clobber_alias() {
        let interp = run_pipeline(
            r#"
define action called f with parameters x as number:
    return x plus 1
end action

define action called f with parameters x as text:
    return "text"
end action

define action called g with parameters y as text:
    return "gee"
end action

store h as f

define action called never_called:
    change h to g
end action

store r as h of 5
"#,
        )
        .await
        .expect("an uncalled body must not remap the outer alias");
        assert_eq!(global_number(&interp, "r"), 6.0);
    }

    // Finding 3: a branch that may or may not run degrades the alias to
    // "dynamic" — later calls defer to runtime instead of erroring.
    #[tokio::test]
    async fn conditional_reassignment_degrades_alias() {
        let interp = run_pipeline(
            r#"
define action called f with parameters x as number:
    return x plus 1
end action

define action called f with parameters x as text:
    return "text"
end action

define action called g with parameters y as text:
    return "gee"
end action

store h as f
store flag as no

check if flag:
    change h to g
otherwise:
    display "kept"
end check

store r as h of 5
"#,
        )
        .await
        .expect("an alias modified in an untaken branch must still be callable");
        assert_eq!(global_number(&interp, "r"), 6.0);
    }

    // Finding 3: the typechecker must observe alias state at each statement,
    // not the analyzer's final table.
    #[tokio::test]
    async fn call_before_later_reassignment_typechecks() {
        let interp = run_pipeline(
            r#"
define action called f with parameters x as number:
    return x plus 1
end action

define action called f with parameters x as text:
    return "text"
end action

define action called g with parameters y as text:
    return "gee"
end action

store h as f
store r as h of 5
change h to g
store s as r plus 1
"#,
        )
        .await
        .expect("a call before a later reassignment must stay valid");
        assert_eq!(global_number(&interp, "s"), 7.0);
    }

    // Round 4, finding 1: an overload defined inside a branch must not leak
    // into the visible-signature prefix a later alias captures. Whether the
    // branch ran is unknowable statically, so the alias defers wholly to
    // runtime dispatch — the rejection below must come from the runtime, not
    // from static validation against a leaked (and here future-including)
    // signature prefix.
    #[tokio::test]
    async fn branch_defined_overload_does_not_leak_into_alias_prefix() {
        let err = match run_pipeline(
            r#"
define action called f with parameters x as number:
    return x plus 1
end action

store flag as no
check if flag:
    define action called f with parameters t as text:
        return t
    end action
end check

store h as f
store r as h of yes

define action called f with parameters t as text:
    return t
end action
"#,
        )
        .await
        {
            Ok(_) => panic!("a call matching no runtime member must fail"),
            Err(err) => err,
        };
        assert!(
            err.starts_with("runtime:"),
            "a conditionally-extended alias must defer to runtime dispatch, got: {err}"
        );
    }

    // Round 4, finding 1 (guardrail — the maintainer's exact program): the
    // alias holds only the number member at runtime, so the text call fails
    // at dispatch even though a text overload exists lexically later.
    #[tokio::test]
    async fn branch_defined_overload_alias_fails_at_runtime_dispatch() {
        let err = match run_pipeline(
            r#"
define action called f with parameters x as number:
    return x plus 1
end action

store flag as no
check if flag:
    define action called f with parameters t as text:
        return t
    end action
end check

store h as f
store r as h of "hello"

define action called f with parameters t as text:
    return t
end action
"#,
        )
        .await
        {
            Ok(_) => panic!("the alias must not see the branch or future overload"),
            Err(err) => err,
        };
        assert!(
            err.starts_with("runtime:"),
            "expected a runtime dispatch failure, got: {err}"
        );
    }

    // Round 4, finding 1 (loop variant): loop bodies may run zero times, so a
    // definition inside one poisons the counter the same way a branch does.
    #[tokio::test]
    async fn loop_defined_overload_does_not_leak_into_alias_prefix() {
        let err = match run_pipeline(
            r#"
define action called f with parameters x as number:
    return x plus 1
end action

store limit as 0
count from 1 to limit:
    define action called f with parameters t as text:
        return t
    end action
end count

store h as f
store r as h of yes

define action called f with parameters t as text:
    return t
end action
"#,
        )
        .await
        {
            Ok(_) => panic!("a call matching no runtime member must fail"),
            Err(err) => err,
        };
        assert!(
            err.starts_with("runtime:"),
            "a loop-extended alias must defer to runtime dispatch, got: {err}"
        );
    }

    // Round 4, finding 2: temporal dispatch enforcement must reach tests
    // nested under `describe`. An interleaved wrong-type call between two
    // same-block definitions must be rejected, not run the wrong body.
    #[tokio::test]
    async fn describe_nested_interleaved_call_rejected() {
        let code = r#"
describe "temporal":
    test "interleaved call":
        define action called choose with parameters value as number:
            return "number body"
        end action
        store selected as choose of "hello"
        define action called choose with parameters value as text:
            return "text body"
        end action
    end test
end describe
"#;
        let tokens = lex_wfl_with_positions(code);
        let mut parser = Parser::new(&tokens);
        let program = parser.parse().expect("parse");

        let mut analyzer = Analyzer::new();
        analyzer.analyze(&program).expect("analyze");
        let mut checker = TypeChecker::new();
        checker.check_types(&program).expect("typecheck");

        let mut interpreter = Interpreter::new();
        interpreter.set_test_mode(true);
        interpreter.interpret(&program).await.expect("interpret");

        let results = interpreter.get_test_results();
        assert_eq!(
            results.failed_tests, 1,
            "the interleaved call inside a describe-nested test must fail"
        );
        assert!(
            results
                .failures
                .first()
                .is_some_and(|f| f.assertion_message.contains("expects")),
            "the failure must be the temporal dispatch rejection: {:?}",
            results.failures
        );
    }

    // Round 4 (drift guard): an alias taken inside a branch can see more
    // definitions than PASS 1 registered top-level signatures. Clamping to a
    // shorter prefix would statically reject a call that runtime dispatch
    // accepts — the alias must go dynamic instead.
    #[tokio::test]
    async fn in_branch_alias_over_counter_drift_defers_to_runtime() {
        let interp = run_pipeline(
            r#"
define action called f with parameters x as number:
    return x plus 1
end action

store flag as yes
check if flag:
    define action called f with parameters t as text:
        return t
    end action
    store h as f
    store r as h of "hello"
end check
"#,
        )
        .await
        .expect("an in-branch alias seeing extra definitions must defer to runtime");
        assert_eq!(global_text(&interp, "r"), "hello");
    }

    // Round 5, finding 1: a block whose definition will merge with an
    // existing same-scope action must arm that member's enforcement at block
    // entry — an interleaved dynamic call before the in-block definition
    // must not run the wrong body during the temporal window.
    #[tokio::test]
    async fn cross_block_merge_arms_existing_member_at_entry() {
        let err = match run_pipeline(
            r#"
define action called f with parameters x as number:
    return "num body"
end action

define action called caller with parameters passthrough:
    return f of passthrough
end action

store flag as yes
check if flag:
    store mid as caller of "sneaky"
    define action called f with parameters t as text:
        return "text body"
    end action
end check
"#,
        )
        .await
        {
            Ok(_) => panic!("the interleaved call must hit temporal enforcement"),
            Err(err) => err,
        };
        assert!(
            err.starts_with("runtime:") && err.contains("expects"),
            "expected the temporal dispatch rejection, got: {err}"
        );
    }

    // Round 5, finding 2 (loop): a `break` can exit a loop while an alias
    // holds an intermediate binding the body later restores. Endpoint-only
    // joins would keep the restored binding and statically reject a call the
    // runtime accepts — mutated aliases must degrade to runtime dispatch.
    #[tokio::test]
    async fn alias_mutated_in_loop_with_break_defers_to_runtime() {
        let interp = run_pipeline(
            r#"
define action called f with parameters x as number:
    return x plus 1
end action

define action called f with parameters b as boolean:
    return "eff bool"
end action

define action called g with parameters t:
    return "gee text"
end action

store h as f
store limit as 1
count from 1 to limit:
    change h to g
    break
    change h to f
end count
store r as h of "boom"
"#,
        )
        .await
        .expect("the break-time binding accepts this call; static analysis must defer");
        assert_eq!(global_text(&interp, "r"), "gee text");
    }

    // Round 5, finding 2 (try): an error can transfer to a `when` handler
    // while an alias holds an intermediate binding the body later restores.
    // The handler must see mutated aliases as dynamic, not the endpoint state.
    #[tokio::test]
    async fn alias_mutated_in_try_body_is_dynamic_in_handler() {
        let interp = run_pipeline(
            r#"
define action called f with parameters x as number:
    return x plus 1
end action

define action called f with parameters b as boolean:
    return "eff bool"
end action

define action called g with parameters t:
    return "gee text"
end action

store h as f
store r as "unset"
try:
    change h to g
    store bad as 10 divided by 0
    change h to f
when error:
    change r to h of "boom"
end try
"#,
        )
        .await
        .expect("the throw-time binding accepts this call; static analysis must defer");
        assert_eq!(global_text(&interp, "r"), "gee text");
    }

    // Round 6, finding 1: every loop variant participates in alias flow
    // analysis — a `repeat while` body mutating an alias must degrade it to
    // runtime dispatch just like the other loop forms.
    #[tokio::test]
    async fn alias_mutated_in_repeat_while_defers_to_runtime() {
        let interp = run_pipeline(
            r#"
define action called f with parameters x as number:
    return x plus 1
end action

define action called f with parameters b as boolean:
    return "eff bool"
end action

define action called g with parameters t:
    return "gee text"
end action

store h as f
repeat while yes:
    change h to g
    break
    change h to f
end repeat
store r as h of "boom"
"#,
        )
        .await
        .expect("the break-time binding accepts this call; static analysis must defer");
        assert_eq!(global_text(&interp, "r"), "gee text");
    }

    // Round 6, finding 1 (main loop variant): same rule for `main loop`.
    #[tokio::test]
    async fn alias_mutated_in_main_loop_defers_to_runtime() {
        let interp = run_pipeline(
            r#"
define action called f with parameters x as number:
    return x plus 1
end action

define action called f with parameters b as boolean:
    return "eff bool"
end action

define action called g with parameters t:
    return "gee text"
end action

store h as f
main loop:
    change h to g
    break
    change h to f
end loop
store r as h of "boom"
"#,
        )
        .await
        .expect("the break-time binding accepts this call; static analysis must defer");
        assert_eq!(global_text(&interp, "r"), "gee text");
    }

    // Finding 4: a single (non-overloaded) typed action keeps its historical
    // dynamic-call behavior — annotations are not runtime guards for it.
    #[tokio::test]
    async fn legacy_single_typed_action_dynamic_call_runs() {
        let interp = run_pipeline(
            r#"
define action called render with parameters value as number:
    return "value: " with value
end action

define action called forward with parameters dynamic_value:
    return render of dynamic_value
end action

store out as forward of "hello"
"#,
        )
        .await
        .expect("a legacy single typed action must accept dynamic calls at runtime");
        assert_eq!(global_text(&interp, "out"), "value: hello");
    }
}
