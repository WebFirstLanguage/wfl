//! Subset + equivalence guard (manifest grammar §10.1).
//!
//! The frozen data-literal grammar is defined as a *strict subset* of WFL:
//! `L(manifest) ⊂ L(WFL)`. Every byte string the manifest grammar accepts must
//! also be accepted by the **full** WFL lexer + parser, and yield an equivalent
//! tree (here: a program made only of `create map … end map` records).
//!
//! This test lives in the root `wfl` crate because it needs both the full WFL
//! parser (`wfl`) and the manifest parser (`wflpkg`), and `wflpkg` cannot depend
//! on `wfl` (that would be a dependency cycle). It is the differential harness
//! ADR-001 §5.3 asks for, in unit-test form.

use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::parser::ast::Statement;

/// A spread of manifests the frozen grammar accepts, covering every value form.
const ACCEPTED: &[&str] = &[
    // Minimal envelope-only document.
    "create map wflpkg:\n    grammar is \"1.0.0\"\nend map\n",
    // All scalar value forms.
    "create map t:\n\
     s is \"hello world\"\n\
     n is 42\n\
     z is 0\n\
     b is yes\n\
     c is no\n\
     nums is [1, 2, 3]\n\
     names is [\"a\", \"b\"]\n\
     mixed is [\"x\", 1, yes]\n\
     blank is []\n\
     end map\n",
    // A realistic manifest with repeated `dependency` records and a quoted key.
    "create map wflpkg:\n    grammar is \"1.0.0\"\nend map\n\n\
     create map package:\n\
     name is \"greeting\"\n\
     version is \"26.2.1\"\n\
     description is \"A friendly greeter\"\n\
     authors is [\"Alice Smith\", \"Bob Jones\"]\n\
     \"requires\" is \"quoted-keyword-key\"\n\
     end map\n\n\
     create map dependency:\n\
     name is \"http-client\"\n\
     version is \"26.1 or newer\"\n\
     end map\n\n\
     create map dependency:\n\
     name is \"json-parser\"\n\
     version is \"between 25.12 and 26.2\"\n\
     scope is \"dev\"\n\
     end map\n",
    // Escapes inside strings.
    "create map t:\n    s is \"line\\nbreak\\ttab \\\"quote\\\" back\\\\slash\"\nend map\n",
];

fn full_wfl_accepts_as_records(src: &str) -> Result<usize, String> {
    let tokens = lex_wfl_with_positions(src);
    let mut parser = Parser::new(&tokens);
    let program = parser
        .parse()
        .map_err(|errs| format!("full WFL parser rejected it: {errs:?}"))?;
    if program.statements.is_empty() {
        return Err("full WFL parser produced no statements".to_string());
    }
    for stmt in &program.statements {
        if !matches!(stmt, Statement::MapCreation { .. }) {
            return Err(format!(
                "expected only `create map` records, found {stmt:?}"
            ));
        }
    }
    Ok(program.statements.len())
}

#[test]
fn manifest_language_is_a_subset_of_wfl() {
    for src in ACCEPTED {
        // 1. The manifest grammar accepts it.
        assert!(
            wflpkg::datalit::parse(src.as_bytes()).is_ok(),
            "the manifest grammar should accept:\n{src}"
        );
        // 2. The full WFL parser also accepts it, as records only (subset guard).
        match full_wfl_accepts_as_records(src) {
            Ok(n) => assert!(n >= 1),
            Err(why) => panic!(
                "subset violation: an accepted manifest is NOT valid WFL.\n{why}\n---\n{src}"
            ),
        }
    }
}

#[test]
fn record_count_matches_between_parsers() {
    // Structural equivalence: the two parsers agree on how many records exist.
    for src in ACCEPTED {
        let doc = wflpkg::datalit::parse(src.as_bytes()).expect("manifest accepts");
        let n_full = full_wfl_accepts_as_records(src).expect("full WFL accepts");
        assert_eq!(
            doc.records.len(),
            n_full,
            "manifest and full WFL parser disagree on record count for:\n{src}"
        );
    }
}
