#!/usr/bin/env node
import { existsSync, readFileSync } from "fs";
import { formatDiagnostic, run } from "./index";

const USAGE = `uwu — un lenguaje con la sintaxis de Rust y los tipos de TypeScript

uso:
  uwu <archivo.uwu>          chequea los tipos y ejecuta
  uwu check <archivo.uwu>    sólo chequea los tipos
`;

function main(argv: string[]): number {
  const onlyCheck = argv[0] === "check";
  const file = onlyCheck ? argv[1] : argv[0] === "run" ? argv[1] : argv[0];
  if (!file || file === "-h" || file === "--help") {
    console.log(USAGE);
    return file ? 0 : 1;
  }
  if (!existsSync(file)) {
    console.error(`no existe el archivo "${file}"`);
    return 1;
  }
  const source = readFileSync(file, "utf8");
  const result = run(source, (line) => console.log(line), onlyCheck);
  for (const d of result.diagnostics) console.error(formatDiagnostic(d, source, file) + "\n");
  const errors = result.diagnostics.filter((d) => d.severity !== "warning").length;
  if (errors) {
    console.error(`no se pudo ejecutar \`${file}\`: ${errors} error(es)`);
    return 1;
  }
  if (result.runtimeError) {
    console.error("\n" + formatDiagnostic(result.runtimeError, source, file));
    return 101;
  }
  if (onlyCheck) console.log(`✓ ${file}: sin errores de tipos`);
  return 0;
}

process.exitCode = main(process.argv.slice(2));
