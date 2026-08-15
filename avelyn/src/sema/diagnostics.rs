// sema/diagnostics.rs — Compiler diagnostic infrastructure
//
// Diagnostics flow through every compiler stage: Sema, AIRGen, Verifier,
// and IRGen all emit `Diagnostic` values.  The driver collects them and
// pretty-prints them to stderr before exiting.

use crate::ast::Span;

// ─── Severity ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// A lint-level note — emitted only with `--verbose`.
    Note,
    /// A non-fatal issue; compilation continues.
    Warning,
    /// A fatal issue; compilation aborts after the current phase.
    Error,
    /// An internal compiler bug — always fatal.
    Ice,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Note    => write!(f, "note"),
            Severity::Warning => write!(f, "warning"),
            Severity::Error   => write!(f, "error"),
            Severity::Ice     => write!(f, "internal compiler error"),
        }
    }
}

// ─── Diagnostic ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub span:     Span,
    pub message:  String,
    /// Optional secondary label (appears on a separate line).
    pub hint:     Option<String>,
    /// Optional note with a fix-it suggestion.
    pub fix_hint: Option<String>,
}

impl Diagnostic {
    pub fn error(span: Span, message: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Error,
            span,
            message: message.into(),
            hint: None,
            fix_hint: None,
        }
    }

    pub fn warning(span: Span, message: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Warning,
            span,
            message: message.into(),
            hint: None,
            fix_hint: None,
        }
    }

    pub fn note(span: Span, message: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Note,
            span,
            message: message.into(),
            hint: None,
            fix_hint: None,
        }
    }

    pub fn ice(message: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Ice,
            span: Span::UNKNOWN,
            message: message.into(),
            hint: None,
            fix_hint: None,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn with_fix(mut self, fix: impl Into<String>) -> Self {
        self.fix_hint = Some(fix.into());
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity >= Severity::Error
    }
}

// ─── DiagnosticEmitter ───────────────────────────────────────────────────────

/// Accumulates diagnostics during a compiler phase.
/// Call `emit_to_stderr` at the end of a phase or driver stage.
#[derive(Debug, Default)]
pub struct DiagnosticEmitter {
    pub diagnostics: Vec<Diagnostic>,
    /// Path table: index = file_id, value = file path string.
    pub files: Vec<String>,
}

impl DiagnosticEmitter {
    pub fn new() -> Self {
        DiagnosticEmitter {
            diagnostics: Vec::new(),
            files: vec!["<unknown>".to_string()],
        }
    }

    /// Register a source file and return its file_id for use in Spans.
    pub fn register_file(&mut self, path: &str) -> u32 {
        let id = self.files.len() as u32;
        self.files.push(path.to_string());
        id
    }

    pub fn file_name(&self, file_id: u32) -> &str {
        self.files.get(file_id as usize).map(|s| s.as_str()).unwrap_or("<unknown>")
    }

    pub fn emit(&mut self, d: Diagnostic) {
        self.diagnostics.push(d);
    }

    pub fn error(&mut self, span: Span, message: impl Into<String>) {
        self.emit(Diagnostic::error(span, message));
    }

    pub fn warning(&mut self, span: Span, message: impl Into<String>) {
        self.emit(Diagnostic::warning(span, message));
    }

    pub fn note(&mut self, span: Span, message: impl Into<String>) {
        self.emit(Diagnostic::note(span, message));
    }

    pub fn ice(&mut self, message: impl Into<String>) {
        self.emit(Diagnostic::ice(message));
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.is_error())
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.severity == Severity::Error).count()
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.severity == Severity::Warning).count()
    }

    /// Print all diagnostics to stderr.
    /// `source_lines` is optional — when provided, the offending source line
    /// is printed with a caret pointer for better UX.
    pub fn emit_to_stderr(&self, source_lines: Option<&[&str]>) {
        for d in &self.diagnostics {
            let sev = d.severity.to_string().to_uppercase();
            let loc = if d.span.is_known() {
                let file = self.file_name(d.span.file_id);
                if d.span.col > 0 {
                    format!("{}:{}:{}: ", file, d.span.line, d.span.col)
                } else {
                    format!("{}:{}: ", file, d.span.line)
                }
            } else {
                String::new()
            };
            eprintln!("{}{}: {}", loc, sev, d.message);

            // Source line + caret
            if let (Some(lines), true) = (source_lines, d.span.is_known()) {
                let line_idx = d.span.line.saturating_sub(1) as usize;
                if let Some(src_line) = lines.get(line_idx) {
                    eprintln!("  | {}", src_line);
                    if d.span.col > 0 {
                        let spaces = " ".repeat(4 + d.span.col.saturating_sub(1) as usize);
                        eprintln!("{}^", spaces);
                    }
                }
            }

            if let Some(hint) = &d.hint {
                eprintln!("  hint: {}", hint);
            }
            if let Some(fix) = &d.fix_hint {
                eprintln!("  fix:  {}", fix);
            }
        }

        let ec = self.error_count();
        let wc = self.warning_count();
        if ec > 0 || wc > 0 {
            eprintln!("  [{} error(s), {} warning(s)]", ec, wc);
        }
    }

    /// Drain diagnostics into a Vec and return them.
    pub fn take(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.diagnostics)
    }
}
