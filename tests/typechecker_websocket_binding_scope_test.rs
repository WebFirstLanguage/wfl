//! Red→Green regression for issue #642: the WebSocket handler binding must be
//! recreated (and shadow any outer same-named symbol) in the type checker's
//! handler scope.
//!
//! Runtime binds the event object with `define_direct`, deliberately
//! shadowing an outer variable of the same name. The #641 typechecker pass
//! added the handler scope but never defined the binding symbol in it, so the
//! binding resolved to the OUTER symbol's concrete type and the checker
//! reasoned over the wrong type inside the handler — false errors on
//! runtime-valid programs (analyzer body scopes are discarded before this
//! pass; compare the `OpenFileStatement` symbol recreation).

use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::typechecker::TypeChecker;

fn typecheck(code: &str) -> Result<(), String> {
    let tokens = lex_wfl_with_positions(code);
    let program = Parser::new(&tokens).parse().expect("parse");
    TypeChecker::new()
        .check_types(&program)
        .map_err(|errors| format!("{errors:?}"))
}

#[test]
fn websocket_binding_shadows_outer_symbol_inside_handler_body() {
    // `conn` is a Number outside the handler; inside, it is the event object,
    // and indexing it is runtime-valid. The checker must not flag the body
    // against the outer Number type.
    let code = "store conn as 5\n\
                listen for websockets on port 8080 as srv\n\
                on websocket connect to srv as conn:\n\
                \x20\x20\x20\x20store cid as conn[\"id\"]\n\
                end on\n\
                store next_conn as conn plus 1\n";
    let result = typecheck(code);
    assert!(
        result.is_ok(),
        "the handler binding must shadow the outer `conn` (Number) inside the \
         handler body, and the outer type must be restored after it: {result:?}"
    );
}
