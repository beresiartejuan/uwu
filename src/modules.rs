//! Carga de módulos: sigue los `import` de cada archivo, detecta ciclos y
//! devuelve los módulos en orden topológico (cada uno después de sus
//! dependencias; el archivo de entrada queda último).
//!
//! - `import <fs>;`            biblioteca estándar
//! - `import "util.uwu";`      archivo, relativo al que importa (`.uwu` es opcional)
//! - `import geometria;`       busca `geometria.uwu` junto al que importa y en `$UWU_PATH`

use crate::diagnostics::{Diagnostic, Severity};
use crate::stdlib;
use crate::syntax::ast::{ImportDecl, ImportTarget, Program, Stmt};
use crate::syntax::lexer::{tokenize, Span};
use crate::syntax::parser::parse;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct SourceFile {
    /// Ruta tal como se muestra en los errores.
    pub name: String,
    pub source: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Dep {
    /// Biblioteca estándar.
    Std(String),
    /// Índice de otro módulo en la lista cargada.
    File(usize),
}

#[derive(Clone, Debug)]
pub struct ModuleDep {
    pub dep: Dep,
    pub reexport: bool,
    pub span: Span,
}

pub struct Module {
    pub file: usize,
    pub program: Program,
    pub deps: Vec<ModuleDep>,
}

pub struct Loaded {
    pub files: Vec<SourceFile>,
    /// `None` si hubo errores de sintaxis o de importación.
    pub modules: Option<Vec<Module>>,
    pub diagnostics: Vec<Diagnostic>,
}

struct Loader {
    /// Índice global del primer archivo de esta carga (el REPL hace varias).
    first_file: usize,
    files: Vec<SourceFile>,
    modules: Vec<Module>,
    by_path: HashMap<PathBuf, usize>,
    /// Archivos que se están cargando ahora mismo (para detectar ciclos).
    stack: Vec<PathBuf>,
    diagnostics: Vec<Diagnostic>,
}

/// Carga el archivo de entrada y todo lo que importa.
/// `path` es su ruta en disco (None si el código no viene de un archivo);
/// `dir` es la carpeta desde la que se resuelven sus imports.
/// `first_file` es el índice que recibe el primer archivo en los `Span`.
pub fn load(source: String, name: String, path: Option<PathBuf>, dir: PathBuf, first_file: usize) -> Loaded {
    let mut loader = Loader {
        first_file,
        files: Vec::new(),
        modules: Vec::new(),
        by_path: HashMap::new(),
        stack: Vec::new(),
        diagnostics: Vec::new(),
    };
    let canon = path.and_then(|p| p.canonicalize().ok());
    let dir = dir.canonicalize().unwrap_or(dir);
    let ok = loader.load_file(source, name, canon, dir).is_ok();
    Loaded {
        files: loader.files,
        modules: if ok { Some(loader.modules) } else { None },
        diagnostics: loader.diagnostics,
    }
}

fn error(message: impl Into<String>, span: Span, hint: Option<String>) -> Diagnostic {
    Diagnostic { message: message.into(), span, hint, severity: Severity::Error }
}

fn display_path(p: &Path) -> String {
    let cwd = std::env::current_dir().ok();
    match cwd.as_deref().and_then(|c| p.strip_prefix(c).ok()) {
        Some(rel) => rel.display().to_string(),
        None => p.display().to_string(),
    }
}

impl Loader {
    fn load_file(&mut self, source: String, name: String, canon: Option<PathBuf>, dir: PathBuf) -> Result<usize, ()> {
        let file = self.first_file + self.files.len();
        self.files.push(SourceFile { name, source });
        let source = &self.files.last().unwrap().source;
        let program = match tokenize(source, file).and_then(|t| parse(t, file)) {
            Ok(p) => p,
            Err(e) => {
                self.diagnostics.extend(e.diagnostics);
                return Err(());
            }
        };

        if let Some(c) = &canon {
            self.stack.push(c.clone());
        }
        let mut deps: Vec<ModuleDep> = Vec::new();
        let mut failed = false;
        for stmt in &program.stmts {
            let Stmt::Import(imp) = stmt else { continue };
            match self.resolve(imp, &dir) {
                Ok(dep) => match deps.iter_mut().find(|d| d.dep == dep) {
                    // importar dos veces lo mismo no es un error, como en C++
                    Some(existing) => existing.reexport |= imp.reexport,
                    None => deps.push(ModuleDep { dep, reexport: imp.reexport, span: imp.span }),
                },
                Err(()) => failed = true,
            }
        }
        if canon.is_some() {
            self.stack.pop();
        }
        if failed {
            return Err(());
        }

        let idx = self.modules.len();
        self.modules.push(Module { file, program, deps });
        if let Some(c) = canon {
            self.by_path.insert(c, idx);
        }
        Ok(idx)
    }

    fn resolve(&mut self, imp: &ImportDecl, dir: &Path) -> Result<Dep, ()> {
        let span = imp.span;
        let candidates: Vec<PathBuf> = match &imp.target {
            ImportTarget::Std(name) => {
                if stdlib::module(name).is_some() {
                    return Ok(Dep::Std(name.clone()));
                }
                let hint = match stdlib::suggest(name) {
                    Some(s) => format!("¿quisiste decir `import <{}>;`?", s),
                    None => format!(
                        "las disponibles son: {}. Para importar un archivo usá comillas: `import \"{}.uwu\";`",
                        stdlib::module_names(),
                        name
                    ),
                };
                self.diagnostics.push(error(
                    format!("no existe la biblioteca estándar `<{}>`", name),
                    span,
                    Some(hint),
                ));
                return Err(());
            }
            ImportTarget::File(rel) => {
                let base = dir.join(rel);
                if Path::new(rel).extension().is_none() {
                    vec![base.with_extension("uwu"), base]
                } else {
                    vec![base]
                }
            }
            ImportTarget::Module(name) => {
                let mut dirs = vec![dir.to_path_buf()];
                if let Ok(extra) = std::env::var("UWU_PATH") {
                    dirs.extend(extra.split(':').filter(|s| !s.is_empty()).map(PathBuf::from));
                }
                dirs.into_iter().map(|d| d.join(format!("{}.uwu", name))).collect()
            }
        };

        let Some(found) = candidates.iter().find(|p| p.is_file()) else {
            let (what, std_name) = match &imp.target {
                ImportTarget::File(rel) => {
                    (format!("no se encontró el archivo `{}`", rel), rel.trim_end_matches(".uwu"))
                }
                ImportTarget::Module(name) => (format!("no se encontró el módulo `{}`", name), name.as_str()),
                ImportTarget::Std(_) => unreachable!(),
            };
            let hint = if stdlib::module(std_name).is_some() {
                format!("`{}` es una biblioteca estándar: se importa con `import <{}>;`", std_name, std_name)
            } else {
                let tried: Vec<String> = candidates.iter().map(|p| display_path(p)).collect();
                format!("se buscó en: {}", tried.join(", "))
            };
            self.diagnostics.push(error(what, span, Some(hint)));
            return Err(());
        };
        let found = found.clone();
        let canon = found.canonicalize().unwrap_or_else(|_| found.clone());

        if let Some(pos) = self.stack.iter().position(|p| p == &canon) {
            let mut chain: Vec<String> = self.stack[pos..].iter().map(|p| display_path(p)).collect();
            chain.push(display_path(&canon));
            self.diagnostics.push(error(
                format!("importación circular: {}", chain.join(" → ")),
                span,
                Some("sacá las funciones compartidas a un tercer archivo que importen los dos".into()),
            ));
            return Err(());
        }

        let idx = match self.by_path.get(&canon) {
            Some(idx) => *idx,
            None => {
                let source = match std::fs::read_to_string(&found) {
                    Ok(s) => s,
                    Err(e) => {
                        self.diagnostics.push(error(
                            format!("no se pudo leer `{}`: {}", display_path(&found), e),
                            span,
                            None,
                        ));
                        return Err(());
                    }
                };
                let sub_dir = canon.parent().map(Path::to_path_buf).unwrap_or_default();
                self.load_file(source, display_path(&found), Some(canon), sub_dir)?
            }
        };

        // `import geometria;` exige que el archivo, si declara un nombre, declare ése
        if let ImportTarget::Module(name) = &imp.target {
            if let Some((declared, _)) = &self.modules[idx].program.module_name {
                if declared != name {
                    self.diagnostics.push(error(
                        format!("se importó `{}`, pero el archivo declara `export module {};`", name, declared),
                        span,
                        Some(format!("usá `import {};` o cambiá la declaración del módulo", declared)),
                    ));
                    return Err(());
                }
            }
        }
        Ok(Dep::File(idx))
    }
}
