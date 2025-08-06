use super::*;
use crate::parser::ast::ParseError;
use crate::typechecker::TypeError;

#[test]
fn test_parse_error_conversion() {
    let mut reporter = DiagnosticReporter::new();
    let source = "store x as 42\nstore y as \"hello\"\nstore z as";
    let file_id = reporter.add_file("test.wfl", source);

    let error = ParseError::new("Expected expression after 'as'".to_string(), 3, 11);

    let diagnostic = reporter.convert_parse_error(file_id, &error);
    assert_eq!(diagnostic.severity, Severity::Error);
    assert_eq!(diagnostic.message, "Expected expression after 'as'");
    assert_eq!(diagnostic.labels.len(), 1);
}

#[test]
fn test_type_error_conversion() {
    let mut reporter = DiagnosticReporter::new();
    let source = "store x as 42\nstore y as \"hello\"\ndisplay x plus y";
    let file_id = reporter.add_file("test.wfl", source);

    let error = TypeError::new(
        "Cannot add number and text".to_string(),
        Some(crate::parser::ast::Type::Number),
        Some(crate::parser::ast::Type::Text),
        3,
        12,
    );

    let diagnostic = reporter.convert_type_error(file_id, &error);
    assert_eq!(diagnostic.severity, Severity::Error);
    assert!(diagnostic.message.contains("Cannot add number and text"));
    assert_eq!(diagnostic.labels.len(), 1);
    assert!(!diagnostic.notes.is_empty());
}

#[test]
fn test_offset_to_line_col() {
    let mut reporter = DiagnosticReporter::new();
    let source = "line 1\nline 2\nline 3";
    let file_id = reporter.add_file("test.wfl", source);

    assert_eq!(reporter.offset_to_line_col(file_id, 0), Some((1, 1)));
    assert_eq!(reporter.offset_to_line_col(file_id, 1), Some((1, 2)));
    assert_eq!(reporter.offset_to_line_col(file_id, 6), Some((1, 7)));
    assert_eq!(reporter.offset_to_line_col(file_id, 7), Some((2, 1)));
    assert_eq!(reporter.offset_to_line_col(file_id, 14), Some((3, 1)));
    assert_eq!(reporter.offset_to_line_col(file_id, 20), Some((3, 7)));
}

#[test]
fn test_line_col_to_offset() {
    let mut reporter = DiagnosticReporter::new();
    let source = "line 1\nline 2\nline 3";
    let file_id = reporter.add_file("test.wfl", source);

    assert_eq!(reporter.line_col_to_offset(file_id, 1, 1), Some(0));
    assert_eq!(reporter.line_col_to_offset(file_id, 1, 2), Some(1));
    assert_eq!(reporter.line_col_to_offset(file_id, 2, 1), Some(7));
    assert_eq!(reporter.line_col_to_offset(file_id, 3, 1), Some(14));
}

#[test]
fn test_line_col_to_offset_with_many_newlines() {
    let mut reporter = DiagnosticReporter::new();
    // Simplified test: verify that newlines don't cause offset drift
    let source = "line1\n\n\n\nline5 with ++ error";
    let file_id = reporter.add_file("test.wfl", source);

    // Line 5, column 12 should point to the first + (1-indexed)
    let offset = reporter.line_col_to_offset(file_id, 5, 12).unwrap();
    assert_eq!(&source[offset..offset + 2], "++");

    // Test with the exact case from the bug report
    let source2 = "store x as 1\n\n\nstore y as 2++3";
    let file_id2 = reporter.add_file("test2.wfl", source2);

    // Line 4, column 13 should point to the first + (1-indexed)
    let offset = reporter.line_col_to_offset(file_id2, 4, 13).unwrap();
    assert_eq!(&source2[offset..offset + 2], "++");

    // Verify the fix: before the fix, adding newlines would shift the offset
    // This test ensures that the offset calculation is correct regardless of newlines
    let source3 = "abc\n\n\n\n\n\n\n\n\n\nxyz++123";
    let file_id3 = reporter.add_file("test3.wfl", source3);
    let offset = reporter.line_col_to_offset(file_id3, 11, 4).unwrap();
    assert_eq!(&source3[offset..offset + 2], "++");
}

#[test]
fn test_line_col_to_offset_edge_cases() {
    let mut reporter = DiagnosticReporter::new();

    // Test empty lines
    let source = "\n\n\nabc";
    let file_id = reporter.add_file("test.wfl", source);
    assert_eq!(reporter.line_col_to_offset(file_id, 4, 1), Some(3));
    assert_eq!(reporter.line_col_to_offset(file_id, 4, 2), Some(4));

    // Test last line without newline
    let source2 = "line1\nline2";
    let file_id2 = reporter.add_file("test2.wfl", source2);
    assert_eq!(reporter.line_col_to_offset(file_id2, 2, 1), Some(6));
    assert_eq!(reporter.line_col_to_offset(file_id2, 2, 5), Some(10));

    // Test out of bounds
    assert_eq!(reporter.line_col_to_offset(file_id2, 3, 1), None);
    assert_eq!(reporter.line_col_to_offset(file_id2, 2, 10), None);
}
