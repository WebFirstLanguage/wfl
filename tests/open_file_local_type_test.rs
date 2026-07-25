//! Regression coverage for local `open file ... as ...` bindings that analyzer
//! body scopes do not retain for the type-checker pass.

use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::typechecker::TypeChecker;

fn typecheck(source: &str) -> Result<(), String> {
    let program = Parser::new(&lex_wfl_with_positions(source))
        .parse()
        .expect("parse");
    TypeChecker::new()
        .check_types(&program)
        .map_err(|errors| format!("{errors:?}"))
}

#[test]
fn fresh_local_file_handles_are_concrete_in_action_loop_and_method_scopes() {
    let sources = [
        (
            "action",
            "define action called dump:\n\
             \x20\x20\x20\x20open file at \"unused.txt\" for writing as out\n\
             \x20\x20\x20\x20store line value as \"classic\"\n\
             \x20\x20\x20\x20write line value to out\n\
             \x20\x20\x20\x20close out\n\
             end action\n",
        ),
        (
            "main loop",
            "main loop:\n\
             \x20\x20\x20\x20open file at \"unused.txt\" for writing as out\n\
             \x20\x20\x20\x20store line value as \"classic\"\n\
             \x20\x20\x20\x20write line value to out\n\
             \x20\x20\x20\x20close out\n\
             \x20\x20\x20\x20break\n\
             end loop\n",
        ),
        (
            "container method",
            "create container Writer:\n\
             \x20\x20\x20\x20action dump:\n\
             \x20\x20\x20\x20\x20\x20\x20\x20open file at \"unused.txt\" for writing as out\n\
             \x20\x20\x20\x20\x20\x20\x20\x20store line value as \"classic\"\n\
             \x20\x20\x20\x20\x20\x20\x20\x20write line value to out\n\
             \x20\x20\x20\x20\x20\x20\x20\x20close out\n\
             \x20\x20\x20\x20end\n\
             end\n",
        ),
    ];

    for (scope, source) in sources {
        assert!(
            typecheck(source).is_ok(),
            "a fresh File handle in {scope} must select only the classic write \
             reading; errors: {:?}\nsource:\n{source}",
            typecheck(source).err()
        );
    }
}

#[test]
fn opening_a_local_file_shadows_instead_of_retyping_an_outer_binding() {
    let source = "store out as \"outer.txt\"\n\
                  main loop:\n\
                  \x20\x20\x20\x20open file at \"inner.txt\" for writing as out\n\
                  \x20\x20\x20\x20close out\n\
                  \x20\x20\x20\x20break\n\
                  end loop\n\
                  close out\n";
    let errors =
        typecheck(source).expect_err("the outer Text binding must remain Text after the loop");
    assert!(
        errors.contains("file or stream handle") || errors.contains("File"),
        "expected the outer Text/handle diagnostic, got: {errors}"
    );
}
