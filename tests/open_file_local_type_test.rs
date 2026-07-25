//! Regression coverage for local `open file ... as ...` bindings that analyzer
//! body scopes do not retain for the type-checker pass.
//!
//! These type-checker tests do not claim that runtime file bindings can shadow
//! an outer variable: `Environment::define` rejects parent-scope collisions.

use std::fs;
use std::process::Command;
use tempfile::TempDir;
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
fn reconstructed_local_file_type_does_not_retype_an_outer_visible_binding() {
    // The type checker must reconstruct `out` as File while checking the loop,
    // then expose the original outer Text binding after leaving that scope.
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

#[test]
fn ambiguous_write_uses_the_classic_branch_for_a_real_open_file_handle() {
    let temp = TempDir::new().expect("tempdir");
    let program_path = temp.path().join("program.wfl");
    let output_path = temp.path().join("actual-output.txt");
    let wfl_output_path = output_path.to_string_lossy().replace('\\', "/");
    let source = format!(
        "define action called dump:\n\
         \x20\x20\x20\x20open file at \"{wfl_output_path}\" for writing as out\n\
         \x20\x20\x20\x20store line value as \"classic\"\n\
         \x20\x20\x20\x20write line value to out\n\
         \x20\x20\x20\x20close out\n\
         end action\n\
         call dump\n"
    );
    fs::write(&program_path, source).expect("write WFL program");

    let output = Command::new(env!("CARGO_BIN_EXE_wfl"))
        .arg(&program_path)
        .output()
        .expect("run WFL program");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "a real opened File handle must select the classic write branch; output:\n{combined}"
    );
    assert_eq!(
        fs::read_to_string(&output_path).expect("read output file"),
        "classic",
        "the classic fallback content must be written to the opened handle"
    );
}
