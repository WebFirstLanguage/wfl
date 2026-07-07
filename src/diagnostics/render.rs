//! The Elm-style terminal renderer for WFL diagnostics.
//!
//! Every WFL error and warning is a [`WflDiagnostic`]; this module turns one into
//! the clear, actionable layout that reads like a helpful colleague:
//!
//! ```text
//! ✕ Type Error   line 5, column 8
//!
//!     Expected: Number
//!     Found:    Text ("hello")
//!
//! The expression
//!     age plus "hello"
//! cannot add a Number and Text.
//!
//! 💡 Try converting first:
//!     age plus 5
//!     — or —
//!     string of age with "hello"
//! ```
//!
//! The renderer writes to any [`WriteColor`] sink, so the CLI (stderr) and the
//! REPL (an in-memory buffer) share a single code path. Color is decided by the
//! sink: a `no_color` buffer or a non-TTY stream simply drops the styling.

use crate::diagnostics::{DiagnosticReporter, Severity, WflDiagnostic};
use codespan_reporting::term::termcolor::{Color, ColorSpec, WriteColor};
use std::io;

/// Four-space indent used for the Expected/Found block, the source frame, and
/// suggestion examples — matching the mockup.
const INDENT: &str = "    ";

/// Options controlling how a diagnostic is rendered. Color is handled by the
/// [`WriteColor`] sink, so this only carries presentation choices the sink can't.
#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// Use Unicode glyphs (✕, 💡, —). When false, ASCII fallbacks are used so the
    /// output stays readable on terminals that mangle Unicode.
    pub unicode: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        RenderOptions { unicode: true }
    }
}

fn severity_color(severity: Severity) -> Color {
    match severity {
        Severity::Error => Color::Red,
        Severity::Warning => Color::Yellow,
        Severity::Note => Color::Blue,
        Severity::Help => Color::Cyan,
    }
}

/// ASCII fallback glyph for a severity, used when `unicode` is disabled.
fn severity_ascii(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "x",
        Severity::Warning => "!",
        Severity::Note => "-",
        Severity::Help => "*",
    }
}

/// Primary-label messages that are too generic to show as a caption above the
/// source frame (they were placeholders from the old converters). Suppressed so
/// the frame reads cleanly until stages set a meaningful caption.
fn is_generic_caption(message: &str) -> bool {
    matches!(
        message.trim().to_ascii_lowercase().as_str(),
        "" | "here"
            | "error occurred here"
            | "type error occurred here"
            | "runtime error occurred here"
            | "semantic error occurred here"
            | "parse error occurred here"
    )
}

fn set(w: &mut dyn WriteColor, spec: &ColorSpec) -> io::Result<()> {
    w.set_color(spec)
}

fn colored(fg: Color, bold: bool) -> ColorSpec {
    let mut spec = ColorSpec::new();
    spec.set_fg(Some(fg)).set_bold(bold);
    spec
}

fn dimmed() -> ColorSpec {
    let mut spec = ColorSpec::new();
    spec.set_dimmed(true);
    spec
}

/// Resolve the (line, column) shown in the title: prefer the diagnostic's own
/// stored position, falling back to the first label's span.
fn resolve_location(
    reporter: &mut DiagnosticReporter,
    file_id: usize,
    diag: &WflDiagnostic,
) -> (usize, usize) {
    if diag.line > 0 {
        return (diag.line, diag.column);
    }
    if let Some((span, _)) = diag.labels.first()
        && let Some((line, column)) = reporter.offset_to_line_col(file_id, span.start)
    {
        return (line, column);
    }
    (diag.line, diag.column)
}

/// Render a single diagnostic to the given sink in the Elm-style layout.
pub fn render_diagnostic(
    w: &mut dyn WriteColor,
    reporter: &mut DiagnosticReporter,
    file_id: usize,
    diag: &WflDiagnostic,
    opts: &RenderOptions,
) -> io::Result<()> {
    let (line, column) = resolve_location(reporter, file_id, diag);

    // --- Title: "✕ Type Error   line 5, column 8" ---
    let glyph = if opts.unicode {
        diag.severity.glyph()
    } else {
        severity_ascii(diag.severity)
    };
    set(w, &colored(severity_color(diag.severity), true))?;
    write!(w, "{glyph} {}", diag.title())?;
    w.reset()?;
    if line > 0 {
        set(w, &dimmed())?;
        write!(w, "   line {line}, column {column}")?;
        w.reset()?;
    }
    writeln!(w)?;

    // --- Expected / Found block ---
    if let Some(tm) = &diag.type_info {
        writeln!(w)?;
        let label_width = "Expected:".len();
        // Expected: (green)
        write!(w, "{INDENT}")?;
        set(w, &colored(Color::Green, false))?;
        write!(w, "Expected:")?;
        w.reset()?;
        set(w, &colored(Color::Green, false))?;
        writeln!(w, " {}", tm.expected)?;
        w.reset()?;
        // Found:    (red), padded to align values
        write!(w, "{INDENT}")?;
        set(w, &colored(Color::Red, false))?;
        write!(w, "Found:")?;
        w.reset()?;
        let pad = label_width.saturating_sub("Found:".len());
        set(w, &colored(Color::Red, false))?;
        writeln!(w, "{} {}", " ".repeat(pad), tm.found)?;
        w.reset()?;
    }

    // --- Body: caption + source frame + explanation ---
    let caption = diag
        .labels
        .iter()
        .map(|(_, m)| m.as_str())
        .find(|m| !is_generic_caption(m));
    let source_line = if line > 0 {
        reporter.line_text(file_id, line)
    } else {
        None
    };
    let explanation: Option<&str> = diag.explanation.as_deref().or(if diag.message.is_empty() {
        None
    } else {
        Some(&diag.message)
    });

    if caption.is_some() || source_line.is_some() || explanation.is_some() {
        writeln!(w)?;
        if let Some(cap) = caption {
            writeln!(w, "{cap}")?;
        }
        if let Some(src) = &source_line {
            writeln!(w, "{INDENT}{src}")?;
        }
        if let Some(expl) = explanation {
            for expl_line in expl.split('\n') {
                writeln!(w, "{expl_line}")?;
            }
        }
    }

    // --- Suggestion: "💡 Try ...:" + example fixes ---
    if let Some(sug) = &diag.suggestion {
        writeln!(w)?;
        let bulb = if opts.unicode { "💡" } else { "*" };
        set(w, &colored(Color::Cyan, true))?;
        write!(w, "{bulb} {}", sug.message)?;
        w.reset()?;
        writeln!(w)?;
        for (i, example) in sug.examples.iter().enumerate() {
            if i > 0 {
                set(w, &dimmed())?;
                writeln!(w, "{INDENT}{}", sug.joiner_str())?;
                w.reset()?;
            }
            writeln!(w, "{INDENT}{example}")?;
        }
    }

    // --- Leftover notes (backward-compat bucket) ---
    for note in &diag.notes {
        set(w, &dimmed())?;
        writeln!(w, "note: {note}")?;
        w.reset()?;
    }

    Ok(())
}
