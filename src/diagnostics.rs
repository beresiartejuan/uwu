use crate::lexer::Span;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub message: String,
    pub span: Span,
    pub hint: Option<String>,
    pub severity: Severity,
}

/// Error de compilación (lexer/parser): aborta inmediatamente.
#[derive(Debug)]
pub struct UwuError {
    pub diagnostics: Vec<Diagnostic>,
}

impl UwuError {
    pub fn new(message: impl Into<String>, span: Span, hint: Option<String>) -> Self {
        Self { diagnostics: vec![Diagnostic { message: message.into(), span, hint, severity: Severity::Error }] }
    }
}

/// Error en tiempo de ejecución (pánico).
#[derive(Clone, Debug)]
pub struct RuntimeError {
    pub message: String,
    pub span: Span,
}

impl RuntimeError {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self { message: message.into(), span }
    }
}

/// Formatea un diagnóstico al estilo de rustc.
pub fn format_diagnostic(d: &Diagnostic, source: &str, file: &str, color: bool) -> String {
    let paint = |code: &str, s: &str| {
        if color {
            format!("\x1b[{}m{}\x1b[0m", code, s)
        } else {
            s.to_string()
        }
    };
    let kind = if d.severity == Severity::Warning { "warning" } else { "error" };
    let code = if d.severity == Severity::Warning { "1;33" } else { "1;31" };
    let lines: Vec<&str> = source.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l)).collect();
    let line_text = lines.get(d.span.line.saturating_sub(1)).copied().unwrap_or("");
    let line_chars = line_text.chars().count();
    let num = d.span.line.to_string();
    let pad = " ".repeat(num.len());
    let bar = paint("1;34", "|");
    let width = d.span.end.saturating_sub(d.span.start);
    let avail = line_chars.saturating_sub(d.span.col.saturating_sub(1));
    let len = width.min(avail).max(1);
    let mut out = vec![
        format!("{}{}", paint(code, kind), paint("1", &format!(": {}", d.message))),
        format!("{}{} {}:{}:{}", pad, paint("1;34", "-->"), file, d.span.line, d.span.col),
        format!("{} {}", pad, bar),
        format!("{} {} {}", paint("1;34", &num), bar, line_text),
        format!("{} {} {}{}", pad, bar, " ".repeat(d.span.col.saturating_sub(1)), paint(code, &"^".repeat(len))),
    ];
    if let Some(hint) = &d.hint {
        out.push(format!("{} {} {}: {}", pad, paint("1;34", "="), paint("1", "ayuda"), hint));
    }
    out.join("\n")
}
