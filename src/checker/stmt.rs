//! Chequeo de sentencias, bloques y declaraciones de funciones.

use super::*;

impl Checker {
    pub(super) fn check_stmt(&mut self, s: &Stmt) -> Type {
        match s {
            Stmt::Let { mutable, name, ty, init, .. } => {
                let ann = ty.as_ref().map(|t| self.lower_type(t));
                let init_t = self.check_expr(init, ann.as_ref());
                let var_t;
                if let Some(ann) = ann {
                    self.expect_assignable(&init_t, &ann, init.span(), None);
                    var_t = ann;
                } else {
                    // `let mut p = { ok: true }` tiene que poder cambiar `p.ok`; `let p` no.
                    // Si el valor ya existía (`let mut q = p`), `q` es otra referencia al mismo
                    // objeto y no puede tener un tipo más amplio que el original.
                    let fresh = !matches!(
                        init,
                        Expr::Ident { .. } | Expr::Member { .. } | Expr::Index { .. } | Expr::Path { .. }
                    );
                    let mut t = if *mutable && fresh { widen(&init_t) } else { widen_value(&init_t) };
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
            Stmt::Destructure { pattern, ty, init, .. } => {
                let ann = ty.as_ref().map(|t| self.lower_type(t));
                let init_t = self.check_expr(init, ann.as_ref());
                let t = match ann {
                    Some(ann) => {
                        self.expect_assignable(&init_t, &ann, init.span(), None);
                        ann
                    }
                    None if has_never_array(&widen(&init_t)) => {
                        self.error(
                            "no se puede inferir el tipo de las variables",
                            init.span(),
                            Some(
                                "agregá una anotación después del patrón, ej: `let (a, b): (number[], string) = ...;`"
                                    .into(),
                            ),
                        );
                        ERR
                    }
                    None => widen_value(&init_t),
                };
                self.prep_pattern(pattern);
                self.bind_irrefutable(pattern, &t, "`let`");
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
            Stmt::Namespace(n) => {
                if self.user_ns.contains_key(&n.name) {
                    let view = self.enter_ns(&n.name);
                    for s in &n.stmts {
                        self.check_stmt(s);
                    }
                    self.leave_ns(view);
                }
                VOID
            }
            Stmt::Using { ns, name, span } => {
                if let Some(info) = self.user_ns.get(ns).cloned() {
                    self.using_user_ns(ns, info, name.as_deref(), *span);
                    return VOID;
                }
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

    /// `using namespace geo;` / `using geo::area;` con un namespace declarado en uwu.
    fn using_user_ns(&mut self, ns: &str, info: NsInfo, name: Option<&str>, span: Span) {
        // lo traído con `using` es un alias: no se puede reasignar
        let alias = |v: &VarInfo| VarInfo { kind: VarKind::Imported, mutable: false, ..v.clone() };
        let mut found = false;
        for (n, v) in &info.values {
            if name.is_none_or(|x| x == n) {
                self.define(n, alias(v));
                found = true;
            }
        }
        for (n, key) in &info.types {
            if name.is_none_or(|x| x == n) {
                self.type_scope.insert(n.clone(), key.clone());
                found = true;
            }
        }
        if let (Some(n), false) = (name, found) {
            self.ns_member_error(ns, n, &info, span);
        }
    }

    pub(super) fn ns_member_error(&mut self, ns: &str, name: &str, info: &NsInfo, span: Span) {
        if info.lets.iter().any(|l| l == name) {
            self.error(
                format!("`{}::{}` se usa antes de declararse", ns, name),
                span,
                Some(format!("mové el `namespace {}` más arriba, antes de este uso", ns)),
            );
            return;
        }
        let mut members: Vec<&String> = info.values.keys().chain(info.types.iter().map(|(n, _)| n)).collect();
        members.sort();
        let hint = match members.iter().find(|n| levenshtein(n, name) <= 2) {
            Some(s) => format!("¿quisiste decir `{}::{}`?", ns, s),
            None if members.is_empty() => format!("el namespace `{}` está vacío", ns),
            None => {
                format!("`{}` tiene: {}", ns, members.iter().map(|n| format!("`{}`", n)).collect::<Vec<_>>().join(", "))
            }
        };
        self.error(format!("el namespace `{}` no tiene `{}`", ns, name), span, Some(hint));
    }

    /// Para `let` y `for`: el patrón tiene que coincidir siempre.
    pub(super) fn bind_irrefutable(&mut self, p: &Pattern, t: &Type, what: &str) {
        let flat = self.tc.flatten(t, true);
        if flat.iter().any(|m| self.might_match(p, m)) {
            let remaining: Vec<Type> = flat.iter().flat_map(|m| self.subtract(p, m)).collect();
            if !remaining.is_empty() && !remaining.iter().any(|m| matches!(m, Type::Err)) {
                let missing: Vec<String> = remaining.iter().map(|m| format!("`{}`", print(m))).collect();
                self.error(
                    format!("el patrón de {} no cubre todos los valores: falta {}", what, missing.join(", ")),
                    p.span(),
                    Some("para valores que pueden no coincidir con el patrón usá `match`".into()),
                );
            }
        }
        self.bind_pattern(p, t);
    }

    /// ¿Está importada la biblioteca `ns`? Si no, explica cómo importarla.
    pub(super) fn require_namespace(&mut self, ns: &str, span: Span) -> bool {
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

    pub(super) fn namespace_member(&mut self, ns: &str, name: &str, span: Span) -> Option<Type> {
        if let Some(m) = stdlib::lookup(ns, name) {
            return Some(stdlib::member_type(&m));
        }
        if let Some(new) = stdlib::renamed(ns, name) {
            self.error(
                format!("la biblioteca `{}` no tiene `{}`", ns, name),
                span,
                Some(format!("ahora se llama `{}::{}`", ns, new)),
            );
            return None;
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

    pub(super) fn check_block(&mut self, b: &Block, expected: Option<&Type>) -> Type {
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
}
