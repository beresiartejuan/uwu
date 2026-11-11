use crate::diagnostics::UwuError;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub col: usize,
    /// Índice del archivo fuente (ver `modules::SourceFile`).
    pub file: usize,
}

impl Span {
    pub fn join(a: Span, b: Span) -> Span {
        Span { start: a.start, end: b.end, line: a.line, col: a.col, file: a.file }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Num,
    Str,
    Ident,
    Kw,
    Op,
    Eof,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub value: String,
    pub span: Span,
}

pub const KEYWORDS: &[&str] = &[
    "let", "mut", "fn", "return", "if", "else", "while", "loop", "for", "in", "match", "type", "true", "false",
    "break", "continue", "import", "export",
];

// Ordenados de más largo a más corto para que `==` gane sobre `=`.
const OPERATORS: &[&str] = &[
    "::", "..", "=>", "==", "!=", "<=", ">=", "&&", "||", "+=", "-=", "*=", "/=", "+", "-", "*", "/", "%", "=", "<",
    ">", "!", "(", ")", "{", "}", "[", "]", ",", ";", ":", ".", "|",
];

fn escape(c: char) -> Option<char> {
    match c {
        'n' => Some('\n'),
        't' => Some('\t'),
        'r' => Some('\r'),
        '"' => Some('"'),
        '\\' => Some('\\'),
        '0' => Some('\0'),
        _ => None,
    }
}

struct Lexer {
    src: Vec<char>,
    i: usize,
    line: usize,
    col: usize,
    file: usize,
}

impl Lexer {
    fn advance(&mut self, n: usize) {
        for _ in 0..n {
            if self.src.get(self.i) == Some(&'\n') {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
            self.i += 1;
        }
    }

    fn starts_with(&self, s: &str, at: usize) -> bool {
        s.chars().enumerate().all(|(k, c)| self.src.get(at + k) == Some(&c))
    }

    fn slice(&self, from: usize, to: usize) -> String {
        self.src[from..to].iter().collect()
    }

    fn span(&self, start: usize, line: usize, col: usize) -> Span {
        Span { start, end: self.i, line, col, file: self.file }
    }
}

pub fn tokenize(src: &str, file: usize) -> Result<Vec<Token>, UwuError> {
    let mut lx = Lexer { src: src.chars().collect(), i: 0, line: 1, col: 1, file };
    let mut tokens: Vec<Token> = Vec::new();

    while lx.i < lx.src.len() {
        let ch = lx.src[lx.i];
        if ch == ' ' || ch == '\t' || ch == '\r' || ch == '\n' {
            lx.advance(1);
            continue;
        }
        if lx.starts_with("//", lx.i) {
            while lx.i < lx.src.len() && lx.src[lx.i] != '\n' {
                lx.advance(1);
            }
            continue;
        }
        let start = lx.i;
        let s_line = lx.line;
        let s_col = lx.col;

        if lx.starts_with("/*", lx.i) {
            lx.advance(2);
            while lx.i < lx.src.len() && !lx.starts_with("*/", lx.i) {
                lx.advance(1);
            }
            if lx.i >= lx.src.len() {
                return Err(UwuError::new(
                    "comentario de bloque sin cerrar",
                    Span { start, end: start + 2, line: s_line, col: s_col, file },
                    None,
                ));
            }
            lx.advance(2);
            continue;
        }

        if ch.is_ascii_digit() {
            // 0x1F, 0o755, 0b1010
            let radix = match (ch, lx.src.get(lx.i + 1)) {
                ('0', Some('x')) => 16,
                ('0', Some('o')) => 8,
                ('0', Some('b')) => 2,
                _ => 10,
            };
            if radix != 10 {
                lx.advance(2);
                let digits_start = lx.i;
                while lx.i < lx.src.len() && (lx.src[lx.i].is_ascii_alphanumeric() || lx.src[lx.i] == '_') {
                    lx.advance(1);
                }
                let digits = lx.slice(digits_start, lx.i).replace('_', "");
                let Ok(n) = i64::from_str_radix(&digits, radix) else {
                    return Err(UwuError::new(
                        format!("número inválido `{}`", lx.slice(start, lx.i)),
                        lx.span(start, s_line, s_col),
                        None,
                    ));
                };
                tokens.push(Token { kind: TokenKind::Num, value: n.to_string(), span: lx.span(start, s_line, s_col) });
                continue;
            }
            while lx.i < lx.src.len() && (lx.src[lx.i].is_ascii_digit() || lx.src[lx.i] == '_') {
                lx.advance(1);
            }
            // `1..5` es un rango, no un decimal
            if lx.src.get(lx.i) == Some(&'.') && lx.src.get(lx.i + 1).is_some_and(|c| c.is_ascii_digit()) {
                lx.advance(1);
                while lx.i < lx.src.len() && (lx.src[lx.i].is_ascii_digit() || lx.src[lx.i] == '_') {
                    lx.advance(1);
                }
            }
            let value: String = lx.slice(start, lx.i).replace('_', "");
            tokens.push(Token { kind: TokenKind::Num, value, span: lx.span(start, s_line, s_col) });
            continue;
        }

        if ch.is_alphabetic() || ch == '_' {
            while lx.i < lx.src.len() && (lx.src[lx.i].is_alphanumeric() || lx.src[lx.i] == '_') {
                lx.advance(1);
            }
            let word = lx.slice(start, lx.i);
            let kind = if KEYWORDS.contains(&word.as_str()) { TokenKind::Kw } else { TokenKind::Ident };
            tokens.push(Token { kind, value: word, span: lx.span(start, s_line, s_col) });
            continue;
        }

        if ch == '"' {
            lx.advance(1);
            let mut value = String::new();
            loop {
                if lx.i >= lx.src.len() || lx.src[lx.i] == '\n' {
                    return Err(UwuError::new(
                        "string sin cerrar",
                        lx.span(start, s_line, s_col),
                        Some("agregá `\"` al final del texto".into()),
                    ));
                }
                let c = lx.src[lx.i];
                if c == '"' {
                    lx.advance(1);
                    break;
                }
                if c == '\\' {
                    let n = lx.src.get(lx.i + 1).copied();
                    match n.and_then(escape) {
                        Some(e) => {
                            value.push(e);
                            lx.advance(2);
                        }
                        None => {
                            let shown = n.map(String::from).unwrap_or_default();
                            return Err(UwuError::new(
                                format!("secuencia de escape desconocida `\\{}`", shown),
                                Span { start: lx.i, end: lx.i + 2, line: lx.line, col: lx.col, file },
                                Some("las secuencias válidas son \\n \\t \\r \\\" \\\\ \\0".into()),
                            ));
                        }
                    }
                    continue;
                }
                value.push(c);
                lx.advance(1);
            }
            tokens.push(Token { kind: TokenKind::Str, value, span: lx.span(start, s_line, s_col) });
            continue;
        }

        if let Some(op) = OPERATORS.iter().find(|o| lx.starts_with(o, lx.i)) {
            lx.advance(op.chars().count());
            tokens.push(Token { kind: TokenKind::Op, value: op.to_string(), span: lx.span(start, s_line, s_col) });
            continue;
        }

        return Err(UwuError::new(
            format!("carácter inesperado `{}`", ch),
            Span { start, end: start + 1, line: lx.line, col: lx.col, file },
            None,
        ));
    }

    tokens.push(Token {
        kind: TokenKind::Eof,
        value: String::new(),
        span: Span { start: lx.i, end: lx.i, line: lx.line, col: lx.col, file },
    });
    Ok(tokens)
}
