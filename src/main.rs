use std::io::{BufRead, IsTerminal, Write};
use std::path::Path;
use uwu::diagnostics::{Diagnostic, Severity};
use uwu::modules::SourceFile;
use uwu::repl::Repl;
use uwu::{format_diagnostic, run_file, STACK_SIZE};

const USAGE: &str = "uwu — un lenguaje con la sintaxis de Rust y los tipos de TypeScript

uso:
  uwu                            abre el REPL
  uwu <archivo.uwu> [args...]    chequea los tipos y ejecuta
  uwu check <archivo.uwu>        sólo chequea los tipos
  uwu repl                       abre el REPL

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

const REPL_HELP: &str = "comandos:
  :tipo <expresión>   muestra el tipo sin ejecutarla (también `:t`)
  :limpiar            olvida todo lo definido
  :ayuda              muestra esta ayuda (también `:h`)
  :salir              sale del REPL (también `:q` o Ctrl+D)

Lo que escribas se ejecuta enseguida y se muestra el valor de la última
expresión. Si falta cerrar un `{`, `(` o `[`, el REPL sigue leyendo.";

fn use_color() -> bool {
    std::io::stderr().is_terminal() && std::env::var_os("NO_COLOR").is_none()
}

fn repl() -> i32 {
    let interactive = std::io::stdin().is_terminal();
    let color = use_color();
    if interactive {
        println!("uwu {} — `:ayuda` muestra los comandos, Ctrl+D sale", env!("CARGO_PKG_VERSION"));
    }
    let mut out = |line: &str| println!("{}", line);
    let mut repl = Repl::new(&mut out);
    let mut lines = std::io::stdin().lock().lines();
    let mut buf = String::new();
    loop {
        if interactive {
            print!("{}", if buf.is_empty() { "uwu> " } else { " ... " });
            let _ = std::io::stdout().flush();
        }
        let Some(Ok(line)) = lines.next() else {
            if interactive {
                println!();
            }
            break;
        };
        if buf.is_empty() {
            let cmd = line.trim();
            if cmd.is_empty() {
                continue;
            }
            if let Some(cmd) = cmd.strip_prefix(':') {
                let (name, rest) = cmd.split_once(' ').unwrap_or((cmd, ""));
                match name {
                    "salir" | "q" | "quit" | "exit" => break,
                    "ayuda" | "h" | "help" => println!("{}", REPL_HELP),
                    "limpiar" | "reset" => repl.reset(),
                    "tipo" | "t" | "type" => match repl.type_of(rest) {
                        Ok(t) => println!("{}", t),
                        Err(ds) => ds.iter().for_each(|d| {
                            eprintln!(
                                "{}
",
                                show(d, repl.files(), color)
                            )
                        }),
                    },
                    _ => eprintln!("comando desconocido `:{}`; `:ayuda` muestra los comandos", name),
                }
                continue;
            }
        }
        buf.push_str(&line);
        buf.push('\n');
        if Repl::is_incomplete(&buf) {
            continue;
        }
        let r = repl.eval(&std::mem::take(&mut buf));
        for d in &r.diagnostics {
            eprintln!("{}\n", show(d, repl.files(), color));
        }
        if let Some(e) = &r.runtime_error {
            eprintln!("{}\n", show(e, repl.files(), color));
        }
        if let Some(v) = r.value {
            println!("{}", v);
        }
    }
    0
}

/// El REPL corre en un hilo con pila grande, igual que los programas.
fn repl_thread() -> i32 {
    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(repl)
        .expect("no se pudo crear el hilo del REPL")
        .join()
        .unwrap_or(101)
}

fn real_main(argv: &[String]) -> i32 {
    let first = argv.first().map(String::as_str);
    if first.is_none() || first == Some("repl") {
        return repl_thread();
    }
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
    let color = use_color();
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
