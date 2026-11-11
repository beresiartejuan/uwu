use std::io::IsTerminal;
use std::path::Path;
use uwu::diagnostics::{Diagnostic, Severity};
use uwu::modules::SourceFile;
use uwu::{format_diagnostic, run_file};

const USAGE: &str = "uwu — un lenguaje con la sintaxis de Rust y los tipos de TypeScript

uso:
  uwu <archivo.uwu> [args...]    chequea los tipos y ejecuta
  uwu check <archivo.uwu>        sólo chequea los tipos

variables de entorno:
  UWU_PATH    carpetas extra (separadas por `:`) donde buscar `import modulo;`
  NO_COLOR    desactiva los colores en los errores
";

fn show(d: &Diagnostic, files: &[SourceFile], color: bool) -> String {
    match files.get(d.span.file) {
        Some(f) => format_diagnostic(d, &f.source, &f.name, color),
        None => format!("error: {}", d.message),
    }
}

fn real_main(argv: &[String]) -> i32 {
    let first = argv.first().map(String::as_str);
    let only_check = first == Some("check");
    let file_idx = if only_check || first == Some("run") { 1 } else { 0 };
    let Some(file) = argv.get(file_idx) else {
        println!("{}", USAGE);
        return 1;
    };
    if file == "-h" || file == "--help" {
        println!("{}", USAGE);
        return 0;
    }
    let script_args: Vec<String> = argv[file_idx + 1..].to_vec();
    let mut out = |line: &str| println!("{}", line);
    let result = match run_file(Path::new(file), &mut out, only_check, &script_args) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("no se pudo abrir \"{}\": {}", file, e);
            return 1;
        }
    };
    let color = std::io::stderr().is_terminal() && std::env::var_os("NO_COLOR").is_none();
    for d in &result.diagnostics {
        eprintln!("{}\n", show(d, &result.files, color));
    }
    let errors = result.diagnostics.iter().filter(|d| d.severity != Severity::Warning).count();
    if errors > 0 {
        eprintln!("no se pudo ejecutar `{}`: {} error(es)", file, errors);
        return 1;
    }
    if let Some(e) = &result.runtime_error {
        eprintln!("\n{}", show(e, &result.files, color));
        return 101;
    }
    if only_check {
        println!("✓ {}: sin errores de tipos", file);
    }
    0
}

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(real_main(&argv));
}
