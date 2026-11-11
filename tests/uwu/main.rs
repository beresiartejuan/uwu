//! Tests de integración: cada archivo cubre una parte del lenguaje.

mod ejemplos;
mod errores;
mod lenguaje;
mod modulos;
mod repl;
mod stdlib;
mod tipos;

use uwu::diagnostics::Severity;
use uwu::run;

/// Ejecuta código uwu y devuelve lo que imprimió. Falla si hay errores de compilación.
fn exec(code: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let r = run(code, &mut |l| out.push(l.to_string()), false, &[]);
    let errors: Vec<String> =
        r.diagnostics.iter().filter(|d| d.severity != Severity::Warning).map(|d| d.message.clone()).collect();
    assert!(errors.is_empty(), "errores inesperados:\n{}", errors.join("\n"));
    assert!(r.runtime_error.is_none(), "pánico: {}", r.runtime_error.unwrap().message);
    out
}

/// Devuelve los mensajes de error de compilación.
fn errors(code: &str) -> Vec<String> {
    let r = run(code, &mut |_| {}, true, &[]);
    r.diagnostics.iter().filter(|d| d.severity != Severity::Warning).map(|d| d.message.clone()).collect()
}

fn panic_msg(code: &str) -> String {
    let r = run(code, &mut |_| {}, false, &[]);
    r.runtime_error.map(|d| d.message).unwrap_or_default()
}

// ---------- archivos ----------

/// Carpeta temporal propia de cada test, que se borra al terminar.
struct TempDir(std::path::PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("uwu_{}_{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }

    fn write(&self, rel: &str, code: &str) -> std::path::PathBuf {
        let p = self.0.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, code).unwrap();
        p
    }

    fn path(&self) -> String {
        self.0.display().to_string()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Ejecuta un archivo; devuelve el resultado y lo que imprimió.
fn run_path(path: &std::path::Path) -> (uwu::RunResult, Vec<String>) {
    let mut out: Vec<String> = Vec::new();
    let r = uwu::run_file(path, &mut |l| out.push(l.to_string()), false, &[]).unwrap();
    (r, out)
}

/// Ejecuta un archivo y devuelve lo que imprimió. Falla si hay errores.
fn exec_file(path: &std::path::Path) -> Vec<String> {
    let (r, out) = run_path(path);
    let errors: Vec<String> = r
        .diagnostics
        .iter()
        .filter(|d| d.severity != Severity::Warning)
        .map(|d| format!("{} ({})", d.message, r.files[d.span.file].name))
        .collect();
    assert!(errors.is_empty(), "errores inesperados:\n{}", errors.join("\n"));
    assert!(r.runtime_error.is_none(), "pánico: {}", r.runtime_error.unwrap().message);
    out
}

/// Errores de compilación de un archivo, con el nombre del archivo donde ocurrieron.
fn file_errors(path: &std::path::Path) -> Vec<(String, String)> {
    let (r, _) = run_path(path);
    r.diagnostics
        .iter()
        .filter(|d| d.severity != Severity::Warning)
        .map(|d| {
            let file =
                std::path::Path::new(&r.files[d.span.file].name).file_name().unwrap().to_string_lossy().into_owned();
            (d.message.clone(), file)
        })
        .collect()
}
