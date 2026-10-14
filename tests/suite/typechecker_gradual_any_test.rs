use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::parser::ast::{Argument, Expression, Literal, Program, Statement};
use wfl::typechecker::{TypeCheckError, TypeChecker};

fn typecheck(source: &str) -> Result<(), TypeCheckError> {
    let program = parse_program(source);
    TypeChecker::new().check_types(&program)
}

fn parse_program(source: &str) -> Program {
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    parser.parse().expect("test program should parse")
}

fn typecheck_program(program: &Program) -> Result<(), TypeCheckError> {
    TypeChecker::new().check_types(program)
}

#[test]
fn any_values_defer_operation_specific_validation_to_runtime() {
    for source in [
        r#"
store paths as ["data.txt"]
open file at paths[0] for reading as input_file
"#,
        r#"
store conditions as [yes]
check if conditions[0]:
    display "dynamic condition"
end check
"#,
        r#"
store bounds as [1, 2]
count from bounds[0] to bounds[1]:
    display count
end count
"#,
    ] {
        typecheck(source).unwrap_or_else(|failure| {
            panic!(
                "Any is statically unknown and must be checked at runtime: {:?}",
                failure.into_diagnostics()
            )
        });
    }
}

#[test]
fn concrete_invalid_values_are_still_rejected() {
    for source in [
        "open file at 1 for reading as input_file\n",
        "check if 1:\n    display \"never\"\nend check\n",
        "count from \"one\" to 2:\n    display count\nend count\n",
    ] {
        assert!(
            typecheck(source).is_err(),
            "a concrete incompatible type must remain a static error: {source}"
        );
    }
}

#[test]
fn homogeneous_list_literals_preserve_their_element_type() {
    assert!(
        typecheck("store values as [1, 2]\ncreate directory at values[0]\n").is_err(),
        "a homogeneous Number literal must not erase its element type to Any"
    );

    typecheck("store values as [1, \"dynamic\"]\ncreate directory at values[0]\n")
        .expect("a genuinely heterogeneous literal widens to Any");
}

#[test]
fn element_preserving_list_builtins_keep_concrete_types() {
    for expression in [
        "random_from of values",
        "pop of values",
        "shift of values",
        "remove_at of values and 0",
    ] {
        let source = format!(
            "store values as [1, 2]\nstore result as {expression}\ncreate directory at result\n"
        );
        assert!(
            typecheck(&source).is_err(),
            "{expression} must preserve the known Number element type"
        );
    }

    assert!(
        typecheck(
            "store values as [1, 2]\nstore result as slice of values and 0 and 1\ncreate directory at result[0]\n",
        )
        .is_err(),
        "slice must preserve the known Number element type"
    );
}

#[test]
fn find_result_requires_a_nothing_guard() {
    let number = |value| Expression::Literal(Literal::Integer(value), 1, 1);
    let program = Program {
        statements: vec![
            Statement::VariableDeclaration {
                name: "values".to_string(),
                value: Expression::Literal(Literal::List(vec![number(1), number(2)]), 1, 1),
                is_constant: false,
                line: 1,
                column: 1,
            },
            Statement::VariableDeclaration {
                name: "match_value".to_string(),
                value: Expression::FunctionCall {
                    function: Box::new(Expression::Variable("find".to_string(), 2, 1)),
                    arguments: vec![
                        Argument {
                            name: None,
                            value: Expression::Variable("values".to_string(), 2, 1),
                        },
                        Argument {
                            name: None,
                            value: number(3),
                        },
                    ],
                    line: 2,
                    column: 1,
                },
                is_constant: false,
                line: 2,
                column: 1,
            },
            Statement::CreateDirectoryStatement {
                path: Expression::Variable("match_value".to_string(), 3, 1),
                line: 3,
                column: 1,
            },
        ],
    };
    typecheck_program(&program)
        .expect_err("find returns Number or Nothing, not an unrestricted gradual value");
}

#[test]
fn path_params_result_requires_a_nothing_guard() {
    typecheck(
        "store params as path_params of \"/posts/42\" and \"/users/:id\"\n\
         store invalid as params minus 1\n",
    )
    .expect_err("path_params returns a capture map or Nothing, never an arbitrary scalar");
}

#[test]
fn shape_preserving_builtins_return_lists_for_gradual_inputs() {
    for operation in ["slice of dynamic and 0 and 1", "unique of dynamic"] {
        let source = format!(
            "store dynamic as parse_json of \"[1]\"\n\
             store result as {operation}\n\
             store invalid as result minus 1\n"
        );
        assert!(
            typecheck(&source).is_err(),
            "{operation} produces a list when it succeeds, not a top-level Any"
        );
    }
}

#[test]
fn mutating_list_builtins_widen_the_bound_list() {
    typecheck(
        "store values as [1]\npush with values and \"text\"\ncreate directory at values[1]\n",
    )
    .expect("a heterogeneous builtin push widens the list element type to Any");
}

#[test]
fn fill_replaces_the_list_element_type_instead_of_joining_it() {
    assert!(
        typecheck(
            "store values as [1]\n\
             store ignored as fill of values and \"text\"\n\
             store invalid as values[0] minus 1\n",
        )
        .is_err(),
        "fill overwrites every element, so the resulting element type is Text"
    );
}

#[test]
fn statement_push_through_an_alias_updates_the_original_list_type() {
    typecheck(
        "store values as [1]\n\
         store alias_values as values\n\
         push with alias_values and \"text\"\n\
         store removed as pop of values\n\
         open file at removed for reading as input_file\n",
    )
    .expect("the original and alias share one runtime list allocation");
}

#[test]
fn mutating_builtins_through_an_alias_update_element_return_types() {
    for (mutation, removal) in [
        (
            "store ignored as push of alias_values and \"text\"",
            "pop of values",
        ),
        (
            "store ignored as unshift of alias_values and \"text\"",
            "shift of values",
        ),
        (
            "store ignored as insert_at of alias_values and 0 and \"text\"",
            "remove_at of values and 0",
        ),
        (
            "store ignored as fill of alias_values and \"text\"",
            "pop of values",
        ),
    ] {
        let source = format!(
            "store values as [1]\n\
             store alias_values as values\n\
             {mutation}\n\
             store removed as {removal}\n\
             open file at removed for reading as input_file\n"
        );
        typecheck(&source).unwrap_or_else(|failure| {
            panic!(
                "{mutation} mutates the allocation read by {removal}: {:?}",
                failure.into_diagnostics()
            )
        });
    }
}

#[test]
fn alias_mutations_retain_exact_types_when_the_effect_is_homogeneous_or_replacing() {
    let homogeneous = typecheck(
        "store values as [1]\n\
         store alias_values as values\n\
         store ignored as push of alias_values and 2\n\
         create directory at values[1]\n",
    )
    .expect_err("pushing another Number keeps both aliases at List<Number>")
    .into_diagnostics();
    assert!(
        homogeneous
            .iter()
            .any(|error| error.found == Some(wfl::parser::ast::Type::Number)),
        "expected the original alias to retain Number, got {homogeneous:?}"
    );

    let replaced = typecheck(
        "store values as [1]\n\
         store alias_values as values\n\
         store ignored as fill of alias_values and \"text\"\n\
         store invalid as values[0] minus 1\n",
    )
    .expect_err("fill replaces every element through every alias")
    .into_diagnostics();
    assert!(
        replaced
            .iter()
            .any(|error| error.found == Some(wfl::parser::ast::Type::Text)),
        "expected the original alias to become List<Text>, got {replaced:?}"
    );
}

#[test]
fn copying_a_list_alias_does_not_widen_it_before_a_mutation() {
    let diagnostics = typecheck(
        "store values as [1]\n\
         store alias_values as values\n\
         create directory at values[0]\n",
    )
    .expect_err("an ordinary alias copy must retain List<Number> precision")
    .into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|error| error.found == Some(wfl::parser::ast::Type::Number)),
        "expected the copied list to remain List<Number>, got {diagnostics:?}"
    );
}

#[test]
fn list_alias_mutation_does_not_widen_an_unrelated_list() {
    let diagnostics = typecheck(
        "store values as [1]\n\
         store alias_values as values\n\
         store unrelated as [2]\n\
         push with alias_values and \"text\"\n\
         create directory at unrelated[0]\n",
    )
    .expect_err("mutating one alias group must not widen another list")
    .into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|error| error.found == Some(wfl::parser::ast::Type::Number)),
        "expected the unrelated list to remain List<Number>, got {diagnostics:?}"
    );
}

#[test]
fn action_parameter_list_mutation_widens_only_the_passed_list() {
    typecheck(
        "define action called append_text with parameters items:\n\
             push with items and \"text\"\n\
         end action\n\
         store values as [1]\n\
         call append_text with values\n\
         store removed as pop of values\n\
         open file at removed for reading as input_file\n",
    )
    .expect("a list passed to a user action can be mutated through its parameter");

    let diagnostics = typecheck(
        "define action called append_text with parameters items:\n\
             push with items and \"text\"\n\
         end action\n\
         store values as [1]\n\
         store unrelated as [2]\n\
         call append_text with values\n\
         create directory at unrelated[0]\n",
    )
    .expect_err("an action escape must not widen lists that were not passed")
    .into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|error| error.found == Some(wfl::parser::ast::Type::Number)),
        "expected the unrelated list to remain List<Number>, got {diagnostics:?}"
    );
}

#[test]
fn of_form_action_list_arguments_cross_the_same_escape_boundary() {
    typecheck(
        "define action called append_text with parameters items:\n\
             push with items and \"text\"\n\
         end action\n\
         store values as [1]\n\
         store ignored as append_text of values\n\
         store removed as pop of values\n\
         open file at removed for reading as input_file\n",
    )
    .expect("the of-form user-action call can mutate its list argument");
}

#[test]
fn dynamically_resolved_action_calls_escape_list_arguments() {
    for invocation in [
        "call selected with values",
        "store ignored as selected of values",
    ] {
        let source = format!(
            "define action called append_text with parameters items:\n\
                 push with items and \"text\"\n\
             end action\n\
             define action called leave_unchanged with parameters items:\n\
                 display items\n\
             end action\n\
             store selected as leave_unchanged\n\
             check if yes:\n\
                 change selected to append_text\n\
             end check\n\
             store values as [1]\n\
             {invocation}\n\
             store removed as pop of values\n\
             open file at removed for reading as input_file\n"
        );
        typecheck(&source)
            .expect("a dynamically resolved user action may mutate its list argument");
    }
}

#[test]
fn nested_list_target_mutation_updates_only_the_affected_root_path() {
    typecheck(
        "store nested as [[1]]\n\
         store unrelated as [2]\n\
         push with nested[0] and \"text\"\n\
         store removed as pop of nested[0]\n\
         open file at removed for reading as input_file\n",
    )
    .expect("mutating an indexed inner list must update the nested root type");

    let diagnostics = typecheck(
        "store nested as [[1]]\n\
         store unrelated as [2]\n\
         push with nested[0] and \"text\"\n\
         create directory at unrelated[0]\n",
    )
    .expect_err("nested-root mutation must not widen an unrelated list")
    .into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|error| error.found == Some(wfl::parser::ast::Type::Number)),
        "expected the unrelated list to remain List<Number>, got {diagnostics:?}"
    );
}

#[test]
fn extracting_a_nested_list_uses_a_conservative_alias_escape() {
    typecheck(
        "store nested as [[1]]\n\
         store inner_values as nested[0]\n\
         push with inner_values and \"text\"\n\
         store removed as pop of nested[0]\n\
         open file at removed for reading as input_file\n",
    )
    .expect("a nested list extraction shares the inner runtime allocation");
}

#[test]
fn control_flow_mutation_propagates_across_a_list_alias_group() {
    typecheck(
        "store values as [1]\n\
         store alias_values as values\n\
         check if yes:\n\
             push with alias_values and \"text\"\n\
         end check\n\
         store removed as pop of values\n\
         open file at removed for reading as input_file\n",
    )
    .expect("a branch mutation can affect the allocation read through the original alias");
}

#[test]
fn add_to_list_through_an_alias_updates_the_original_list_type() {
    typecheck(
        "store values as [1]\n\
         store alias_values as values\n\
         add \"text\" to alias_values\n\
         store removed as pop of values\n\
         open file at removed for reading as input_file\n",
    )
    .expect("the add-to-list statement mutates the allocation shared by both aliases");
}

#[test]
fn reassignment_can_create_a_list_alias_group() {
    typecheck(
        "store source_values as [1]\n\
         store alias_values as [2]\n\
         change alias_values to source_values\n\
         push with alias_values and \"text\"\n\
         store removed as pop of source_values\n\
         open file at removed for reading as input_file\n",
    )
    .expect("reassigning a list variable can make it alias the source allocation");
}

#[test]
fn reassignment_from_a_gradual_binding_can_create_a_list_alias_group() {
    typecheck(
        "store source_values as [1]\n\
         store alias_values as parse_json of \"null\"\n\
         change alias_values to source_values\n\
         push with alias_values and \"text\"\n\
         store removed as pop of source_values\n\
         open file at removed for reading as input_file\n",
    )
    .expect("a definite list assignment replaces a formerly gradual runtime binding");
}

#[test]
fn promoted_try_handler_aliases_retain_their_alias_group() {
    for mutation in [
        "push with alias_values and \"text\"",
        "add \"text\" to alias_values",
        "store mutation_result as push of alias_values and \"text\"",
        "store mutation_result as fill of alias_values and \"text\"",
    ] {
        let source = format!(
            "store values as [1]\n\
             try:\n\
                 store alias_values as values\n\
                 store ignored as 1 divided by 0\n\
             when error:\n\
                 store alias_values as values\n\
             finally:\n\
                 {mutation}\n\
             end try\n\
             store removed as pop of values\n\
             open file at removed for reading as input_file\n"
        );
        typecheck(&source)
            .expect("a handler-local alias promoted into finally must still alias its source");
    }
}

#[test]
fn promoted_gradual_aliases_escape_through_user_actions() {
    typecheck(
        "define action called mutate with parameters items:\n\
             push with items and \"text\"\n\
         end action\n\
         store values as [1]\n\
         try:\n\
             store alias_values as values\n\
             store ignored as 1 divided by 0\n\
         when error:\n\
             store alias_values as values\n\
         finally:\n\
             call mutate with alias_values\n\
         end try\n\
         store removed as pop of values\n\
         open file at removed for reading as input_file\n",
    )
    .expect("a promoted gradual alias passed to user code can mutate its source allocation");
}

#[test]
fn promoted_branch_aliases_retain_their_alias_group() {
    typecheck(
        "store values as [1]\n\
         check if yes:\n\
             store alias_values as values\n\
         otherwise:\n\
             store alias_values as values\n\
         end check\n\
         push with alias_values and \"text\"\n\
         store removed as pop of values\n\
         open file at removed for reading as input_file\n",
    )
    .expect("an alias established on every branch remains linked after promotion");
}

#[test]
fn loop_rechecks_preserve_list_alias_effects() {
    for source in [
        "store values as [1]\n\
         store alias_values as values\n\
         repeat while yes:\n\
             push with alias_values and \"text\"\n\
             break\n\
         end repeat\n\
         store removed as pop of values\n\
         open file at removed for reading as input_file\n",
        "store values as [1]\n\
         count from 1 to 2:\n\
             store iteration_alias as values\n\
             push with iteration_alias and \"text\"\n\
         end count\n\
         store removed as pop of values\n\
         open file at removed for reading as input_file\n",
    ] {
        typecheck(source)
            .expect("fixed-point and fresh-iteration rechecks must retain may-alias effects");
    }
}

#[test]
fn sequential_loop_local_names_do_not_merge_distinct_alias_groups() {
    let diagnostics = typecheck(
        "store first_values as [1]\n\
         store second_values as [2]\n\
         count from 1 to 1:\n\
             store iteration_alias as first_values\n\
         end count\n\
         count from 1 to 1:\n\
             store iteration_alias as second_values\n\
             push with iteration_alias and \"text\"\n\
         end count\n\
         create directory at first_values[0]\n",
    )
    .expect_err("separate loop scopes may reuse a local name without alias-key collision")
    .into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|error| error.found == Some(wfl::parser::ast::Type::Number)),
        "expected the first loop's source to retain List<Number>, got {diagnostics:?}"
    );
}

#[test]
fn branch_local_alias_names_do_not_merge_with_later_bindings() {
    for source in [
        "store first_values as [1]\n\
         check if no:\n\
             store alias_values as first_values\n\
         end check\n\
         store second_values as [2]\n\
         store alias_values as second_values\n\
         push with alias_values and \"text\"\n\
         create directory at first_values[0]\n",
        "store first_values as [1]\n\
         store second_values as [2]\n\
         check if yes:\n\
             store alias_values as first_values\n\
         otherwise:\n\
             store alias_values as second_values\n\
         end check\n\
         push with alias_values and \"text\"\n\
         create directory at second_values[0]\n",
    ] {
        let diagnostics = typecheck(source)
            .expect_err("an unreachable alias branch must not widen an unrelated list")
            .into_diagnostics();
        assert!(
            diagnostics
                .iter()
                .any(|error| error.found == Some(wfl::parser::ast::Type::Number)),
            "only the reachable literal branch may contribute alias effects: {diagnostics:?}"
        );
    }
}

#[test]
fn checker_reuse_does_not_leak_list_alias_groups_between_programs() {
    let first = parse_program(
        "store values as [1]\n\
         store alias_values as values\n\
         push with alias_values and \"text\"\n\
         store removed as pop of values\n\
         open file at removed for reading as input_file\n",
    );
    let second = parse_program("store values as [1]\ncreate directory at values[0]\n");
    let mut checker = TypeChecker::new();

    checker
        .check_types(&first)
        .expect("the first program's alias mutation should be soundly widened");
    let diagnostics = checker
        .check_types(&second)
        .expect_err("the second program must infer its fresh list independently")
        .into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|error| error.found == Some(wfl::parser::ast::Type::Number)),
        "expected checker reuse to retain fresh List<Number>, got {diagnostics:?}"
    );
}

#[test]
fn structured_collection_joins_preserve_outer_shape() {
    assert!(
        typecheck(
            "store nested as [[1], [\"text\"]]\n\
             store invalid as nested[0] minus 1\n",
        )
        .is_err(),
        "joining nested list elements must produce List<List<Any>>, not List<Any>"
    );

    assert!(
        typecheck(
            "store values as [1]\n\
             store condition as yes\n\
             check if condition:\n\
                 push with values and \"text\"\n\
             end check\n\
             store invalid as values minus 1\n",
        )
        .is_err(),
        "a control-flow join must preserve that values is still a list"
    );
}

#[test]
fn nested_list_extraction_escapes_both_alias_views() {
    typecheck(
        "store nested as [[1]]\n\
         store inner_values as nested[0]\n\
         push with nested[0] and \"text\"\n\
         store removed as pop of inner_values\n\
         open file at removed for reading as input_file\n",
    )
    .expect("mutating a nested list path must widen a previously extracted shared list");
}

#[test]
fn whole_list_aliases_lift_to_nested_paths() {
    typecheck(
        "store nested as [[1]]\n\
         store alias_nested as nested\n\
         push with alias_nested[0] and \"text\"\n\
         store removed as pop of nested[0]\n\
         open file at removed for reading as input_file\n",
    )
    .expect("whole-list aliases share every nested Rc path");
}

#[test]
fn extracted_aliases_translate_deeper_nested_paths() {
    typecheck(
        "store nested as [[[1]]]\n\
         store inner_values as nested[0]\n\
         push with inner_values[0] and \"text\"\n\
         store removed as pop of nested[0][0]\n\
         open file at removed for reading as input_file\n",
    )
    .expect("mutations below an extracted alias retain their relative path depth");
}

#[test]
fn nested_list_extraction_preserves_precision_until_a_mutation() {
    for source in [
        "store nested as [[1]]\ncreate directory at nested[0][0]\n",
        "store nested as [[1]]\n\
         store inner_values as nested[0]\n\
         create directory at inner_values[0]\n",
    ] {
        let diagnostics = typecheck(source)
            .expect_err("alias creation alone cannot make a known Number gradual")
            .into_diagnostics();
        assert!(
            diagnostics
                .iter()
                .any(|error| error.found == Some(wfl::parser::ast::Type::Number)),
            "the untouched nested element must remain Number: {diagnostics:?}"
        );
    }
}

#[test]
fn aggregate_literals_retain_nested_list_alias_provenance() {
    typecheck(
        "store values as [1]\n\
         store nested as [values]\n\
         push with nested[0] and \"text\"\n\
         store removed as pop of values\n\
         open file at removed for reading as input_file\n",
    )
    .expect("list literals shallow-clone nested list Rc values");
}

#[test]
fn aggregate_alias_paths_do_not_merge_distinct_elements() {
    let diagnostics = typecheck(
        "store first_values as [1]\n\
         store second_values as [2]\n\
         store nested as [first_values, second_values]\n\
         push with first_values and \"text\"\n\
         create directory at second_values[0]\n",
    )
    .expect_err("different aggregate elements are distinct list allocations")
    .into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|error| error.found == Some(wfl::parser::ast::Type::Number)),
        "mutating one nested list must not widen its sibling: {diagnostics:?}"
    );
}

#[test]
fn deeply_nested_aggregate_aliases_retain_leaf_provenance() {
    typecheck(
        "store leaf_values as [1]\n\
         store inner_values as [leaf_values]\n\
         store outer_values as [inner_values]\n\
         push with outer_values[0][0] and \"text\"\n\
         store removed as pop of leaf_values\n\
         open file at removed for reading as input_file\n",
    )
    .expect("nested aggregate paths must retain the shared leaf list allocation");
}

#[test]
fn inserted_lists_retain_runtime_alias_provenance() {
    typecheck(
        "store inner_values as [1]\n\
         store outer_values as []\n\
         push with outer_values and inner_values\n\
         push with outer_values[0] and \"text\"\n\
         store removed as pop of inner_values\n\
         open file at removed for reading as input_file\n",
    )
    .expect("pushing a list shallow-clones its shared runtime allocation");
}

#[test]
fn aggregate_self_reassignment_preserves_descendant_aliases() {
    typecheck(
        "store leaf_values as [1]\n\
         store outer_values as [leaf_values]\n\
         change outer_values to outer_values\n\
         push with outer_values[0] and \"text\"\n\
         store removed as pop of leaf_values\n\
         open file at removed for reading as input_file\n",
    )
    .expect("self-reassignment must retain descendant list provenance");
}

#[test]
fn clearing_an_aggregate_detaches_stale_descendant_aliases() {
    let diagnostics = typecheck(
        "store leaf_values as [1]\n\
         store outer_values as [leaf_values]\n\
         clear outer_values\n\
         push with outer_values and [2]\n\
         push with outer_values[0] and \"text\"\n\
         create directory at leaf_values[0]\n",
    )
    .expect_err("clearing the aggregate removes its old nested allocation")
    .into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|error| error.found == Some(wfl::parser::ast::Type::Number)),
        "the detached leaf must remain List<Number>: {diagnostics:?}"
    );
}

#[test]
fn user_action_results_do_not_hide_shared_list_mutations() {
    typecheck(
        "store values as [1]\n\
         define action called get_values:\n\
             return values\n\
         end action\n\
         store alias_values as call get_values\n\
         push with values and \"text\"\n\
         store removed as pop of alias_values\n\
         open file at removed for reading as input_file\n",
    )
    .expect("a list returned by user code may share its runtime allocation with captured state");
}

#[test]
fn user_actions_escape_captured_lists_they_can_mutate() {
    typecheck(
        "store values as [1]\n\
         define action called mutate:\n\
             push with values and \"text\"\n\
         end action\n\
         call mutate\n\
         store removed as pop of values\n\
         open file at removed for reading as input_file\n",
    )
    .expect("calling user code must account for mutations to captured runtime lists");
}

#[test]
fn bare_zero_argument_action_statements_apply_captured_list_effects() {
    typecheck(
        "store values as [1]\n\
         define action called mutate:\n\
             push with values and \"text\"\n\
         end action\n\
         mutate\n\
         store removed as pop of values\n\
         open file at removed for reading as input_file\n",
    )
    .expect("bare zero-argument action statements execute and may mutate captured lists");
}

#[test]
fn forward_action_calls_propagate_captured_list_effects() {
    typecheck(
        "store values as [1]\n\
         define action called first:\n\
             call later\n\
         end action\n\
         define action called later:\n\
             push with values and \"text\"\n\
         end action\n\
         call first\n\
         store removed as pop of values\n\
         open file at removed for reading as input_file\n",
    )
    .expect("effect summaries must not depend on action definition order");
}

#[test]
fn reassignment_detaches_a_binding_from_its_previous_list_alias() {
    let diagnostics = typecheck(
        "store first_values as [1]\n\
         store alias_values as first_values\n\
         change alias_values to [2]\n\
         push with alias_values and \"text\"\n\
         create directory at first_values[0]\n",
    )
    .expect_err("replacing one alias must leave the original List<Number> precise")
    .into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|error| error.found == Some(wfl::parser::ast::Type::Number)),
        "the detached source must retain its Number element type: {diagnostics:?}"
    );
}

#[test]
fn self_reassignment_preserves_existing_runtime_aliases() {
    typecheck(
        "store values as [1]\n\
         store alias_values as values\n\
         change alias_values to alias_values\n\
         push with alias_values and \"text\"\n\
         store removed as pop of values\n\
         open file at removed for reading as input_file\n",
    )
    .expect("self-assignment retains the same shared list allocation");
}

#[test]
fn branch_alias_join_retains_every_runtime_alias_path() {
    typecheck(
        "store first_values as [1]\n\
         store alias_values as first_values\n\
         store second_values as [2]\n\
         store flag as yes\n\
         check if flag:\n\
             change alias_values to second_values\n\
         end check\n\
         push with alias_values and \"text\"\n\
         store removed as pop of first_values\n\
         open file at removed for reading as input_file\n",
    )
    .expect("the implicit else path still aliases the first list at runtime");
}

#[test]
fn branch_alias_join_does_not_invent_transitive_aliases() {
    let diagnostics = typecheck(
        "store first_values as [1]\n\
         store alias_values as first_values\n\
         store second_values as [2]\n\
         store flag as yes\n\
         check if flag:\n\
             change alias_values to second_values\n\
         end check\n\
         push with first_values and \"text\"\n\
         create directory at second_values[0]\n",
    )
    .expect_err("the first and second lists are distinct on every runtime path")
    .into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|error| error.found == Some(wfl::parser::ast::Type::Number)),
        "the unrelated second list must remain List<Number>: {diagnostics:?}"
    );
}

#[test]
fn loop_alias_join_retains_the_zero_iteration_path() {
    typecheck(
        "store first_values as [1]\n\
         store alias_values as first_values\n\
         store second_values as [2]\n\
         store flag as yes\n\
         repeat while flag:\n\
             change alias_values to second_values\n\
             break\n\
         end repeat\n\
         push with alias_values and \"text\"\n\
         store removed as pop of first_values\n\
         open file at removed for reading as input_file\n",
    )
    .expect("a maybe-empty loop must retain aliases from its zero-iteration path");
}

#[test]
fn foreach_list_items_retain_nested_alias_provenance() {
    typecheck(
        "store nested as [[1]]\n\
         for each inner_values in nested:\n\
             push with inner_values and \"text\"\n\
         end for\n\
         store removed as pop of nested[0]\n\
         open file at removed for reading as input_file\n",
    )
    .expect("for-each clones the inner list Rc, so item mutations affect the collection");
}

#[test]
fn checking_an_uncalled_action_does_not_change_runtime_aliases() {
    typecheck(
        "store values as [1]\n\
         store alias_values as values\n\
         define action called uncalled:\n\
             change alias_values to [2]\n\
         end action\n\
         push with values and \"text\"\n\
         store removed as pop of alias_values\n\
         open file at removed for reading as input_file\n",
    )
    .expect("a deferred action body has no effects until the action is called");
}

#[test]
fn action_return_joins_preserve_collection_shape() {
    assert!(
        typecheck(
            "define action called choose with parameters condition as boolean:\n\
                 check if condition:\n\
                     return [1]\n\
                 otherwise:\n\
                     return [\"text\"]\n\
                 end check\n\
             end action\n\
             store result as choose of yes\n\
             store invalid as result minus 1\n",
        )
        .is_err(),
        "differing list returns must join to List<Any>, not top-level Any"
    );
}

#[test]
fn any_typed_container_method_parameters_accept_concrete_arguments() {
    typecheck(
        r#"
create container Sink:
    action take needs value: any:
        display value
    end
end

create new Sink as sink:
end

sink.take(1)
sink.take("text")
"#,
    )
    .unwrap_or_else(|failure| {
        panic!(
            "Any method parameters must accept concrete values: {:?}",
            failure.into_diagnostics()
        )
    });
}

#[test]
fn heterogeneous_collection_inference_is_order_independent() {
    for source in [
        r#"
create list values:
    add parse_json of "1"
    add 1
end list
create directory at values[0]
"#,
        r#"
create list values:
    add 1
    add parse_json of "1"
end list
create directory at values[0]
"#,
        r#"
create map values:
    dynamic is parse_json of "1"
    fixed is 1
end map
create directory at values["dynamic"]
"#,
        r#"
create map values:
    fixed is 1
    dynamic is parse_json of "1"
end map
create directory at values["dynamic"]
"#,
    ] {
        typecheck(source).unwrap_or_else(|failure| {
            panic!(
                "Any must dominate a heterogeneous collection join regardless of order: {:?}",
                failure.into_diagnostics()
            )
        });
    }
}

#[test]
fn unknown_collection_elements_do_not_narrow_to_later_concrete_values() {
    let source = r#"
define action called collect with mystery:
    create list values:
        add mystery
        add 1
    end list
    create directory at values[0]
end action
"#;
    typecheck(source).unwrap_or_else(|failure| {
        panic!(
            "an unresolved element keeps the collection gradual: {:?}",
            failure.into_diagnostics()
        )
    });
}

#[test]
fn heterogeneous_appends_widen_lists_instead_of_rejecting_them() {
    for append in ["push with values and \"text\"", "add \"text\" to values"] {
        let source = format!(
            "create list values:\n\
             \x20\x20\x20\x20add 1\n\
             end list\n\
             {append}\n\
             create directory at values[1]\n"
        );
        typecheck(&source).unwrap_or_else(|failure| {
            panic!(
                "WFL lists are heterogeneous, so append must widen to Any: {:?}",
                failure.into_diagnostics()
            )
        });
    }
}

#[test]
fn push_visits_both_operands_for_semantic_errors() {
    let failure = typecheck("push with missing_list and missing_value\n")
        .expect_err("undefined push operands must not be skipped");
    let messages: Vec<_> = failure
        .into_diagnostics()
        .into_iter()
        .map(|error| error.message)
        .collect();
    assert!(
        messages
            .iter()
            .any(|message| message.contains("missing_list"))
            && messages
                .iter()
                .any(|message| message.contains("missing_value")),
        "both operands must be analyzed: {messages:?}"
    );
}

#[test]
fn captured_scalar_action_effect_invalidates_optional_narrowing() {
    let failure = typecheck(
        r#"
define action called maybe_label with parameters enabled as boolean:
    check if enabled:
        return "ready"
    end check
end action

store label as call maybe_label with yes
define action called clear_label:
    change label to nothing
end action

check if label is not nothing:
    call clear_label
    open file at label for reading as input_file
end check
"#,
    )
    .expect_err("the action can invalidate the guarded Text refinement");
    assert!(
        failure
            .into_diagnostics()
            .iter()
            .any(|error| error.message.contains("File path")),
        "expected the post-call Optional<Text> to fail the file-path contract"
    );
}

#[test]
fn stored_zero_argument_action_applies_captured_scalar_effects() {
    let failure = typecheck(
        r#"
define action called maybe_label with parameters enabled as boolean:
    check if enabled:
        return "ready"
    end check
end action

store label as call maybe_label with yes
define action called clear_label with parameters ignored as number:
    change label to nothing
end action
store clearer as clear_label

check if label is not nothing:
    call clearer with 1
    open file at label for reading as input_file
end check
"#,
    )
    .expect_err("stored action aliases must carry captured scalar effects");
    assert!(
        failure
            .into_diagnostics()
            .iter()
            .any(|error| error.message.contains("File path")),
        "expected the stored call to invalidate the Text refinement"
    );
}

#[test]
fn forward_action_calls_propagate_captured_scalar_effects() {
    let failure = typecheck(
        r#"
define action called maybe_label with parameters enabled as boolean:
    check if enabled:
        return "ready"
    end check
end action

store label as call maybe_label with yes
define action called clear_through_helper:
    call clear_label
end action
define action called clear_label:
    change label to nothing
end action

check if label is not nothing:
    call clear_through_helper
    open file at label for reading as input_file
end check
"#,
    )
    .expect_err("forward action dependencies must carry scalar effects");
    assert!(
        failure
            .into_diagnostics()
            .iter()
            .any(|error| error.message.contains("File path")),
        "expected the forward call graph to invalidate the Text refinement"
    );
}

#[test]
fn opaque_static_method_calls_invalidate_optional_narrowing() {
    let failure = typecheck(
        r#"
define action called maybe_label with parameters enabled as boolean:
    check if enabled:
        return "ready"
    end check
end action
store label as call maybe_label with yes

create container Mutator:
    static action reset_label:
        change label to nothing
    end
end

check if label is not nothing:
    Mutator.reset_label()
    open file at label for reading as input_file
end check
"#,
    )
    .expect_err("an opaque method can invalidate a captured Optional guard");
    assert!(
        failure
            .into_diagnostics()
            .iter()
            .any(|error| error.message.contains("File path")),
        "expected the method boundary to restore Optional<Text>"
    );
}
