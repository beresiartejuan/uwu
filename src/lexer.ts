import { UwuError } from "./diagnostics";

export interface Span {
  start: number;
  end: number;
  line: number;
  col: number;
}

export type TokenKind = "num" | "str" | "ident" | "kw" | "op" | "eof";

export interface Token {
  kind: TokenKind;
  value: string;
  span: Span;
}

export const KEYWORDS = new Set([
  "let", "mut", "fn", "return", "if", "else", "while", "loop", "for", "in",
  "match", "type", "true", "false", "break", "continue",
]);

// Ordenados de más largo a más corto para que `==` gane sobre `=`.
const OPERATORS = [
  "::", "..", "=>", "==", "!=", "<=", ">=", "&&", "||", "+=", "-=", "*=", "/=",
  "+", "-", "*", "/", "%", "=", "<", ">", "!", "(", ")", "{", "}", "[", "]",
  ",", ";", ":", ".", "|",
];

const ESCAPES: Record<string, string> = { n: "\n", t: "\t", r: "\r", '"': '"', "\\": "\\", "0": "\0" };

export function tokenize(src: string): Token[] {
  const tokens: Token[] = [];
  let i = 0;
  let line = 1;
  let col = 1;

  const advance = (n: number) => {
    for (let k = 0; k < n; k++) {
      if (src[i] === "\n") {
        line++;
        col = 1;
      } else col++;
      i++;
    }
  };

  while (i < src.length) {
    const ch = src[i];
    if (ch === " " || ch === "\t" || ch === "\r" || ch === "\n") {
      advance(1);
      continue;
    }
    if (src.startsWith("//", i)) {
      while (i < src.length && src[i] !== "\n") advance(1);
      continue;
    }
    const start = i;
    const sLine = line;
    const sCol = col;
    const span = (): Span => ({ start, end: i, line: sLine, col: sCol });

    if (src.startsWith("/*", i)) {
      advance(2);
      while (i < src.length && !src.startsWith("*/", i)) advance(1);
      if (i >= src.length) {
        throw new UwuError([{ message: "comentario de bloque sin cerrar", span: { start, end: start + 2, line: sLine, col: sCol } }]);
      }
      advance(2);
      continue;
    }

    if (/[0-9]/.test(ch)) {
      while (i < src.length && /[0-9_]/.test(src[i])) advance(1);
      // `1..5` es un rango, no un decimal
      if (src[i] === "." && /[0-9]/.test(src[i + 1] ?? "")) {
        advance(1);
        while (i < src.length && /[0-9_]/.test(src[i])) advance(1);
      }
      tokens.push({ kind: "num", value: src.slice(start, i).replace(/_/g, ""), span: span() });
      continue;
    }

    if (/[\p{L}_]/u.test(ch)) {
      while (i < src.length && /[\p{L}\p{N}_]/u.test(src[i])) advance(1);
      const word = src.slice(start, i);
      tokens.push({ kind: KEYWORDS.has(word) ? "kw" : "ident", value: word, span: span() });
      continue;
    }

    if (ch === '"') {
      advance(1);
      let value = "";
      for (;;) {
        if (i >= src.length || src[i] === "\n") {
          throw new UwuError([{ message: "string sin cerrar", span: span(), hint: 'agregá `"` al final del texto' }]);
        }
        const c = src[i];
        if (c === '"') {
          advance(1);
          break;
        }
        if (c === "\\") {
          const n = src[i + 1];
          if (n === undefined || !(n in ESCAPES)) {
            throw new UwuError([{
              message: `secuencia de escape desconocida \`\\${n ?? ""}\``,
              span: { start: i, end: i + 2, line, col },
              hint: "las secuencias válidas son \\n \\t \\r \\\" \\\\ \\0",
            }]);
          }
          value += ESCAPES[n];
          advance(2);
          continue;
        }
        value += c;
        advance(1);
      }
      tokens.push({ kind: "str", value, span: span() });
      continue;
    }

    const op = OPERATORS.find((o) => src.startsWith(o, i));
    if (op) {
      advance(op.length);
      tokens.push({ kind: "op", value: op, span: span() });
      continue;
    }

    throw new UwuError([{ message: `carácter inesperado \`${ch}\``, span: { start, end: start + 1, line, col } }]);
  }

  tokens.push({ kind: "eof", value: "", span: { start: i, end: i, line, col } });
  return tokens;
}
