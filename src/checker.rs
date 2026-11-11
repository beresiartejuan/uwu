use crate::ast::*;
use crate::diagnostics::{Diagnostic, Severity};
use crate::lexer::Span;
use crate::modules::{Dep, Module};
use crate::stdlib;
use crate::types::{
    alias_key, arr, clean_name, contains_param, has_never_array, is_fresh, lit, mk_union, obj_get, param, print, subst,
    t_fn, t_fn_gen, t_obj, widen, Alias, Type, TypeCtx, BOOLEAN, ERR, NEVER, NUMBER, RANGE, STRING, VOID,
};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VarKind {
    Var,
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
}

#[derive(Clone)]
struct VarInfo {
    ty: Type,
    mutable: bool,
    kind: VarKind,
}

struct FnCtx {
    ret: Option<Type>, // None = inferir (closures)
    inferred: Vec<Type>,
}

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
    module_exports: Vec<ModuleExports>,
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
            module_exports: Vec::new(),
        }
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
        self.module_id = idx;
        self.scopes = vec![self.globals.clone(), HashMap::new()];
        self.type_scope = HashMap::new();
        self.namespaces = HashSet::new();

        for d in &module.deps {
            match &d.dep {
                Dep::Std(name) => {
                    self.namespaces.insert(name.clone());
                }
                Dep::File(j) => {
                    let ex = self.module_exports[*j].clone();
                    self.bring_exports(ex, d.span);
                }
            }
        }

        let program = &module.program;
        self.hoist(&program.stmts);
        for s in &program.stmts {
            self.check_stmt(s);
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
        for s in &program.stmts {
            let name = match s {
                Stmt::Let { exported: true, name, .. } => name,
                Stmt::Fn(d) if d.exported => &d.name,
                Stmt::Type(d) if d.exported => {
                    if let Some(key) = self.type_scope.get(&d.name) {
                        ex.types.push((d.name.clone(), key.clone()));
                    }
                    continue;
                }
                _ => continue,
            };
            if let Some(info) = frame.get(name) {
                ex.values.retain(|(n, _)| n != name);
                ex.values.push((name.clone(), VarInfo { kind: VarKind::Imported, mutable: false, ..info.clone() }));
            }
        }
        for d in module.deps.iter().filter(|d| d.reexport) {
            match &d.dep {
                Dep::Std(name) => ex.namespaces.push(name.clone()),
                Dep::File(j) => {
                    let sub = self.module_exports[*j].clone();
                    ex.values.extend(sub.values);
                    ex.types.extend(sub.types);
                    ex.namespaces.extend(sub.namespaces);
                }
            }
        }
        self.module_exports.push(ex);
    }

    /// Mete en el módulo actual lo que exporta otro (`import "x.uwu";`).
    fn bring_exports(&mut self, ex: ModuleExports, span: Span) {
        for (name, info) in ex.values {
            if self.scopes[1].contains_key(&name) {
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
            if self.type_scope.get(&name).is_some_and(|k| k != &key) {
                self.error(format!("el tipo `{}` se importa desde dos módulos distintos", name), span, None);
                continue;
            }
            self.type_scope.insert(name, key);
        }
        self.namespaces.extend(ex.namespaces);
    }

    /// Registra tipos y funciones de un bloque antes de chequearlo (permite recursión mutua).
    fn hoist(&mut self, stmts: &[Stmt]) {
        let mut type_decls: Vec<(&TypeDecl, String)> = Vec::new();
        for s in stmts {
            let Stmt::Type(d) = s else { continue };
            if self.type_scope.contains_key(&d.name) || primitive(&d.name).is_some() {
                self.error(format!("el tipo `{}` ya está definido", d.name), d.span, None);
                continue;
            }
            let key = alias_key(&d.name, self.module_id);
            self.type_scope.insert(d.name.clone(), key.clone());
            self.tc.aliases.insert(key.clone(), Alias { params: d.params.clone(), body: ERR });
            type_decls.push((d, key));
        }
        for (d, key) in &type_decls {
            self.type_param_scopes.push(d.params.iter().cloned().collect());
            let body = self.lower_type(&d.ty);
            self.type_param_scopes.pop();
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
        for s in stmts {
            let Stmt::Fn(d) = s else { continue };
            if let Some(existing) = self.scopes.last().unwrap().get(&d.name) {
                let msg = if existing.kind == VarKind::Imported {
                    format!("`{}` ya fue importado de otro módulo", d.name)
                } else {
                    format!("`{}` ya está definido en este bloque", d.name)
                };
                self.error(msg, d.span, None);
                continue;
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

    // ======================= sentencias =======================

    fn check_stmt(&mut self, s: &Stmt) -> Type {
        match s {
            Stmt::Let { mutable, name, ty, init, .. } => {
                let ann = ty.as_ref().map(|t| self.lower_type(t));
                let init_t = self.check_expr(init, ann.as_ref());
                let var_t;
                if let Some(ann) = ann {
                    self.expect_assignable(&init_t, &ann, init.span(), None);
                    var_t = ann;
                } else {
                    let mut t = widen(&init_t);
                    if has_never_array(&t) {
                        self.error(
                            format!("no se puede inferir el tipo de `{}`", name),
                            init.span(),
                            Some(format!(
                                "agregá una anotación, ej: `let {}{}: number[] = [];`",
                                if *mutable { "mut " } else { "" },
                                name
                            )),
                        );
                        t = ERR;
                    }
                    if matches!(t, Type::Void) {
                        self.warn(
                            format!("`{}` tiene tipo `void`: la expresión no produce ningún valor", name),
                            init.span(),
                            None,
                        );
                    }
                    var_t = t;
                }
                self.define(name, VarInfo { ty: var_t, mutable: *mutable, kind: VarKind::Var });
                if matches!(init_t, Type::Never) {
                    NEVER
                } else {
                    VOID
                }
            }
            Stmt::Expr { expr, .. } => self.check_expr(expr, None),
            Stmt::Fn(d) => {
                self.check_fn_decl(d);
                VOID
            }
            Stmt::Type(_) | Stmt::Import(_) => VOID,
            Stmt::Using { ns, name, span } => {
                if !self.require_namespace(ns, *span) {
                    return VOID;
                }
                let members = stdlib::module(ns).unwrap_or_default();
                match name {
                    None => {
                        for (n, m) in members {
                            self.define(
                                n,
                                VarInfo { ty: stdlib::member_type(m), mutable: false, kind: VarKind::Builtin },
                            );
                        }
                    }
                    Some(n) => {
                        if let Some(ty) = self.namespace_member(ns, n, *span) {
                            self.define(n, VarInfo { ty, mutable: false, kind: VarKind::Builtin });
                        }
                    }
                }
                VOID
            }
        }
    }

    /// ¿Está importada la biblioteca `ns`? Si no, explica cómo importarla.
    fn require_namespace(&mut self, ns: &str, span: Span) -> bool {
        if self.namespaces.contains(ns) {
            return true;
        }
        if stdlib::module(ns).is_some() {
            self.error(
                format!("la biblioteca `{}` no está importada", ns),
                span,
                Some(format!("agregá `import <{}>;` al principio del archivo", ns)),
            );
        } else {
            let hint = match stdlib::suggest(ns) {
                Some(s) => format!("¿quisiste decir `{}::`? (se importa con `import <{}>;`)", s, s),
                None => format!("las bibliotecas estándar son: {}", stdlib::module_names()),
            };
            self.error(format!("no existe el módulo `{}`", ns), span, Some(hint));
        }
        false
    }

    fn namespace_member(&mut self, ns: &str, name: &str, span: Span) -> Option<Type> {
        if let Some(m) = stdlib::lookup(ns, name) {
            return Some(stdlib::member_type(&m));
        }
        let members = stdlib::module(ns).unwrap_or_default();
        let similar = members.iter().map(|(n, _)| *n).find(|n| levenshtein(n, name) <= 2);
        let hint = match similar {
            Some(s) => format!("¿quisiste decir `{}::{}`?", ns, s),
            None => format!(
                "`<{}>` tiene: {}",
                ns,
                members.iter().map(|(n, _)| format!("`{}`", n)).collect::<Vec<_>>().join(", ")
            ),
        };
        self.error(format!("la biblioteca `{}` no tiene `{}`", ns, name), span, Some(hint));
        None
    }

    fn check_fn_decl(&mut self, d: &FnDecl) {
        let Some(Type::Fn { params: sig_params, ret: sig_ret, .. }) = self.signatures.get(&d.id).cloned() else {
            return;
        };
        let sig_ret = *sig_ret;
        self.type_param_scopes.push(d.type_params.iter().cloned().collect());
        self.scopes.push(HashMap::new());
        for (i, p) in d.params.iter().enumerate() {
            if self.scopes.last().unwrap().contains_key(&p.name) {
                self.error(format!("parámetro `{}` repetido", p.name), p.span, None);
            }
            let ty = sig_params.get(i).cloned().unwrap_or(ERR);
            self.define(&p.name, VarInfo { ty, mutable: false, kind: VarKind::Var });
        }
        self.fn_stack.push(FnCtx { ret: Some(sig_ret.clone()), inferred: Vec::new() });
        self.loop_stack.push(None);
        let body = self.check_block(&d.body, Some(&sig_ret));
        self.loop_stack.pop();
        self.fn_stack.pop();
        self.scopes.pop();
        self.type_param_scopes.pop();

        if matches!(sig_ret, Type::Void) || self.tc.is_assignable(&body, &sig_ret) {
            return;
        }
        if let Some(tail) = &d.body.tail {
            self.expect_assignable(&body, &sig_ret, tail.span(), None);
            return;
        }
        let hint = match d.body.stmts.last() {
            Some(Stmt::Expr { semi: true, .. }) => {
                "quitá el `;` de la última expresión: en uwu, la última expresión de un bloque es su valor"
            }
            _ => "la última expresión del bloque (sin `;`) es el valor de retorno",
        };
        self.error(
            format!("la función `{}` debe devolver `{}`, pero su cuerpo no produce un valor", d.name, print(&sig_ret)),
            d.span,
            Some(hint.into()),
        );
    }

    fn check_block(&mut self, b: &Block, expected: Option<&Type>) -> Type {
        self.scopes.push(HashMap::new());
        self.hoist(&b.stmts);
        let mut diverges = false;
        for s in &b.stmts {
            if diverges && !matches!(s, Stmt::Fn(_) | Stmt::Type(_)) {
                self.warn("código inalcanzable", s.span(), None);
                // avisamos una sola vez
                self.check_stmt(s);
                diverges = true;
                continue;
            }
            if matches!(self.check_stmt(s), Type::Never) {
                diverges = true;
            }
        }
        let mut result = match &b.tail {
            Some(tail) => self.check_expr(tail, expected),
            None => VOID,
        };
        if diverges {
            result = NEVER;
        }
        self.scopes.pop();
        result
    }

    // ======================= expresiones =======================

    pub fn check_expr(&mut self, e: &Expr, expected: Option<&Type>) -> Type {
        match e {
            Expr::Num { value, .. } => lit(LitVal::Num(*value)),
            Expr::Str { value, .. } => lit(LitVal::Str(value.clone())),
            Expr::Bool { value, .. } => lit(LitVal::Bool(*value)),
            Expr::Ident { name, span } => {
                let Some(v) = self.lookup(name) else {
                    if self.namespaces.contains(name) {
                        self.error(
                            format!("`{}` es una biblioteca, no un valor", name),
                            *span,
                            Some(format!("sus miembros se usan con `::`, ej: `{}::...`", name)),
                        );
                        return ERR;
                    }
                    if stdlib::module(name).is_some() {
                        self.error(
                            format!("no existe `{}` en este ámbito", name),
                            *span,
                            Some(format!(
                                "`{}` es una biblioteca estándar: agregá `import <{}>;` y usá `{}::...`",
                                name, name, name
                            )),
                        );
                        return ERR;
                    }
                    let similar = self.names().into_iter().find(|n| levenshtein(n, name) <= 2);
                    self.error(
                        format!("no existe `{}` en este ámbito", name),
                        *span,
                        similar.map(|s| format!("¿quisiste decir `{}`?", s)),
                    );
                    return ERR;
                };
                v.ty.clone()
            }
            Expr::Path { ns, name, span } => {
                if !self.require_namespace(ns, *span) {
                    return ERR;
                }
                self.namespace_member(ns, name, *span).unwrap_or(ERR)
            }
            Expr::Array { items, .. } => self.check_array(items, expected),
            Expr::Object { fields, span } => self.check_object(fields, *span, expected),
            Expr::Unary { op, expr, span } => {
                let t = self.check_expr(expr, None);
                if op == "-" {
                    if !self.tc.is_assignable(&t, &NUMBER) {
                        self.error(format!("no se puede negar un valor de tipo `{}`", print(&t)), *span, None);
                    }
                    NUMBER
                } else {
                    if !self.tc.is_assignable(&t, &BOOLEAN) {
                        self.error(format!("`!` necesita un `boolean`, se encontró `{}`", print(&t)), *span, None);
                    }
                    BOOLEAN
                }
            }
            Expr::Binary { op, left, right, span } => self.check_binary(op, left, right, *span),
            Expr::Range { start, end, .. } => {
                let st = self.check_expr(start, None);
                self.expect_assignable(&st, &NUMBER, start.span(), None);
                let et = self.check_expr(end, None);
                self.expect_assignable(&et, &NUMBER, end.span(), None);
                RANGE
            }
            Expr::Assign { op, target, value, .. } => self.check_assign(op, target, value),
            Expr::Call { callee, type_args, args, span } => {
                if let Expr::Member { object, name, .. } = &**callee {
                    self.check_method_call(object, name, args, *span)
                } else {
                    self.check_call(callee, type_args, args, *span)
                }
            }
            Expr::Member { object, name, span } => self.check_member(object, name, *span),
            Expr::Index { object, index, .. } => {
                let obj_t = self.check_expr(object, None);
                let t = self.tc.resolve(&obj_t);
                let idx_t = self.check_expr(index, None);
                self.expect_assignable(&idx_t, &NUMBER, index.span(), None);
                match t {
                    Type::Err => ERR,
                    Type::Array(elem) => *elem,
                    t if self.tc.is_assignable(&t, &STRING) => STRING,
                    t => {
                        self.error(
                            format!("no se puede indexar un valor de tipo `{}`", print(&t)),
                            object.span(),
                            None,
                        );
                        ERR
                    }
                }
            }
            Expr::Block { block, .. } => self.check_block(block, expected),
            Expr::If { cond, then, else_, .. } => {
                let ct = self.check_expr(cond, None);
                self.expect_assignable(
                    &ct,
                    &BOOLEAN,
                    cond.span(),
                    Some("uwu no convierte valores a boolean automáticamente".into()),
                );
                let a = self.check_block(then, expected);
                let Some(else_) = else_ else { return VOID };
                let b = self.check_expr(else_, expected);
                mk_union(vec![a, b])
            }
            Expr::While { cond, body, .. } => {
                let ct = self.check_expr(cond, None);
                self.expect_assignable(&ct, &BOOLEAN, cond.span(), None);
                self.loop_stack.push(Some(LoopCtx { has_break: false }));
                self.check_block(body, None);
                self.loop_stack.pop();
                VOID
            }
            Expr::Loop { body, .. } => {
                self.loop_stack.push(Some(LoopCtx { has_break: false }));
                self.check_block(body, None);
                let ctx = self.loop_stack.pop().unwrap().unwrap();
                if ctx.has_break {
                    VOID
                } else {
                    NEVER
                }
            }
            Expr::For { name, iter, body, .. } => {
                let iter_t = self.check_expr(iter, None);
                let it = self.tc.resolve(&iter_t);
                let elem = match &it {
                    Type::Array(e) => (**e).clone(),
                    Type::Range => NUMBER,
                    Type::Err => ERR,
                    t if self.tc.is_assignable(t, &STRING) => STRING,
                    t => {
                        self.error(
                            format!("no se puede recorrer un valor de tipo `{}`", print(t)),
                            iter.span(),
                            Some("`for` recorre arreglos, strings y rangos como `0..10`".into()),
                        );
                        ERR
                    }
                };
                self.scopes.push(HashMap::new());
                self.define(name, VarInfo { ty: elem, mutable: false, kind: VarKind::Var });
                self.loop_stack.push(Some(LoopCtx { has_break: false }));
                self.check_block(body, None);
                self.loop_stack.pop();
                self.scopes.pop();
                VOID
            }
            Expr::Match { subject, arms, span, .. } => self.check_match(subject, arms, *span, expected),
            Expr::Closure { params, body, .. } => self.check_closure(params, body, expected),
            Expr::Return { value, span } => {
                if self.fn_stack.is_empty() {
                    self.error("`return` fuera de una función", *span, None);
                    return NEVER;
                }
                let ret_opt = self.fn_stack.last().unwrap().ret.clone();
                let vt = match value {
                    Some(v) => self.check_expr(v, ret_opt.as_ref()),
                    None => VOID,
                };
                match ret_opt {
                    Some(ret) => {
                        if !matches!(ret, Type::Void) || value.is_some() {
                            let sp = value.as_ref().map(|v| v.span()).unwrap_or(*span);
                            self.expect_assignable(&vt, &ret, sp, None);
                        }
                    }
                    None => self.fn_stack.last_mut().unwrap().inferred.push(vt),
                }
                NEVER
            }
            Expr::Break { span } | Expr::Continue { span } => {
                let kind = if matches!(e, Expr::Break { .. }) { "break" } else { "continue" };
                match self.loop_stack.last_mut() {
                    Some(Some(ctx)) => {
                        if kind == "break" {
                            ctx.has_break = true;
                        }
                    }
                    _ => self.error(format!("`{}` fuera de un bucle", kind), *span, None),
                }
                NEVER
            }
        }
    }

    fn check_array(&mut self, items: &[Expr], expected: Option<&Type>) -> Type {
        let exp = expected.map(|t| self.tc.resolve(t));
        if let Some(Type::Array(elem)) = &exp {
            for item in items {
                let t = self.check_expr(item, Some(elem));
                self.expect_assignable(&t, elem, item.span(), None);
            }
            return exp.unwrap();
        }
        if items.is_empty() {
            return arr(NEVER);
        }
        let member_types: Vec<Type> = items.iter().map(|i| self.check_expr(i, None)).collect();
        arr(widen(&mk_union(member_types)))
    }

    fn check_object(&mut self, fields: &[ObjField], _span: Span, expected: Option<&Type>) -> Type {
        // Si esperamos una unión de objetos, cada campo recibe la unión de sus tipos posibles
        let exp_members: Vec<Type> = expected
            .map(|t| self.tc.flatten(t, false).into_iter().filter(|m| matches!(m, Type::Object(_))).collect())
            .unwrap_or_default();
        let mut out: Vec<(String, Type)> = Vec::new();
        for f in fields {
            if out.iter().any(|(n, _)| n == &f.name) {
                self.error(format!("campo `{}` repetido", f.name), f.span, None);
            }
            let field_exp: Vec<Type> = exp_members
                .iter()
                .filter_map(|m| if let Type::Object(mf) = m { obj_get(mf, &f.name).cloned() } else { None })
                .collect();
            let exp_t = if field_exp.is_empty() { None } else { Some(mk_union(field_exp)) };
            let t = self.check_expr(&f.value, exp_t.as_ref());
            if let Some(entry) = out.iter_mut().find(|(n, _)| n == &f.name) {
                entry.1 = t;
            } else {
                out.push((f.name.clone(), t));
            }
        }
        Type::Object(out)
    }

    fn check_binary(&mut self, op: &str, left: &Expr, right: &Expr, span: Span) -> Type {
        // Los literales de objeto/arreglo se ensanchan: `{ a: 1 } == { a: 2 }` es una comparación válida
        let operand = |this: &mut Self, e: &Expr| {
            let t = this.check_expr(e, None);
            if matches!(e, Expr::Object { .. } | Expr::Array { .. }) {
                widen(&t)
            } else {
                t
            }
        };
        let l = operand(self, left);
        let r = operand(self, right);
        if matches!(l, Type::Err) || matches!(r, Type::Err) {
            return if ["==", "!=", "<", ">", "<=", ">=", "&&", "||"].contains(&op) { BOOLEAN } else { ERR };
        }
        let is_num = |this: &Self, t: &Type| this.tc.is_assignable(t, &NUMBER);
        let is_str = |this: &Self, t: &Type| this.tc.is_assignable(t, &STRING);
        let is_bool = |this: &Self, t: &Type| this.tc.is_assignable(t, &BOOLEAN);
        let is_prim = |this: &Self, t: &Type| is_num(this, t) || is_str(this, t) || is_bool(this, t);
        let ls = print(&l);
        let rs = print(&r);
        match op {
            "+" => {
                if is_num(self, &l) && is_num(self, &r) {
                    return NUMBER;
                }
                if (is_str(self, &l) && is_prim(self, &r)) || (is_str(self, &r) && is_prim(self, &l)) {
                    return STRING;
                }
                self.error(format!("no se puede sumar `{}` con `{}`", ls, rs), span, None);
                ERR
            }
            "-" | "*" | "/" | "%" => {
                if is_num(self, &l) && is_num(self, &r) {
                    return NUMBER;
                }
                self.error(format!("`{}` necesita dos `number`, se encontró `{}` y `{}`", op, ls, rs), span, None);
                ERR
            }
            "<" | ">" | "<=" | ">=" => {
                if !((is_num(self, &l) && is_num(self, &r)) || (is_str(self, &l) && is_str(self, &r))) {
                    self.error(
                        format!("no se puede comparar `{}` con `{}`", ls, rs),
                        span,
                        Some("sólo se comparan dos `number` o dos `string`".into()),
                    );
                }
                BOOLEAN
            }
            "==" | "!=" => {
                if !self.tc.overlaps(&l, &r) {
                    let result = if op == "==" { "false" } else { "true" };
                    self.error(
                        format!(
                            "esta comparación siempre da {}: `{}` y `{}` no tienen valores en común",
                            result, ls, rs
                        ),
                        span,
                        None,
                    );
                }
                BOOLEAN
            }
            "&&" | "||" => {
                if !is_bool(self, &l) {
                    self.error(format!("`{}` necesita un `boolean`, se encontró `{}`", op, ls), left.span(), None);
                }
                if !is_bool(self, &r) {
                    self.error(format!("`{}` necesita un `boolean`, se encontró `{}`", op, rs), right.span(), None);
                }
                BOOLEAN
            }
            _ => ERR,
        }
    }

    /// Para `a.b[0].c = x` devuelve `a`, que es lo que tiene que ser `mut`.
    fn root_ident<'e>(&self, e: &'e Expr) -> Option<&'e str> {
        match e {
            Expr::Ident { name, .. } => Some(name),
            Expr::Member { object, .. } | Expr::Index { object, .. } => self.root_ident(object),
            _ => None,
        }
    }

    fn check_mutable(&mut self, target: &Expr, span: Span) {
        let Some(root) = self.root_ident(target) else { return };
        let root = root.to_string();
        let Some(v) = self.lookup(&root) else { return };
        if v.kind == VarKind::Imported {
            self.error(
                format!("no se puede modificar `{}`: fue importado de otro módulo", root),
                span,
                Some("exportá una función que lo modifique, o hacé una copia local".into()),
            );
        } else if v.kind != VarKind::Var {
            self.error(format!("no se puede modificar `{}`: es una función", root), span, None);
        } else if !v.mutable {
            self.error(
                format!("no se puede modificar `{}` porque es inmutable", root),
                span,
                Some(format!("declarala con `let mut {}`", root)),
            );
        }
    }

    fn check_assign(&mut self, op: &str, target: &Expr, value: &Expr) -> Type {
        let tt = self.check_expr(target, None);
        self.check_mutable(target, target.span());
        if op == "=" {
            let vt = self.check_expr(value, Some(&tt));
            self.expect_assignable(&vt, &tt, value.span(), None);
            return VOID;
        }
        let vt = self.check_expr(value, None);
        let ok = if op == "+=" {
            (self.tc.is_assignable(&tt, &NUMBER) && self.tc.is_assignable(&vt, &NUMBER))
                || (self.tc.is_assignable(&tt, &STRING)
                    && [NUMBER, STRING, BOOLEAN].iter().any(|p| self.tc.is_assignable(&vt, p)))
        } else {
            self.tc.is_assignable(&tt, &NUMBER) && self.tc.is_assignable(&vt, &NUMBER)
        };
        if !ok && !matches!(tt, Type::Err) && !matches!(vt, Type::Err) {
            self.error(
                format!("no se puede aplicar `{}` a `{}` y `{}`", op, print(&tt), print(&vt)),
                value.span(),
                None,
            );
        }
        VOID
    }

    fn check_member(&mut self, object: &Expr, name: &str, span: Span) -> Type {
        let ot = self.check_expr(object, None);
        let members = self.tc.flatten(&ot, false);
        if members.iter().any(|m| matches!(m, Type::Err)) {
            return ERR;
        }
        let all_have = members.iter().all(|m| matches!(m, Type::Object(f) if obj_get(f, name).is_some()));
        if all_have {
            let types: Vec<Type> = members
                .iter()
                .map(|m| if let Type::Object(f) = m { obj_get(f, name).cloned().unwrap_or(ERR) } else { ERR })
                .collect();
            return mk_union(types);
        }
        let some_have = members.iter().any(|m| matches!(m, Type::Object(f) if obj_get(f, name).is_some()));
        if some_have {
            self.error(
                format!("no todas las variantes de `{}` tienen el campo `{}`", print(&ot), name),
                span,
                Some("usá `match` para distinguir las variantes antes de acceder al campo".into()),
            );
            return ERR;
        }
        if self.method_type(&ot, name).is_some() {
            self.error(
                format!("`{}` es un método, no un campo", name),
                span,
                Some(format!("llamalo con paréntesis: `.{}()`", name)),
            );
            return ERR;
        }
        let hint = if members.len() == 1 {
            if let Type::Object(f) = &members[0] {
                if f.is_empty() {
                    None
                } else {
                    Some(format!(
                        "campos disponibles: {}",
                        f.iter().map(|(n, _)| format!("`{}`", n)).collect::<Vec<_>>().join(", ")
                    ))
                }
            } else {
                None
            }
        } else {
            None
        };
        self.error(format!("el tipo `{}` no tiene el campo `{}`", print(&ot), name), span, hint);
        ERR
    }

    // ---------- llamadas ----------

    fn check_call(&mut self, callee: &Expr, type_args: &[TypeNode], args: &[Expr], span: Span) -> Type {
        let ct = self.check_expr(callee, None);
        let ft = self.tc.resolve(&ct);
        if matches!(ft, Type::Err) {
            for a in args {
                self.check_expr(a, None);
            }
            return ERR;
        }
        let Type::Fn { params, ret, type_params, variadic } = ft else {
            self.error(format!("`{}` no es una función", print(&ct)), callee.span(), None);
            for a in args {
                self.check_expr(a, None);
            }
            return ERR;
        };
        let name = match callee {
            Expr::Ident { name, .. } => name.clone(),
            Expr::Path { ns, name, .. } => format!("{}::{}", ns, name),
            _ => "la función".to_string(),
        };
        self.apply_fn(&params, &ret, &type_params, variadic, type_args, args, span, &name)
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_fn(
        &mut self,
        ft_params: &[Type],
        ft_ret: &Type,
        ft_type_params: &[String],
        variadic: bool,
        type_arg_nodes: &[TypeNode],
        args: &[Expr],
        span: Span,
        name: &str,
    ) -> Type {
        if variadic {
            for a in args {
                self.check_expr(a, None);
            }
            return ft_ret.clone();
        }
        // Renombramos los parámetros de tipo para que no choquen con los del llamador
        let mut params: Vec<Type> = ft_params.to_vec();
        let mut ret = ft_ret.clone();
        let mut free_list: Vec<String> = Vec::new();
        let mut free: HashSet<String> = HashSet::new();
        if !ft_type_params.is_empty() {
            let mut fresh: HashMap<String, Type> = HashMap::new();
            for tp in ft_type_params {
                let n = format!("{}'{}", tp, self.fresh_counter);
                self.fresh_counter += 1;
                free_list.push(n.clone());
                free.insert(n.clone());
                fresh.insert(tp.clone(), param(n));
            }
            params = params.iter().map(|p| subst(p, &fresh)).collect();
            ret = subst(&ret, &fresh);
        }
        if args.len() != params.len() {
            self.error(
                format!("`{}` espera {} argumento(s), pero recibió {}", name, params.len(), args.len()),
                span,
                None,
            );
        }
        let mut bind: HashMap<String, Type> = HashMap::new();
        if !type_arg_nodes.is_empty() {
            if type_arg_nodes.len() != free_list.len() {
                self.error(
                    format!(
                        "`{}` tiene {} parámetro(s) de tipo, se indicaron {}",
                        name,
                        free_list.len(),
                        type_arg_nodes.len()
                    ),
                    span,
                    None,
                );
            } else {
                for (i, n) in free_list.iter().enumerate() {
                    let t = self.lower_type(&type_arg_nodes[i]);
                    bind.insert(n.clone(), t);
                }
            }
        }
        let mut arg_types: Vec<Type> = Vec::new();
        for (i, a) in args.iter().enumerate() {
            let Some(pt) = params.get(i) else {
                arg_types.push(self.check_expr(a, None));
                continue;
            };
            let pt = pt.clone();
            let ps = subst(&pt, &bind);
            let has_free = contains_param(&ps, &|n| free.contains(n));
            let exp = if has_free && !matches!(a, Expr::Closure { .. }) { None } else { Some(&ps) };
            let at = self.check_expr(a, exp);
            if !free.is_empty() {
                self.unify(&pt, &at, &mut bind, &free, 0);
            }
            arg_types.push(at);
        }
        for n in &free_list {
            if !bind.contains_key(n) {
                self.error(
                    format!("no se pudo inferir el parámetro de tipo `{}` de `{}`", clean_name(n), name),
                    span,
                    Some(format!("indicalo explícitamente: `{}::<number>(...)`", name)),
                );
                bind.insert(n.clone(), ERR);
            }
        }
        for (i, a) in args.iter().enumerate() {
            if let Some(pt) = params.get(i) {
                let expected = subst(pt, &bind);
                let at = arg_types[i].clone();
                self.expect_assignable(&at, &expected, a.span(), None);
            }
        }
        subst(&ret, &bind)
    }

    /// Inferencia de genéricos: encuentra T comparando el tipo del parámetro con el del argumento.
    fn unify(&self, p: &Type, a: &Type, bind: &mut HashMap<String, Type>, free: &HashSet<String>, depth: u32) {
        if depth > 32 || matches!(a, Type::Err | Type::Never) {
            return;
        }
        if let Type::Param(pn) = p {
            if free.contains(pn) {
                let w = widen(a);
                match bind.get(pn) {
                    None => {
                        bind.insert(pn.clone(), w);
                    }
                    Some(cur) => {
                        if !self.tc.is_assignable(&w, cur) {
                            let new = if self.tc.is_assignable(cur, &w) { w } else { mk_union(vec![cur.clone(), w]) };
                            bind.insert(pn.clone(), new);
                        }
                    }
                }
                return;
            }
        }
        if !contains_param(p, &|n| free.contains(n)) {
            return;
        }
        if let (Type::Ref { name: pn, args: pa }, Type::Ref { name: an, args: aa }) = (p, a) {
            if pn == an {
                for (i, x) in pa.iter().enumerate() {
                    if let Some(y) = aa.get(i) {
                        self.unify(x, y, bind, free, depth + 1);
                    }
                }
                return;
            }
        }
        let pr = self.tc.resolve(p);
        let ar = self.tc.resolve(a);
        match (&pr, &ar) {
            (Type::Array(pe), Type::Array(ae)) => self.unify(pe, ae, bind, free, depth + 1),
            (Type::Object(pf), Type::Object(af)) => {
                for (n, ft) in pf {
                    if let Some(aft) = obj_get(af, n) {
                        self.unify(ft, aft, bind, free, depth + 1);
                    }
                }
            }
            (Type::Fn { params: pp, ret: pret, .. }, Type::Fn { params: ap, ret: aret, .. }) => {
                for (i, x) in pp.iter().enumerate() {
                    if let Some(y) = ap.get(i) {
                        self.unify(x, y, bind, free, depth + 1);
                    }
                }
                self.unify(pret, aret, bind, free, depth + 1);
            }
            (Type::Union(pm), _) => {
                if let Type::Union(am) = &ar {
                    for m in am {
                        self.unify(&pr, m, bind, free, depth + 1);
                    }
                    return;
                }
                let err_map: HashMap<String, Type> = free.iter().map(|n| (n.clone(), ERR)).collect();
                let concrete: Vec<&Type> =
                    pm.iter().filter(|m| !matches!(m, Type::Param(n) if free.contains(n))).collect();
                let matching: Vec<&&Type> =
                    concrete.iter().filter(|m| self.tc.is_assignable(&ar, &subst(m, &err_map))).collect();
                if !matching.is_empty() {
                    for m in matching {
                        self.unify(m, &ar, bind, free, depth + 1);
                    }
                } else if let Some(bare) = pm.iter().find(|m| matches!(m, Type::Param(n) if free.contains(n))) {
                    self.unify(bare, &ar, bind, free, depth + 1);
                }
            }
            _ => {}
        }
    }

    // ---------- métodos ----------

    fn method_type(&self, recv: &Type, name: &str) -> Option<(Vec<Type>, Type, Vec<String>)> {
        let flat = mk_union(self.tc.flatten(recv, false));
        let base = self.tc.resolve(&widen(&flat));
        let u = param("U");
        let a = param("A");
        let found: Option<Type> = match &base {
            Type::Array(elem) => {
                let e = (**elem).clone();
                let pred = t_fn(vec![e.clone()], BOOLEAN);
                match name {
                    "len" => Some(t_fn(vec![], NUMBER)),
                    "push" => Some(t_fn(vec![e], VOID)),
                    "pop" => Some(t_fn(vec![], e)),
                    "contains" => Some(t_fn(vec![e], BOOLEAN)),
                    "reverse" => Some(t_fn(vec![], arr(e))),
                    "join" => Some(t_fn(vec![STRING], STRING)),
                    "map" => Some(t_fn_gen(vec![t_fn(vec![e], u)], arr(param("U")), vec!["U".into()])),
                    "filter" => Some(t_fn(vec![pred], arr(e))),
                    "any" => Some(t_fn(vec![pred], BOOLEAN)),
                    "all" => Some(t_fn(vec![pred], BOOLEAN)),
                    "fold" => {
                        Some(t_fn_gen(vec![a.clone(), t_fn(vec![a, e], param("A"))], param("A"), vec!["A".into()]))
                    }
                    "sum" if self.tc.is_assignable(&e, &NUMBER) => Some(t_fn(vec![], NUMBER)),
                    "sort" if self.tc.is_assignable(&e, &NUMBER) || self.tc.is_assignable(&e, &STRING) => {
                        Some(t_fn(vec![], arr(e)))
                    }
                    "slice" => Some(t_fn(vec![NUMBER, NUMBER], arr(e))),
                    "index_of" => Some(t_fn(vec![e], NUMBER)),
                    "is_empty" => Some(t_fn(vec![], BOOLEAN)),
                    _ => None,
                }
            }
            Type::String => match name {
                "len" => Some(t_fn(vec![], NUMBER)),
                "upper" | "lower" | "trim" => Some(t_fn(vec![], STRING)),
                "contains" | "starts_with" | "ends_with" => Some(t_fn(vec![STRING], BOOLEAN)),
                "split" => Some(t_fn(vec![STRING], arr(STRING))),
                "chars" | "lines" => Some(t_fn(vec![], arr(STRING))),
                "repeat" => Some(t_fn(vec![NUMBER], STRING)),
                "replace" => Some(t_fn(vec![STRING, STRING], STRING)),
                "slice" => Some(t_fn(vec![NUMBER, NUMBER], STRING)),
                "index_of" => Some(t_fn(vec![STRING], NUMBER)),
                "is_empty" => Some(t_fn(vec![], BOOLEAN)),
                "to_number" => Some(t_fn(
                    vec![],
                    Type::Union(vec![
                        t_obj(vec![("ok", lit(LitVal::Bool(true))), ("valor", NUMBER)]),
                        t_obj(vec![("ok", lit(LitVal::Bool(false)))]),
                    ]),
                )),
                _ => None,
            },
            Type::Number => match name {
                "to_string" => Some(t_fn(vec![], STRING)),
                "abs" | "floor" | "ceil" | "round" | "sqrt" => Some(t_fn(vec![], NUMBER)),
                "pow" | "min" | "max" => Some(t_fn(vec![NUMBER], NUMBER)),
                "to_base" => Some(t_fn(vec![NUMBER], STRING)),
                _ => None,
            },
            Type::Boolean => match name {
                "to_string" => Some(t_fn(vec![], STRING)),
                _ => None,
            },
            Type::Range => match name {
                "to_array" => Some(t_fn(vec![], arr(NUMBER))),
                _ => None,
            },
            _ => None,
        };
        match found {
            Some(Type::Fn { params, ret, type_params, .. }) => Some((params, *ret, type_params)),
            _ => None,
        }
    }

    fn check_method_call(&mut self, object: &Expr, name: &str, args: &[Expr], span: Span) -> Type {
        let ot = self.check_expr(object, None);
        let members = self.tc.flatten(&ot, false);
        if members.iter().any(|m| matches!(m, Type::Err)) {
            for a in args {
                self.check_expr(a, None);
            }
            return ERR;
        }
        // Un campo que guarda una función: obj.callback(1)
        let all_have = members.iter().all(|m| matches!(m, Type::Object(f) if obj_get(f, name).is_some()));
        if all_have {
            let types: Vec<Type> = members
                .iter()
                .map(|m| if let Type::Object(f) = m { obj_get(f, name).cloned().unwrap_or(ERR) } else { ERR })
                .collect();
            let ft = self.tc.resolve(&mk_union(types));
            if let Type::Fn { params, ret, type_params, variadic } = ft {
                return self.apply_fn(&params, &ret, &type_params, variadic, &[], args, span, name);
            }
        }
        let Some((params, ret, type_params)) = self.method_type(&ot, name) else {
            self.error(format!("el tipo `{}` no tiene el método `{}`", print(&ot), name), span, None);
            for a in args {
                self.check_expr(a, None);
            }
            return ERR;
        };
        if MUTATING_METHODS.contains(&name) {
            self.check_mutable(object, span);
        }
        self.apply_fn(&params, &ret, &type_params, false, &[], args, span, &format!(".{}", name))
    }

    // ---------- closures ----------

    fn check_closure(&mut self, params: &[ParamNode], body: &Expr, expected: Option<&Type>) -> Type {
        let exp = expected.map(|t| self.tc.resolve(t));
        let exp_fn = match &exp {
            Some(Type::Fn { params, ret, .. }) => Some((params.clone(), (**ret).clone())),
            _ => None,
        };
        let usable = |t: Option<&Type>| -> Option<Type> { t.filter(|t| !contains_param(t, &is_fresh)).cloned() };
        let param_types: Vec<Type> = params
            .iter()
            .enumerate()
            .map(|(i, p)| {
                if let Some(ty) = &p.ty {
                    return self.lower_type(ty);
                }
                if let Some(t) = usable(exp_fn.as_ref().and_then(|(ps, _)| ps.get(i))) {
                    return t;
                }
                self.error(
                    format!("no se puede inferir el tipo del parámetro `{}`", p.name),
                    p.span,
                    Some(format!("anotalo: `|{}: number| ...`", p.name)),
                );
                ERR
            })
            .collect();
        let expected_ret = usable(exp_fn.as_ref().map(|(_, r)| r));
        self.scopes.push(HashMap::new());
        for (i, p) in params.iter().enumerate() {
            self.define(&p.name, VarInfo { ty: param_types[i].clone(), mutable: false, kind: VarKind::Var });
        }
        self.fn_stack.push(FnCtx { ret: expected_ret.clone(), inferred: Vec::new() });
        self.loop_stack.push(None);
        let body_t = self.check_expr(body, expected_ret.as_ref());
        self.loop_stack.pop();
        let ctx = self.fn_stack.pop().unwrap();
        self.scopes.pop();
        let ret = match expected_ret {
            Some(er) => {
                if !matches!(er, Type::Void) {
                    self.expect_assignable(&body_t, &er, body.span(), None);
                }
                er
            }
            None => {
                let mut all = vec![body_t];
                all.extend(ctx.inferred);
                mk_union(all)
            }
        };
        t_fn(param_types, ret)
    }

    // ======================= match =======================

    fn check_match(&mut self, subject: &Expr, arms: &[Arm], _span: Span, expected: Option<&Type>) -> Type {
        let st = self.check_expr(subject, None);
        let mut remaining = self.tc.flatten(&st, true);
        let mut results: Vec<Type> = Vec::new();
        for arm in arms {
            self.prep_pattern(&arm.pattern);
            if remaining.is_empty() {
                self.warn(
                    "este brazo nunca se ejecuta: los anteriores ya cubren todos los casos",
                    arm.pattern.span(),
                    None,
                );
            }
            self.scopes.push(HashMap::new());
            self.bind_pattern(&arm.pattern, &st);
            if let Some(guard) = &arm.guard {
                let gt = self.check_expr(guard, None);
                self.expect_assignable(&gt, &BOOLEAN, guard.span(), None);
            }
            results.push(self.check_expr(&arm.body, expected));
            self.scopes.pop();
            if arm.guard.is_none() {
                remaining = remaining.iter().flat_map(|m| self.subtract(&arm.pattern, m)).collect();
            }
        }
        if !remaining.is_empty() && !remaining.iter().any(|m| matches!(m, Type::Err)) {
            let missing: Vec<String> = remaining.iter().map(|m| format!("`{}`", print(m))).collect();
            self.error(
                format!("`match` no exhaustivo: falta cubrir {}", missing.join(", ")),
                subject.span(),
                Some("agregá un brazo para esos casos, o uno comodín: `_ => ...`".into()),
            );
        }
        mk_union(results)
    }

    fn prep_pattern(&mut self, p: &Pattern) {
        match p {
            Pattern::Wild { id, ty: Some(ty), .. } | Pattern::Bind { id, ty: Some(ty), .. } => {
                let t = self.lower_type(ty);
                self.pattern_types.insert(*id, t);
            }
            Pattern::Object { fields, .. } => {
                for (_, f) in fields {
                    self.prep_pattern(f);
                }
            }
            _ => {}
        }
    }

    fn pat_type(&self, p: &Pattern) -> Option<Type> {
        match p {
            Pattern::Wild { id, .. } | Pattern::Bind { id, .. } => self.pattern_types.get(id).cloned(),
            _ => None,
        }
    }

    /// ¿El patrón puede coincidir con algún valor de `m`? (m ya está resuelto y no es unión)
    fn might_match(&self, p: &Pattern, m: &Type) -> bool {
        if matches!(m, Type::Err) {
            return true;
        }
        match p {
            Pattern::Wild { .. } | Pattern::Bind { .. } => {
                let Some(t) = self.pat_type(p) else { return true };
                self.tc.flatten(&t, true).iter().any(|x| self.tc.is_assignable(m, x) || self.tc.is_assignable(x, m))
            }
            Pattern::Lit { value, .. } => match m {
                Type::Lit(v) => v == value,
                Type::Number => matches!(value, LitVal::Num(_)),
                Type::String => matches!(value, LitVal::Str(_)),
                Type::Boolean => matches!(value, LitVal::Bool(_)),
                _ => false,
            },
            Pattern::Object { fields, .. } => {
                let Type::Object(mf) = m else { return false };
                fields.iter().all(|(name, fp)| {
                    obj_get(mf, name)
                        .is_some_and(|ft| self.tc.flatten(ft, true).iter().any(|x| self.might_match(fp, x)))
                })
            }
        }
    }

    /// ¿El patrón coincide con TODOS los valores de `m`?
    fn covers(&self, p: &Pattern, m: &Type) -> bool {
        if matches!(m, Type::Err) {
            return true;
        }
        match p {
            Pattern::Wild { .. } | Pattern::Bind { .. } => {
                let Some(t) = self.pat_type(p) else { return true };
                self.tc.is_assignable(m, &t)
            }
            Pattern::Lit { value, .. } => matches!(m, Type::Lit(v) if v == value),
            Pattern::Object { fields, .. } => {
                let Type::Object(mf) = m else { return false };
                fields.iter().all(|(name, fp)| {
                    obj_get(mf, name).is_some_and(|ft| self.tc.flatten(ft, true).iter().all(|x| self.covers(fp, x)))
                })
            }
        }
    }

    /// Lo que queda de `m` después de quitar lo que cubre `p` (para el chequeo de exhaustividad).
    fn subtract(&self, p: &Pattern, m: &Type) -> Vec<Type> {
        if self.covers(p, m) {
            return vec![];
        }
        let (Pattern::Object { fields: pf, .. }, Type::Object(mf)) = (p, m) else {
            return vec![m.clone()];
        };
        if !self.might_match(p, m) {
            return vec![m.clone()];
        }
        let with_field = |name: &str, t: Type| -> Type {
            let mut fields = mf.clone();
            if let Some(entry) = fields.iter_mut().find(|(n, _)| n == name) {
                entry.1 = t;
            } else {
                fields.push((name.to_string(), t));
            }
            Type::Object(fields)
        };
        // Partimos el objeto por cada campo que sea una unión
        for (name, _) in pf {
            let Some(ft) = obj_get(mf, name) else { continue };
            let parts = self.tc.flatten(ft, true);
            if parts.len() > 1 {
                return parts.into_iter().flat_map(|x| self.subtract(p, &with_field(name, x))).collect();
            }
        }
        let uncovered: Vec<&(String, Pattern)> = pf
            .iter()
            .filter(|(name, fp)| obj_get(mf, name).is_some_and(|ft| !self.covers(fp, &self.tc.resolve(ft))))
            .collect();
        if uncovered.len() == 1 {
            let (name, fp) = uncovered[0];
            let ft = self.tc.resolve(obj_get(mf, name).unwrap());
            return self.subtract(fp, &ft).into_iter().map(|x| with_field(name, x)).collect();
        }
        vec![m.clone()]
    }

    /// Declara las variables del patrón con el tipo más preciso posible (narrowing).
    fn bind_pattern(&mut self, p: &Pattern, t: &Type) {
        let members: Vec<Type> = self.tc.flatten(t, true).into_iter().filter(|m| self.might_match(p, m)).collect();
        if members.is_empty() {
            self.error(format!("este patrón nunca coincide con un valor de tipo `{}`", print(t)), p.span(), None);
        }
        match p {
            Pattern::Wild { .. } | Pattern::Lit { .. } => {}
            Pattern::Bind { name, .. } => {
                let ty = self.pat_type(p).unwrap_or_else(|| t.clone());
                self.define(name, VarInfo { ty, mutable: false, kind: VarKind::Var });
            }
            Pattern::Object { fields, .. } => {
                for (name, fp) in fields {
                    let field_types: Vec<Type> = members
                        .iter()
                        .filter_map(|m| if let Type::Object(mf) = m { obj_get(mf, name).cloned() } else { None })
                        .collect();
                    let ft = if field_types.is_empty() { ERR } else { mk_union(field_types) };
                    self.bind_pattern(fp, &ft);
                }
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
