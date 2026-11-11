//! El chequeo de tipos: recorre cada módulo, infiere los tipos, resuelve los
//! genéricos y verifica que los `match` sean exhaustivos. No ejecuta nada.

use crate::diagnostics::{Diagnostic, Severity};
use crate::modules::{Dep, Module};
use crate::stdlib;
use crate::syntax::ast::*;
use crate::syntax::Span;
use crate::types::{
    alias_key, arr, clean_name, contains_param, has_never_array, is_fresh, lit, mk_union, obj_get, param, print, subst,
    t_fn, t_fn_gen, t_obj, widen, widen_value, Alias, Type, TypeCtx, BOOLEAN, ERR, NEVER, NUMBER, RANGE, STRING, VOID,
};
use std::collections::{HashMap, HashSet};

mod call;
mod expr;
mod pattern;
mod stmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VarKind {
    Var,
    /// La variable de un `for`: se hace mutable con `for let x in ...`.
    LoopVar,
    Fn,
    Builtin,
    /// Traído con `import` desde otro archivo.
    Imported,
}

/// Lo que un módulo deja ver con `export`.
#[derive(Clone, Default)]
struct ModuleExports {
    values: Vec<(String, VarInfo)>,
    /// (nombre visible, clave interna del alias)
    types: Vec<(String, String)>,
    /// Bibliotecas estándar re-exportadas con `export import <fs>;`
    namespaces: Vec<String>,
    /// `export namespace geo { ... }`
    user_ns: Vec<(String, NsInfo)>,
}

/// Los miembros de un `namespace` declarado en uwu.
#[derive(Clone, Default)]
struct NsInfo {
    values: HashMap<String, VarInfo>,
    /// (nombre sin prefijo, clave interna del alias)
    types: Vec<(String, String)>,
    /// Los `let` del namespace, para avisar si se usan antes de declararse.
    lets: Vec<String>,
}

/// Lo que el REPL conserva de una entrada a la siguiente.
#[derive(Clone, Default)]
struct ReplState {
    frame: HashMap<String, VarInfo>,
    type_scope: HashMap<String, String>,
    namespaces: HashSet<String>,
    user_ns: HashMap<String, NsInfo>,
}

/// Lo que hay que restaurar al salir de un namespace (ver `enter_ns`).
struct NsView {
    name: String,
    shadowed: Vec<(String, Option<String>)>,
}

#[derive(Clone)]
struct VarInfo {
    ty: Type,
    mutable: bool,
    kind: VarKind,
}

#[derive(Clone)]
struct FnCtx {
    ret: Option<Type>, // None = inferir (closures)
    inferred: Vec<Type>,
}

#[derive(Clone)]
struct LoopCtx {
    has_break: bool,
}

fn primitive(name: &str) -> Option<Type> {
    match name {
        "number" => Some(NUMBER),
        "string" => Some(STRING),
        "boolean" => Some(BOOLEAN),
        "void" => Some(VOID),
        "never" => Some(NEVER),
        _ => None,
    }
}

fn type_hint(name: &str) -> Option<&'static str> {
    match name {
        "int" | "float" | "i32" | "i64" | "f64" | "u32" | "usize" | "Number" => Some("number"),
        "str" | "String" => Some("string"),
        "bool" | "Boolean" => Some("boolean"),
        _ => None,
    }
}

const MUTATING_METHODS: &[&str] = &["push", "pop"];

/// Métodos cuyos últimos parámetros se pueden omitir: (método, cuántos).
/// `xs.slice(2)` es lo mismo que `xs.slice(2, xs.len())`.
const OPTIONAL_ARGS: &[(&str, usize)] = &[("slice", 1)];

#[derive(Clone)]
pub struct Checker {
    pub diagnostics: Vec<Diagnostic>,
    pub tc: TypeCtx,
    /// Tipos de los patrones `x: T`, que el intérprete necesita para chequear en ejecución.
    pub pattern_types: HashMap<usize, Type>,

    scopes: Vec<HashMap<String, VarInfo>>,
    type_param_scopes: Vec<HashSet<String>>,
    fn_stack: Vec<FnCtx>,
    loop_stack: Vec<Option<LoopCtx>>,
    signatures: HashMap<usize, Type>,
    fresh_counter: usize,

    // ---- estado por módulo ----
    globals: HashMap<String, VarInfo>,
    module_id: usize,
    /// Tipos visibles en el módulo actual: nombre → clave del alias.
    type_scope: HashMap<String, String>,
    /// Bibliotecas estándar importadas en el módulo actual.
    namespaces: HashSet<String>,
    /// Namespaces declarados en uwu visibles en el módulo actual.
    user_ns: HashMap<String, NsInfo>,
    module_exports: Vec<ModuleExports>,

    // ---- REPL ----
    /// `Some` si el checker es de un REPL: el módulo de entrada continúa el anterior.
    repl: Option<ReplState>,
    /// En el REPL se puede redefinir lo que ya existía.
    redefining: bool,
    /// Tipo de la última expresión del archivo de entrada, si termina en una.
    pub last_type: Option<Type>,
}

impl Default for Checker {
    fn default() -> Self {
        Self::new()
    }
}

impl Checker {
    pub fn new() -> Self {
        let mut globals = HashMap::new();
        for (name, ty) in stdlib::global_types() {
            globals.insert(name.to_string(), VarInfo { ty, mutable: false, kind: VarKind::Builtin });
        }
        Self {
            diagnostics: Vec::new(),
            tc: TypeCtx::new(),
            pattern_types: HashMap::new(),
            scopes: vec![globals.clone()],
            type_param_scopes: Vec::new(),
            fn_stack: Vec::new(),
            loop_stack: Vec::new(),
            signatures: HashMap::new(),
            fresh_counter: 0,
            globals,
            module_id: 0,
            type_scope: HashMap::new(),
            namespaces: HashSet::new(),
            user_ns: HashMap::new(),
            module_exports: Vec::new(),
            repl: None,
            redefining: false,
            last_type: None,
        }
    }

    /// Un checker para el REPL: cada entrada ve lo que definieron las anteriores.
    pub fn new_repl() -> Self {
        Self { repl: Some(ReplState::default()), ..Self::new() }
    }

    fn error(&mut self, message: impl Into<String>, span: Span, hint: Option<String>) {
        self.diagnostics.push(Diagnostic { message: message.into(), span, hint, severity: Severity::Error });
    }

    fn warn(&mut self, message: impl Into<String>, span: Span, hint: Option<String>) {
        self.diagnostics.push(Diagnostic { message: message.into(), span, hint, severity: Severity::Warning });
    }

    fn expect_assignable(&mut self, src: &Type, dst: &Type, span: Span, hint: Option<String>) -> bool {
        if self.tc.is_assignable(src, dst) {
            return true;
        }
        self.error(
            format!("tipos incompatibles: se esperaba `{}`, se encontró `{}`", print(dst), print(src)),
            span,
            hint,
        );
        false
    }

    // ---------- ámbitos ----------

    fn define(&mut self, name: &str, info: VarInfo) {
        self.scopes.last_mut().unwrap().insert(name.to_string(), info);
    }

    fn lookup(&self, name: &str) -> Option<&VarInfo> {
        self.scopes.iter().rev().find_map(|frame| frame.get(name))
    }

    fn names(&self) -> Vec<String> {
        self.scopes.iter().rev().flat_map(|f| f.keys().cloned()).collect()
    }

    // ======================= programa =======================

    /// Chequea un módulo. Hay que llamarlo en orden topológico (`idx` = posición
    /// en la lista de `modules::load`), así sus dependencias ya están chequeadas.
    pub fn check_module(&mut self, idx: usize, module: &Module, is_entry: bool) {
        // `idx` es la posición dentro de esta carga; en el REPL hay módulos de cargas anteriores.
        let base = self.module_exports.len() - idx;
        self.module_id = self.module_exports.len();
        let program = &module.program;
        let repl = if is_entry { self.repl.take() } else { None };
        let in_repl = repl.is_some();
        match repl {
            Some(st) => {
                self.scopes = vec![self.globals.clone(), st.frame];
                self.type_scope = st.type_scope;
                self.namespaces = st.namespaces;
                self.user_ns = st.user_ns;
                self.forget_redefined(&program.stmts);
            }
            None => {
                self.scopes = vec![self.globals.clone(), HashMap::new()];
                self.type_scope = HashMap::new();
                self.namespaces = HashSet::new();
                self.user_ns = HashMap::new();
            }
        }
        self.redefining = in_repl;

        for d in &module.deps {
            match &d.dep {
                Dep::Std(name) => {
                    self.namespaces.insert(name.clone());
                }
                Dep::File(j) => {
                    let ex = self.module_exports[base + *j].clone();
                    self.bring_exports(ex, d.span);
                }
            }
        }

        self.hoist(&program.stmts);
        let mut last = None;
        for s in &program.stmts {
            let t = self.check_stmt(s);
            last = matches!(s, Stmt::Expr { semi: false, .. }).then_some(t);
        }
        if is_entry {
            self.last_type = last;
        }
        if is_entry {
            if let Some(Stmt::Fn(d)) = program.stmts.iter().find(|s| matches!(s, Stmt::Fn(d) if d.name == "main")) {
                if !d.params.is_empty() {
                    self.error("`main` no puede recibir parámetros", d.span, None);
                }
            }
        }

        let mut ex = ModuleExports::default();
        let frame = self.scopes[1].clone();
        let imported = |info: &VarInfo| VarInfo { kind: VarKind::Imported, mutable: false, ..info.clone() };
        for s in &program.stmts {
            let names: Vec<&str> = match s {
                Stmt::Let { exported: true, name, .. } => vec![name],
                Stmt::Destructure { exported: true, pattern, .. } => pattern.names(),
                Stmt::Fn(d) if d.exported => vec![&d.name],
                Stmt::Type(d) if d.exported => {
                    if let Some(key) = self.type_scope.get(&d.name) {
                        ex.types.push((d.name.clone(), key.clone()));
                    }
                    continue;
                }
                Stmt::Namespace(n) if n.exported => {
                    let Some(info) = self.user_ns.get(&n.name) else { continue };
                    for (tn, key) in &info.types {
                        ex.types.push((format!("{}::{}", n.name, tn), key.clone()));
                    }
                    let values = info.values.iter().map(|(k, v)| (k.clone(), imported(v))).collect();
                    ex.user_ns.push((n.name.clone(), NsInfo { values, ..info.clone() }));
                    continue;
                }
                _ => continue,
            };
            for name in names {
                if let Some(info) = frame.get(name) {
                    ex.values.retain(|(n, _)| n != name);
                    ex.values.push((name.to_string(), imported(info)));
                }
            }
        }
        for d in module.deps.iter().filter(|d| d.reexport) {
            match &d.dep {
                Dep::Std(name) => ex.namespaces.push(name.clone()),
                Dep::File(j) => {
                    let sub = self.module_exports[base + *j].clone();
                    ex.values.extend(sub.values);
                    ex.types.extend(sub.types);
                    ex.namespaces.extend(sub.namespaces);
                    ex.user_ns.extend(sub.user_ns);
                }
            }
        }
        self.module_exports.push(ex);
        if in_repl {
            self.repl = Some(ReplState {
                frame: self.scopes[1].clone(),
                type_scope: self.type_scope.clone(),
                namespaces: self.namespaces.clone(),
                user_ns: self.user_ns.clone(),
            });
        }
        self.redefining = false;
    }

    /// REPL: lo que una entrada vuelve a declarar reemplaza a lo anterior.
    fn forget_redefined(&mut self, stmts: &[Stmt]) {
        for s in stmts {
            match s {
                Stmt::Fn(d) => {
                    self.scopes[1].remove(&d.name);
                }
                Stmt::Type(d) => {
                    self.type_scope.remove(&d.name);
                }
                Stmt::Namespace(n) => {
                    if let Some(info) = self.user_ns.remove(&n.name) {
                        for (tn, _) in info.types {
                            self.type_scope.remove(&format!("{}::{}", n.name, tn));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Mete en el módulo actual lo que exporta otro (`import "x.uwu";`).
    fn bring_exports(&mut self, ex: ModuleExports, span: Span) {
        for (name, info) in ex.values {
            if self.scopes[1].contains_key(&name) && !self.redefining {
                self.error(
                    format!("`{}` se importa desde dos módulos distintos", name),
                    span,
                    Some("renombrá una de las dos definiciones".into()),
                );
                continue;
            }
            self.scopes[1].insert(name, info);
        }
        for (name, key) in ex.types {
            if self.type_scope.get(&name).is_some_and(|k| k != &key) && !self.redefining {
                self.error(format!("el tipo `{}` se importa desde dos módulos distintos", name), span, None);
                continue;
            }
            self.type_scope.insert(name, key);
        }
        self.namespaces.extend(ex.namespaces);
        for (name, info) in ex.user_ns {
            if self.user_ns.contains_key(&name) && !self.redefining {
                self.error(format!("el namespace `{}` se importa desde dos módulos distintos", name), span, None);
                continue;
            }
            self.user_ns.insert(name, info);
        }
    }

    /// Registra tipos y funciones de un bloque antes de chequearlo (permite recursión mutua).
    fn hoist(&mut self, stmts: &[Stmt]) {
        // 1. los nombres de todos los tipos, también los de los namespaces
        let mut type_decls: Vec<(&TypeDecl, String, Option<&str>)> = Vec::new();
        let mut local = HashSet::new();
        for s in stmts {
            match s {
                Stmt::Type(d) => {
                    if let Some(key) = self.declare_type(d, None, &mut local) {
                        type_decls.push((d, key, None));
                    }
                }
                Stmt::Namespace(n) => {
                    if self.user_ns.contains_key(&n.name) || self.namespaces.contains(&n.name) {
                        let hint = self
                            .namespaces
                            .contains(&n.name)
                            .then(|| format!("ya importaste la biblioteca `<{}>`: elegí otro nombre", n.name));
                        self.error(format!("el namespace `{}` ya existe", n.name), n.span, hint);
                        continue;
                    }
                    let lets = n.stmts.iter().flat_map(|s| match s {
                        Stmt::Let { name, .. } => vec![name.clone()],
                        Stmt::Destructure { pattern, .. } => pattern.names().into_iter().map(String::from).collect(),
                        _ => vec![],
                    });
                    self.user_ns.insert(n.name.clone(), NsInfo { lets: lets.collect(), ..NsInfo::default() });
                    let mut ns_local = HashSet::new();
                    for s in &n.stmts {
                        if let Stmt::Type(d) = s {
                            if let Some(key) = self.declare_type(d, Some(&n.name), &mut ns_local) {
                                type_decls.push((d, key, Some(n.name.as_str())));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        // 2. sus definiciones
        for (d, key, ns) in &type_decls {
            let view = ns.map(|ns| self.enter_ns(ns));
            self.type_param_scopes.push(d.params.iter().cloned().collect());
            let body = self.lower_type(&d.ty);
            self.type_param_scopes.pop();
            if let Some(view) = view {
                self.leave_ns(view);
            }
            let body_is_err = matches!(body, Type::Err);
            self.tc.aliases.get_mut(key).unwrap().body = body;
            let probe = Type::Ref { name: key.clone(), args: d.params.iter().map(|_| ERR).collect() };
            if matches!(self.tc.resolve(&probe), Type::Err) && !body_is_err {
                self.error(
                    format!("el tipo `{}` se refiere a sí mismo sin estructura", d.name),
                    d.span,
                    Some(
                        "un tipo recursivo tiene que pasar por un objeto o un arreglo, ej: `{ siguiente: Lista }`"
                            .into(),
                    ),
                );
                self.tc.aliases.get_mut(key).unwrap().body = ERR;
            }
        }
        // 3. las firmas de las funciones
        for s in stmts {
            if let Stmt::Fn(d) = s {
                self.declare_fn(d);
            }
        }
        for s in stmts {
            let Stmt::Namespace(n) = s else { continue };
            let view = self.enter_ns(&n.name);
            for s in &n.stmts {
                if let Stmt::Fn(d) = s {
                    self.declare_fn(d);
                }
            }
            self.leave_ns(view);
        }
    }

    /// Registra el nombre de un tipo (su cuerpo se resuelve después). `local`
    /// son los nombres ya declarados en la misma lista, para detectar repetidos.
    fn declare_type(&mut self, d: &TypeDecl, ns: Option<&str>, local: &mut HashSet<String>) -> Option<String> {
        let taken = local.contains(&d.name) || (ns.is_none() && self.type_scope.contains_key(&d.name));
        if taken || primitive(&d.name).is_some() {
            self.error(format!("el tipo `{}` ya está definido", d.name), d.span, None);
            return None;
        }
        local.insert(d.name.clone());
        let qualified = match ns {
            Some(ns) => format!("{}::{}", ns, d.name),
            None => d.name.clone(),
        };
        let key = alias_key(&qualified, self.module_id);
        self.type_scope.insert(qualified, key.clone());
        if let Some(ns) = ns {
            self.user_ns.get_mut(ns).unwrap().types.push((d.name.clone(), key.clone()));
        }
        self.tc.aliases.insert(key.clone(), Alias { params: d.params.clone(), body: ERR });
        Some(key)
    }

    fn declare_fn(&mut self, d: &FnDecl) {
        if let Some(existing) = self.scopes.last().unwrap().get(&d.name) {
            let msg = if existing.kind == VarKind::Imported {
                format!("`{}` ya fue importado de otro módulo", d.name)
            } else {
                format!("`{}` ya está definido en este bloque", d.name)
            };
            self.error(msg, d.span, None);
            return;
        }
        self.type_param_scopes.push(d.type_params.iter().cloned().collect());
        let params: Vec<Type> =
            d.params.iter().map(|p| p.ty.as_ref().map(|t| self.lower_type(t)).unwrap_or(ERR)).collect();
        let ret = d.ret.as_ref().map(|t| self.lower_type(t)).unwrap_or(VOID);
        self.type_param_scopes.pop();
        let sig = t_fn_gen(params, ret, d.type_params.clone());
        self.signatures.insert(d.id, sig.clone());
        self.define(&d.name, VarInfo { ty: sig, mutable: false, kind: VarKind::Fn });
    }

    /// Adentro de un namespace sus miembros y tipos se usan sin prefijo.
    fn enter_ns(&mut self, name: &str) -> NsView {
        let info = self.user_ns.get(name).cloned().unwrap_or_default();
        self.scopes.push(info.values);
        let shadowed = info.types.into_iter().map(|(n, key)| (n.clone(), self.type_scope.insert(n, key))).collect();
        NsView { name: name.to_string(), shadowed }
    }

    /// Sale del namespace y guarda lo que se declaró adentro.
    fn leave_ns(&mut self, view: NsView) {
        let frame = self.scopes.pop().unwrap_or_default();
        if let Some(info) = self.user_ns.get_mut(&view.name) {
            info.values = frame;
        }
        for (n, prev) in view.shadowed {
            match prev {
                Some(key) => self.type_scope.insert(n, key),
                None => self.type_scope.remove(&n),
            };
        }
    }

    fn lower_type(&mut self, node: &TypeNode) -> Type {
        match node {
            TypeNode::Name { name, args, span } => {
                if let Some(prim) = primitive(name) {
                    if !args.is_empty() {
                        self.error(format!("`{}` no recibe parámetros de tipo", name), *span, None);
                    }
                    return prim;
                }
                if self.type_param_scopes.iter().any(|s| s.contains(name)) {
                    return param(name.clone());
                }
                if let Some(key) = self.type_scope.get(name).cloned() {
                    let expected = self.tc.aliases.get(&key).map_or(0, |a| a.params.len());
                    if expected != args.len() {
                        self.error(
                            format!("`{}` espera {} parámetro(s) de tipo, recibió {}", name, expected, args.len()),
                            *span,
                            None,
                        );
                        return ERR;
                    }
                    let targs: Vec<Type> = args.iter().map(|a| self.lower_type(a)).collect();
                    return Type::Ref { name: key, args: targs };
                }
                let hint = type_hint(name).map(|s| format!("en uwu se escribe `{}`", s));
                self.error(format!("tipo desconocido `{}`", name), *span, hint);
                ERR
            }
            TypeNode::Lit { value, .. } => lit(value.clone()),
            TypeNode::Array { elem, .. } => arr(self.lower_type(elem)),
            TypeNode::Tuple { items, .. } => Type::Tuple(items.iter().map(|t| self.lower_type(t)).collect()),
            TypeNode::Object { fields, span } => {
                let mut out: Vec<(String, Type)> = Vec::new();
                for (name, ft) in fields {
                    if out.iter().any(|(n, _)| n == name) {
                        self.error(format!("campo `{}` repetido", name), *span, None);
                    }
                    let t = self.lower_type(ft);
                    if let Some(entry) = out.iter_mut().find(|(n, _)| n == name) {
                        entry.1 = t;
                    } else {
                        out.push((name.clone(), t));
                    }
                }
                Type::Object(out)
            }
            TypeNode::Union { members, .. } => mk_union(members.iter().map(|m| self.lower_type(m)).collect()),
            TypeNode::Fn { params, ret, .. } => {
                let ps: Vec<Type> = params.iter().map(|p| self.lower_type(p)).collect();
                t_fn(ps, self.lower_type(ret))
            }
        }
    }
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.len().abs_diff(b.len()) > 2 {
        return 99;
    }
    // Una sola fila: prev[j] es la distancia entre a[..i] y b[..j].
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut row = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            row[j + 1] = (prev[j + 1] + 1).min(row[j] + 1).min(prev[j] + cost);
        }
        prev = row;
    }
    prev[b.len()]
}
