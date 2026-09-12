//! New default native names must remain available to existing user declarations.
mod common;

use std::rc::Rc;
use wfl::interpreter::Interpreter;
use wfl::interpreter::environment::Environment;
use wfl::interpreter::value::{FunctionValue, Value};

const NEW_BUILTINS: &[&str] = &[
    "password_hash_policy",
    "hash_password_with_policy",
    "password_needs_rehash",
    "create_session_store",
    "session_create",
    "session_lookup",
    "session_rotate",
    "session_revoke",
    "session_revoke_account",
    "session_csrf_guard",
    "session_cookie",
    "create_account_rate_limiter",
    "account_rate_limit_allow",
];

fn text(value: &str) -> Value {
    Value::Text(value.into())
}

#[test]
fn global_constants_named_after_new_builtins_execute_in_the_real_binary() {
    let mut failures = Vec::new();
    for name in NEW_BUILTINS {
        let source =
            format!("store new constant {name} as \"prefix\"\ndisplay {name} with \"value\"\n");
        let (output, status) = common::run_src(&source);
        if status != Some(0) || output.trim() != "prefixvalue" {
            failures.push(format!("{name}: status={status:?}, output={output}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[tokio::test]
async fn action_local_constants_shadow_only_the_inherited_default_native() {
    let mut failures = Vec::new();
    for name in NEW_BUILTINS {
        let source = format!(
            "define action called local_label:\n    store new constant {name} as \"prefix\"\n    return {name} with \"value\"\nend action\nstore result as call local_label\n"
        );
        match common::run_wfl(&source).await {
            Ok(interpreter) => {
                assert_eq!(
                    common::get_global(&interpreter, "result"),
                    text("prefixvalue")
                );
                assert!(matches!(
                    common::get_global(&interpreter, name),
                    Value::NativeFunction(native_name, _) if native_name == *name
                ));
            }
            Err(error) => failures.push(format!("{name}: {error}")),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn inherited_default_native_can_be_shadowed_without_mutating_its_owner() {
    let mut failures = Vec::new();
    for name in NEW_BUILTINS {
        for isolated in [false, true] {
            let interpreter = Interpreter::new();
            let global = interpreter.global_env();
            let outer = Environment::new(&global);
            let child = if isolated {
                Environment::new_isolated_child_env(&outer)
            } else {
                Environment::new(&outer)
            };
            let result = child
                .borrow_mut()
                .declare_variable(name, text("local"), true);
            if let Err(error) = result {
                failures.push(format!("{name}, isolated={isolated}: {error}"));
                continue;
            }
            assert_eq!(child.borrow().get_local(name), Some(text("local")));
            assert!(child.borrow().is_constant(name));
            assert!(outer.borrow().get_local(name).is_none());
            assert!(matches!(
                global.borrow().get(name),
                Some(Value::NativeFunction(_, _))
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn user_values_and_constants_keep_duplicate_and_mutation_protection() {
    for name in NEW_BUILTINS {
        for constant in [false, true] {
            // A user-created binding has no default-native provenance, even if
            // its spelling is one of the newly registered names.
            let parent = Environment::new_global();
            parent
                .borrow_mut()
                .declare_variable(name, text("user"), constant)
                .unwrap();
            assert!(
                parent
                    .borrow_mut()
                    .declare_variable(name, text("replacement"), true)
                    .is_err()
            );
            let child = Environment::new(&parent);
            assert!(
                child
                    .borrow_mut()
                    .declare_variable(name, text("replacement"), true)
                    .is_err()
            );
            if constant {
                assert!(
                    parent
                        .borrow_mut()
                        .declare_variable(name, text("replacement"), false)
                        .is_err()
                );
                assert!(
                    child
                        .borrow_mut()
                        .assign(name, text("replacement"))
                        .is_err()
                );
            }
            assert_eq!(parent.borrow().get(name), Some(text("user")));
        }
    }
}

#[test]
fn explicitly_stored_native_aliases_are_user_bindings_not_default_fallbacks() {
    for name in NEW_BUILTINS {
        let interpreter = Interpreter::new();
        let global = interpreter.global_env();
        let alias = global.borrow().get(name).unwrap();
        global
            .borrow_mut()
            .declare_variable(name, alias, false)
            .unwrap();
        assert!(
            global
                .borrow_mut()
                .declare_variable(name, text("replacement"), true)
                .is_err()
        );
        let child = Environment::new(&global);
        assert!(
            child
                .borrow_mut()
                .declare_variable(name, text("replacement"), true)
                .is_err()
        );
    }
}

#[test]
fn user_actions_named_after_new_builtins_execute_in_the_real_binary() {
    let mut failures = Vec::new();
    for name in NEW_BUILTINS {
        let source = format!(
            "define action called {name} with parameters value:\n    return \"user:\" with value\nend action\ndisplay call {name} with \"value\"\n"
        );
        let (output, status) = common::run_src(&source);
        if status != Some(0) || output.trim() != "user:value" {
            failures.push(format!("{name}: status={status:?}, output={output}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn user_overloads_named_after_new_builtins_support_of_and_explicit_calls() {
    let mut failures = Vec::new();
    for name in NEW_BUILTINS {
        let source = format!(
            "define action called {name} with parameters value:\n    return \"one:\" with value\nend action\ndefine action called {name} with parameters left_value and right_value:\n    return \"two:\" with left_value with right_value\nend action\ndisplay {name} of \"value\"\ndisplay call {name} with \"value\" and \"extra\"\n"
        );
        let (output, status) = common::run_src(&source);
        if status != Some(0) || output.trim() != "one:value\ntwo:valueextra" {
            failures.push(format!("{name}: status={status:?}, output={output}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn user_action_aliases_named_after_new_builtins_support_both_call_forms() {
    let mut failures = Vec::new();
    for name in NEW_BUILTINS {
        let source = format!(
            "define action called user_action with parameters value:\n    return \"user:\" with value\nend action\nstore {name} as user_action\ndisplay {name} of \"value\"\ndisplay call {name} with \"value\"\n"
        );
        let (output, status) = common::run_src(&source);
        if status != Some(0) || output.trim() != "user:value\nuser:value" {
            failures.push(format!("{name}: status={status:?}, output={output}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn unshadowed_and_self_aliased_new_natives_keep_arity_checks() {
    for source in [
        "store bad as call session_create\n",
        "store bad as session_create of \"value\"\n",
        "store session_create as session_create\nstore bad as call session_create\n",
    ] {
        let tokens = wfl::lexer::lex_wfl_with_positions(source);
        let program = wfl::parser::Parser::new(&tokens).parse().unwrap();
        let errors = wfl::typechecker::TypeChecker::new()
            .check_types(&program)
            .expect_err("native arity must still be checked")
            .into_diagnostics();
        assert!(
            errors
                .iter()
                .any(|error| error.message.contains("expects 2 arguments")),
            "{source}: {errors:?}"
        );
    }
}

#[test]
fn local_user_action_named_after_a_new_builtin_uses_its_own_contract() {
    let source = "define action called local_runner:\n    define action called session_create with parameters label:\n        return \"local:\" with label\n    end action\n    return (session_create of \"first\") with (call session_create with \"second\")\nend action\ndisplay call local_runner\n";
    let (output, status) = common::run_src(source);
    assert_eq!(status, Some(0), "{output}");
    assert_eq!(output.trim(), "local:firstlocal:second");
}

#[tokio::test]
async fn action_local_user_actions_shadow_inherited_defaults_and_keep_overloads() {
    let mut failures = Vec::new();
    for name in NEW_BUILTINS {
        let source = format!(
            "define action called local_label:\n    define action called {name}:\n        return \"zero\"\n    end action\n    define action called {name} with parameters value:\n        return \"one:\" with value\n    end action\n    return (call {name}) with (call {name} with \"value\")\nend action\nstore result as call local_label\n"
        );
        match common::run_wfl(&source).await {
            Ok(interpreter) => {
                assert_eq!(
                    common::get_global(&interpreter, "result"),
                    text("zeroone:value")
                );
                assert!(matches!(
                    common::get_global(&interpreter, name),
                    Value::NativeFunction(_, _)
                ));
            }
            Err(error) => failures.push(format!("{name}: {error}")),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

async fn sample_action() -> Rc<FunctionValue> {
    let interpreter =
        common::run_wfl("define action called sample:\n    return \"user\"\nend action\n")
            .await
            .unwrap();
    let Value::Function(action) = common::get_global(&interpreter, "sample") else {
        panic!("sample action must be callable")
    };
    action
}

#[tokio::test]
async fn user_action_redeclarations_keep_existing_collision_rules() {
    let action = sample_action().await;
    for name in NEW_BUILTINS {
        for constant in [false, true] {
            let global = Environment::new_global();
            global
                .borrow_mut()
                .declare_variable(name, text("user"), constant)
                .unwrap();
            assert!(
                global
                    .borrow_mut()
                    .define_or_merge_action(name, Rc::clone(&action))
                    .is_err()
            );
            let child = Environment::new(&global);
            assert!(
                child
                    .borrow_mut()
                    .define_or_merge_action(name, Rc::clone(&action))
                    .is_err()
            );
            assert_eq!(global.borrow().get(name), Some(text("user")));
        }
        let global = Environment::new_global();
        global
            .borrow_mut()
            .define_or_merge_action(name, Rc::clone(&action))
            .unwrap();
        assert!(
            global
                .borrow_mut()
                .define_or_merge_action(name, Rc::clone(&action))
                .is_err()
        );
        let child = Environment::new(&global);
        assert!(
            child
                .borrow_mut()
                .define_or_merge_action(name, Rc::clone(&action))
                .is_err()
        );
    }
}

#[test]
fn every_user_write_consumes_default_native_provenance() {
    for name in NEW_BUILTINS {
        for write in [
            "declare",
            "define",
            "define_direct",
            "assign",
            "parent_assign",
            "replace",
            "constant",
            "constant_direct",
        ] {
            let interpreter = Interpreter::new();
            let global = interpreter.global_env();
            let alias = global.borrow().get(name).unwrap();
            match write {
                "declare" => global
                    .borrow_mut()
                    .declare_variable(name, alias, false)
                    .unwrap(),
                "define" => global.borrow_mut().define(name, alias).unwrap(),
                "define_direct" => global.borrow_mut().define_direct(name, alias).unwrap(),
                "assign" => global.borrow_mut().assign(name, alias).unwrap(),
                "parent_assign" => Environment::new(&global)
                    .borrow_mut()
                    .assign(name, alias)
                    .unwrap(),
                "replace" => global.borrow_mut().define_or_replace(name, alias),
                "constant" => global.borrow_mut().define_constant(name, alias).unwrap(),
                "constant_direct" => global
                    .borrow_mut()
                    .define_constant_direct(name, alias)
                    .unwrap(),
                _ => unreachable!(),
            }
            assert!(
                global
                    .borrow_mut()
                    .declare_variable(name, text("replacement"), true)
                    .is_err(),
                "{write} must consume the default marker for {name}"
            );
            let child = Environment::new(&global);
            assert!(
                child
                    .borrow_mut()
                    .define_constant(name, text("replacement"))
                    .is_err()
            );
        }
    }
}

#[test]
fn temporary_bindings_restore_the_original_default_or_user_provenance() {
    for name in NEW_BUILTINS {
        for user_claimed in [false, true] {
            let interpreter = Interpreter::new();
            let global = interpreter.global_env();
            if user_claimed {
                let alias = global.borrow().get(name).unwrap();
                global
                    .borrow_mut()
                    .declare_variable(name, alias, false)
                    .unwrap();
            }
            let saved = global.borrow_mut().take_local_binding(name);
            global
                .borrow_mut()
                .define_or_replace(name, text("temporary"));
            global.borrow_mut().restore_local_binding(name, saved);
            assert!(matches!(
                global.borrow().get(name),
                Some(Value::NativeFunction(_, _))
            ));
            let result = global
                .borrow_mut()
                .declare_variable(name, text("constant"), true);
            assert_eq!(result.is_err(), user_claimed, "{name}: {result:?}");
        }
    }
}

#[test]
fn clearing_a_scope_cannot_revive_default_provenance_for_a_user_alias() {
    for name in NEW_BUILTINS {
        let interpreter = Interpreter::new();
        let global = interpreter.global_env();
        let alias = global.borrow().get(name).unwrap();
        global.borrow_mut().clear();
        global.borrow_mut().define_direct(name, alias).unwrap();
        assert!(
            global
                .borrow_mut()
                .declare_variable(name, text("replacement"), true)
                .is_err()
        );
    }
}

#[test]
fn local_scalar_declarations_leave_native_calls_in_other_scopes_intact() {
    let interpreter = Interpreter::new();
    let global = interpreter.global_env();
    let child = Environment::new(&global);
    child
        .borrow_mut()
        .declare_variable("session_cookie", text("local"), false)
        .unwrap();
    assert_eq!(
        child.borrow().get_local("session_cookie"),
        Some(text("local"))
    );
    let isolated = Environment::new_isolated_child_env(&global);
    let Value::NativeFunction(_, native) = isolated.borrow().get("session_cookie").unwrap() else {
        panic!("isolated lookup must keep the unshadowed native callable")
    };
    let token = "a".repeat(64);
    assert_eq!(
        native(vec![text(&token)]).unwrap(),
        text(&format!(
            "__Host-wfl_session={token}; Path=/; Secure; HttpOnly; SameSite=Strict"
        ))
    );
}

#[tokio::test]
async fn legacy_native_declaration_rules_are_unchanged() {
    let action = sample_action().await;
    for name in ["abs", "count", "hash_password"] {
        let interpreter = Interpreter::new();
        let global = interpreter.global_env();
        assert!(
            global
                .borrow_mut()
                .declare_variable(name, text("constant"), true)
                .is_err()
        );
        assert!(
            global
                .borrow_mut()
                .define_or_merge_action(name, Rc::clone(&action))
                .is_err()
        );
        let child = Environment::new(&global);
        assert!(
            child
                .borrow_mut()
                .declare_variable(name, text("constant"), true)
                .is_err()
        );
        // Existing mutable stores keep their historical ancestor-assignment behavior.
        child
            .borrow_mut()
            .declare_variable(name, text("assigned"), false)
            .unwrap();
        assert_eq!(global.borrow().get(name), Some(text("assigned")));
        assert!(child.borrow().get_local(name).is_none());
    }
}
