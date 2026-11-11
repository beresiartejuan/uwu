import type { Span } from "./lexer";

export interface Diagnostic {
  message: string;
  span: Span;
  hint?: string;
  severity?: "error" | "warning";
}

/** Error de compilación (lexer/parser): aborta inmediatamente. */
export class UwuError extends Error {
  constructor(public diagnostics: Diagnostic[]) {
    super(diagnostics[0]?.message ?? "error");
  }
}

/** Error en tiempo de ejecución (pánico). */
export class RuntimeError extends Error {
  constructor(message: string, public span: Span) {
    super(message);
  }
}

const useColor = !!process.stdout.isTTY && !process.env.NO_COLOR;
const paint = (code: string, s: string) => (useColor ? `\x1b[${code}m${s}\x1b[0m` : s);

/** Formatea un diagnóstico al estilo de rustc. */
export function formatDiagnostic(d: Diagnostic, source: string, file: string): string {
  const kind = d.severity === "warning" ? "warning" : "error";
  const color = kind === "warning" ? "1;33" : "1;31";
  const lines = source.split(/\r?\n/);
  const lineText = lines[d.span.line - 1] ?? "";
  const num = String(d.span.line);
  const pad = " ".repeat(num.length);
  const bar = paint("1;34", "|");
  const len = Math.max(1, Math.min(d.span.end - d.span.start, lineText.length - (d.span.col - 1)));
  const out = [
    paint(color, kind) + paint("1", `: ${d.message}`),
    `${pad}${paint("1;34", "-->")} ${file}:${d.span.line}:${d.span.col}`,
    `${pad} ${bar}`,
    `${paint("1;34", num)} ${bar} ${lineText}`,
    `${pad} ${bar} ${" ".repeat(Math.max(0, d.span.col - 1))}${paint(color, "^".repeat(len))}`,
  ];
  if (d.hint) out.push(`${pad} ${paint("1;34", "=")} ${paint("1", "ayuda")}: ${d.hint}`);
  return out.join("\n");
}
