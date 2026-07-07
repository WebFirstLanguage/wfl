use crate::diagnostics::render::{RenderOptions, render_diagnostic};
use crate::diagnostics::{DiagnosticKind, DiagnosticReporter, Span, Suggestion, WflDiagnostic};
use codespan_reporting::term::termcolor::Buffer;

/// Render a diagnostic against `source` into plain (color-stripped) text.
fn render_plain(source: &str, diag: &WflDiagnostic) -> String {
    let mut reporter = DiagnosticReporter::new();
    let file_id = reporter.add_file("test.wfl", source);
    let mut buffer = Buffer::no_color();
    render_diagnostic(
        &mut buffer,
        &mut reporter,
        file_id,
        diag,
        &RenderOptions::default(),
    )
    .expect("render should succeed");
    String::from_utf8(buffer.into_inner()).expect("utf8")
}

#[test]
fn flagship_type_error_matches_mockup() {
    // Line 5 is the offending expression, so the source frame reproduces the
    // mockup verbatim.
    let source = "line one\nline two\nline three\nline four\nage plus \"hello\"\n";
    let diag = WflDiagnostic::error("Cannot add number and text")
        .with_location(5, 8)
        .with_kind(DiagnosticKind::TypeError)
        .with_type_mismatch("Number", "Text (\"hello\")")
        .with_explanation("cannot add a Number and Text.")
        .with_suggestion(
            Suggestion::new("Try converting first:")
                .with_example("age plus 5")
                .with_example("string of age with \"hello\""),
        )
        .with_primary_label(Span { start: 0, end: 0 }, "The expression");

    let expected = "\
✕ Type Error   line 5, column 8

    Expected: Number
    Found:    Text (\"hello\")

The expression
    age plus \"hello\"
cannot add a Number and Text.

💡 Try converting first:
    age plus 5
    — or —
    string of age with \"hello\"
";
    assert_eq!(render_plain(source, &diag), expected);
}

#[test]
fn parse_error_shows_source_frame_and_suppresses_generic_caption() {
    let source = "store x as\n";
    let diag = WflDiagnostic::error("Expected expression after 'as'")
        .with_location(1, 11)
        .with_kind(DiagnosticKind::ParseError)
        // "here" is a generic placeholder caption and should be suppressed.
        .with_primary_label(Span { start: 0, end: 0 }, "here");

    let expected = "\
✕ Parse Error   line 1, column 11

    store x as
Expected expression after 'as'
";
    assert_eq!(render_plain(source, &diag), expected);
}

#[test]
fn lint_warning_renders_as_warning_with_suggestion() {
    let source = "store myVar as 5\n";
    let diag = WflDiagnostic::warning("Variable name 'myVar' should be snake_case")
        .with_location(1, 7)
        .with_kind(DiagnosticKind::LintWarning)
        .with_suggestion(Suggestion::new("Rename to 'my_var'"));

    let expected = "\
▲ Lint Warning   line 1, column 7

    store myVar as 5
Variable name 'myVar' should be snake_case

💡 Rename to 'my_var'
";
    assert_eq!(render_plain(source, &diag), expected);
}

#[test]
fn diagnostic_without_span_degrades_gracefully() {
    // No location, no labels: title + explanation only, no source frame.
    let diag = WflDiagnostic::warning("Line exceeds the maximum length")
        .with_kind(DiagnosticKind::LintWarning);

    let expected = "\
▲ Lint Warning

Line exceeds the maximum length
";
    assert_eq!(render_plain("", &diag), expected);
}

#[test]
fn ansi_sink_emits_color_escapes() {
    let source = "age plus \"hello\"\n";
    let diag = WflDiagnostic::error("Cannot add number and text")
        .with_location(1, 1)
        .with_kind(DiagnosticKind::TypeError);

    let mut reporter = DiagnosticReporter::new();
    let file_id = reporter.add_file("test.wfl", source);
    let mut buffer = Buffer::ansi();
    render_diagnostic(
        &mut buffer,
        &mut reporter,
        file_id,
        &diag,
        &RenderOptions::default(),
    )
    .unwrap();
    let out = String::from_utf8(buffer.into_inner()).unwrap();
    assert!(out.contains('\u{1b}'), "ansi buffer should contain escapes");
}

#[test]
fn no_color_sink_has_no_escapes() {
    let out = render_plain(
        "x\n",
        &WflDiagnostic::error("boom")
            .with_location(1, 1)
            .with_kind(DiagnosticKind::RuntimeError),
    );
    assert!(
        !out.contains('\u{1b}'),
        "no_color buffer should be plain text"
    );
}

#[test]
fn ascii_fallback_replaces_unicode_glyphs() {
    let mut reporter = DiagnosticReporter::new();
    let file_id = reporter.add_file("test.wfl", "x\n");
    let diag = WflDiagnostic::error("boom")
        .with_location(1, 1)
        .with_kind(DiagnosticKind::RuntimeError)
        .with_suggestion(Suggestion::new("Try again").with_example("y"));
    let mut buffer = Buffer::no_color();
    render_diagnostic(
        &mut buffer,
        &mut reporter,
        file_id,
        &diag,
        &RenderOptions { unicode: false },
    )
    .unwrap();
    let out = String::from_utf8(buffer.into_inner()).unwrap();
    assert!(
        out.starts_with("x Runtime Error"),
        "ascii glyph expected: {out:?}"
    );
    assert!(out.contains("* Try again"), "ascii bulb expected: {out:?}");
    assert!(!out.contains('✕'));
    assert!(!out.contains('💡'));
}
