#[cfg(not(target_os = "linux"))]
compile_error!("por ahora uwu sólo soporta Linux");

pub mod checker;
pub mod diagnostics;
pub mod interpreter;
pub mod modules;
pub mod repl;
pub mod stdlib;
pub mod syntax;
pub mod types;

use checker::Checker;
use diagnostics::{Diagnostic, Severity};
use interpreter::Interpreter;
use modules::{Module, SourceFile};
use std::path::{Path, PathBuf};

pub use diagnostics::format_diagnostic;

/// El intérprete es recursivo: corre en un hilo con esta pila.
pub const STACK_SIZE: usize = 256 * 1024 * 1024;

pub struct RunResult {
    /// Errores y warnings de compilación.
    pub diagnostics: Vec<Diagnostic>,
    /// Pánico durante la ejecución, si lo hubo.
    pub runtime_error: Option<Diagnostic>,
    pub ran: bool,
    /// Todos los archivos que participaron; `span.file` indexa esta lista.
    pub files: Vec<SourceFile>,
}

pub struct Compiled {
    pub files: Vec<SourceFile>,
    /// `None` si no se pudo cargar (errores de sintaxis o de `import`).
    pub modules: Option<Vec<Module>>,
    pub checker: Checker,
    pub diagnostics: Vec<Diagnostic>,
}

fn compile_entry(source: String, name: String, path: Option<PathBuf>, dir: PathBuf) -> Compiled {
    let loaded = modules::load(source, name, path, dir, 0);
    let mut checker = Checker::new();
    let mut diagnostics = loaded.diagnostics;
    if let Some(mods) = &loaded.modules {
        for (i, m) in mods.iter().enumerate() {
            checker.check_module(i, m, i + 1 == mods.len());
        }
        diagnostics.extend(checker.diagnostics.iter().cloned());
    }
    Compiled { files: loaded.files, modules: loaded.modules, checker, diagnostics }
}

/// Compila código suelto; sus `import` se resuelven desde el directorio actual.
pub fn compile(source: &str) -> (Option<Compiled>, Vec<Diagnostic>) {
    let dir = std::env::current_dir().unwrap_or_default();
    let c = compile_entry(source.to_string(), "<entrada>".into(), None, dir);
    let diagnostics = c.diagnostics.clone();
    (Some(c), diagnostics)
}

/// Chequea y ejecuta código suelto (sus `import` se resuelven desde el directorio actual).
pub fn run(source: &str, out: &mut (dyn FnMut(&str) + Send), only_check: bool, script_args: &[String]) -> RunResult {
    let dir = std::env::current_dir().unwrap_or_default();
    let compiled = compile_entry(source.to_string(), "<entrada>".into(), None, dir);
    execute(compiled, out, only_check, script_args)
}

/// Chequea y ejecuta un archivo `.uwu`; sus `import` se resuelven desde su carpeta.
pub fn run_file(
    path: &Path,
    out: &mut (dyn FnMut(&str) + Send),
    only_check: bool,
    script_args: &[String],
) -> std::io::Result<RunResult> {
    let source = std::fs::read_to_string(path)?;
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let dir = if dir.as_os_str().is_empty() { PathBuf::from(".") } else { dir };
    let compiled = compile_entry(source, path.display().to_string(), Some(path.to_path_buf()), dir);
    Ok(execute(compiled, out, only_check, script_args))
}

fn execute(
    compiled: Compiled,
    out: &mut (dyn FnMut(&str) + Send),
    only_check: bool,
    script_args: &[String],
) -> RunResult {
    let Compiled { files, modules, checker, diagnostics } = compiled;
    let has_errors = diagnostics.iter().any(|d| d.severity != Severity::Warning);
    let Some(modules) = modules else {
        return RunResult { diagnostics, runtime_error: None, ran: false, files };
    };
    if has_errors || only_check {
        return RunResult { diagnostics, runtime_error: None, ran: false, files };
    }
    // El intérprete es recursivo: le damos una pila grande y un límite de
    // profundidad propio para reportar `desbordamiento de pila` como pánico.
    let modules = &modules;
    let Checker { tc, pattern_types, .. } = checker;
    let result = std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(STACK_SIZE)
            .spawn_scoped(scope, move || {
                let mut interp = Interpreter::new(out, tc, pattern_types, script_args);
                interp.run(modules)
            })
            .expect("no se pudo crear el hilo del intérprete")
            .join()
            .expect("el intérprete abortó")
    });
    let runtime_error =
        result.err().map(|e| Diagnostic { message: e.message, span: e.span, hint: None, severity: Severity::Error });
    RunResult { diagnostics, runtime_error, ran: true, files }
}
