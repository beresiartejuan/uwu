//! El REPL: cada entrada se chequea y se ejecuta como la continuación de las
//! anteriores. Las variables, funciones, tipos e imports se conservan, y se
//! puede redefinir lo que ya existía.
//!
//! Si una entrada tiene errores (de tipos o en ejecución), el checker vuelve al
//! estado anterior, así nunca cree que existe algo que no se llegó a definir.

use crate::checker::Checker;
use crate::diagnostics::{Diagnostic, Severity};
use crate::interpreter::{Env, Interpreter};
use crate::modules::{self, Module, SourceFile};
use crate::syntax::lexer::{tokenize, TokenKind};
use crate::syntax::parser::parse;
use crate::types::print;
use std::rc::Rc;

pub struct Repl<'o> {
    checker: Checker,
    interp: Interpreter<'static, 'o>,
    globals: Rc<Env<'static>>,
    env: Rc<Env<'static>>,
    /// Todas las entradas (y los archivos que importaron); `span.file` indexa esta lista.
    files: Vec<SourceFile>,
}

pub struct ReplOutput {
    /// Errores y warnings de compilación.
    pub diagnostics: Vec<Diagnostic>,
    /// Pánico durante la ejecución, si lo hubo.
    pub runtime_error: Option<Diagnostic>,
    /// El valor de la última expresión, si la entrada termina en una (y no es `void`).
    pub value: Option<String>,
}

const NAME: &str = "<repl>";

impl<'o> Repl<'o> {
    pub fn new(out: &'o mut dyn FnMut(&str)) -> Self {
        let checker = Checker::new_repl();
        let interp = Interpreter::new(out, checker.tc.clone(), checker.pattern_types.clone(), &[]);
        let globals = Interpreter::global_env();
        let env = Env::new(Some(Rc::clone(&globals)));
        Self { checker, interp, globals, env, files: Vec::new() }
    }

    /// Olvida todo lo definido.
    pub fn reset(&mut self) {
        self.checker = Checker::new_repl();
        self.env = Env::new(Some(Rc::clone(&self.globals)));
    }

    pub fn files(&self) -> &[SourceFile] {
        &self.files
    }

    /// ¿Falta cerrar algún `(`, `[`, `{` o comentario? Entonces hay que seguir leyendo.
    pub fn is_incomplete(input: &str) -> bool {
        let Ok(tokens) = tokenize(input, 0) else {
            return input.contains("/*") && !input.contains("*/");
        };
        let mut depth = 0i32;
        for t in tokens.iter().filter(|t| t.kind == TokenKind::Op) {
            match t.value.as_str() {
                "(" | "[" | "{" => depth += 1,
                ")" | "]" | "}" => depth -= 1,
                _ => {}
            }
        }
        depth > 0
    }

    /// Chequea y ejecuta una entrada.
    pub fn eval(&mut self, input: &str) -> ReplOutput {
        let (checker, modules, diagnostics) = self.compile(input);
        let mut out = ReplOutput { diagnostics, runtime_error: None, value: None };
        let Some(modules) = modules else { return out };
        if out.diagnostics.iter().any(|d| d.severity == Severity::Error) {
            return out;
        }
        // Los valores del REPL (funciones, closures) apuntan al AST de su entrada,
        // que tiene que vivir hasta el final de la sesión.
        let modules: &'static [Module] = Box::leak(modules.into_boxed_slice());
        self.interp.set_types(checker.tc.clone(), checker.pattern_types.clone());
        match self.interp.run_repl(modules, &self.globals, &self.env) {
            Ok(value) => {
                out.value = value;
                self.checker = checker;
            }
            Err(e) => {
                out.runtime_error =
                    Some(Diagnostic { message: e.message, span: e.span, hint: None, severity: Severity::Error });
            }
        }
        out
    }

    /// El tipo de una expresión, sin ejecutarla (el comando `:tipo`).
    pub fn type_of(&mut self, input: &str) -> Result<String, Vec<Diagnostic>> {
        let (checker, modules, diagnostics) = self.compile(input);
        let errors: Vec<Diagnostic> = diagnostics.into_iter().filter(|d| d.severity == Severity::Error).collect();
        if modules.is_none() || !errors.is_empty() {
            return Err(errors);
        }
        Ok(checker.last_type.as_ref().map(print).unwrap_or_else(|| "void".into()))
    }

    /// Devuelve el checker con la entrada ya chequeada (sin confirmar) y sus módulos.
    fn compile(&mut self, input: &str) -> (Checker, Option<Vec<Module>>, Vec<Diagnostic>) {
        // `let x = 5` sin `;` al final también vale
        let parses = |src: &str| tokenize(src, 0).and_then(|t| parse(t, 0)).is_ok();
        let trimmed = input.trim_end();
        let source = if !parses(trimmed) && parses(&format!("{};", trimmed)) {
            format!("{};", trimmed)
        } else {
            trimmed.to_string()
        };

        let dir = std::env::current_dir().unwrap_or_default();
        let loaded = modules::load(source, NAME.into(), None, dir, self.files.len());
        self.files.extend(loaded.files);
        let mut diagnostics = loaded.diagnostics;
        let mut checker = self.checker.clone();
        checker.diagnostics.clear();
        let Some(mods) = loaded.modules else { return (checker, None, diagnostics) };
        for (i, m) in mods.iter().enumerate() {
            checker.check_module(i, m, i + 1 == mods.len());
        }
        diagnostics.append(&mut checker.diagnostics);
        (checker, Some(mods), diagnostics)
    }
}
