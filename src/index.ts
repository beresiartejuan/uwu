import { Checker } from "./checker";
import { Diagnostic, RuntimeError, UwuError } from "./diagnostics";
import { Interpreter } from "./interpreter";
import { tokenize } from "./lexer";
import { parse } from "./parser";

export interface Result {
  /** Errores y warnings de compilación. */
  diagnostics: Diagnostic[];
  /** Pánico durante la ejecución, si lo hubo. */
  runtimeError?: Diagnostic;
  ran: boolean;
}

export function compile(source: string) {
  try {
    const program = parse(tokenize(source));
    const checker = new Checker();
    const diagnostics = checker.check(program);
    return { program, checker, diagnostics };
  } catch (e) {
    if (e instanceof UwuError) return { program: null, checker: null, diagnostics: e.diagnostics };
    throw e;
  }
}

export function run(source: string, out: (line: string) => void = console.log, onlyCheck = false): Result {
  const { program, checker, diagnostics } = compile(source);
  const hasErrors = diagnostics.some((d) => d.severity !== "warning");
  if (!program || !checker || hasErrors || onlyCheck) return { diagnostics, ran: false };
  const interp = new Interpreter({ out, tc: checker.tc, patternTypes: checker.patternTypes });
  try {
    interp.run(program);
    return { diagnostics, ran: true };
  } catch (e) {
    if (e instanceof RuntimeError) {
      return { diagnostics, ran: true, runtimeError: { message: e.message, span: e.span } };
    }
    throw e;
  }
}

export { formatDiagnostic } from "./diagnostics";
