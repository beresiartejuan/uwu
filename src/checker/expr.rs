//! Chequeo de expresiones: literales, operadores, asignaciones y acceso a campos.

use super::*;

impl Checker {
    pub fn check_expr(&mut self, e: &Expr, expected: Option<&Type>) -> Type {
        match e {
            Expr::Num { value, .. } => lit(LitVal::Num(*value)),
            Expr::Str { value, .. } => lit(LitVal::Str(value.clone())),
            Expr::Template { parts, .. } => {
                for part in parts {
                    let StrPart::Expr(e) = part else { continue };
                    let t = self.check_expr(e, None);
                    match self.tc.resolve(&t) {
                        Type::Void => self.error(
                            "no se puede interpolar algo de tipo `void`",
                            e.span(),
                            Some("la expresión no produce ningún valor".into()),
                        ),
                        Type::Fn { .. } => self.error(
                            format!("no se puede interpolar una función (`{}`)", print(&t)),
                            e.span(),
                            Some("¿te faltó llamarla con `(...)`?".into()),
                        ),
                        _ => {}
                    }
                }
                STRING
            }
            Expr::Bool { value, .. } => lit(LitVal::Bool(*value)),
            Expr::Ident { name, span } => {
                let Some(v) = self.lookup(name) else {
                    if self.user_ns.contains_key(name) {
                        self.error(
                            format!("`{}` es un namespace, no un valor", name),
                            *span,
                            Some(format!("sus miembros se usan con `::`, ej: `{}::...`", name)),
                        );
                        return ERR;
                    }
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
                if let Some(info) = self.user_ns.get(ns) {
                    if let Some(v) = info.values.get(name) {
                        return v.ty.clone();
                    }
                    let info = info.clone();
                    self.ns_member_error(ns, name, &info, *span);
                    return ERR;
                }
                if !self.require_namespace(ns, *span) {
                    return ERR;
                }
                self.namespace_member(ns, name, *span).unwrap_or(ERR)
            }
            Expr::Array { items, .. } => self.check_array(items, expected),
            Expr::Tuple { items, .. } => {
                // Con una tupla esperada (o una unión de tuplas), cada elemento recibe su tipo
                let exp: Vec<Vec<Type>> = expected
                    .map(|t| {
                        self.tc
                            .flatten(t, false)
                            .into_iter()
                            .filter_map(|m| match m {
                                Type::Tuple(ts) if ts.len() == items.len() => Some(ts),
                                _ => None,
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let types = items
                    .iter()
                    .enumerate()
                    .map(|(i, item)| {
                        let item_exp = if exp.is_empty() {
                            None
                        } else {
                            Some(mk_union(exp.iter().map(|ts| ts[i].clone()).collect()))
                        };
                        self.check_expr(item, item_exp.as_ref())
                    })
                    .collect();
                Type::Tuple(types)
            }
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
            Expr::For { pattern, iter, body, .. } => {
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
                self.prep_pattern(pattern);
                self.bind_irrefutable(pattern, &elem, "`for`");
                for name in pattern.names() {
                    if let Some(v) = self.scopes.last_mut().unwrap().get_mut(name) {
                        v.kind = VarKind::LoopVar;
                    }
                }
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
        arr(widen_value(&mk_union(member_types)))
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
    fn root_var(&self, e: &Expr) -> Option<(String, VarInfo)> {
        match e {
            Expr::Ident { name, .. } => Some((name.clone(), self.lookup(name)?.clone())),
            Expr::Path { ns, name, .. } => {
                let info = match self.user_ns.get(ns) {
                    Some(info) => info.values.get(name)?.clone(),
                    None => VarInfo { ty: ERR, mutable: false, kind: VarKind::Builtin },
                };
                Some((format!("{}::{}", ns, name), info))
            }
            Expr::Member { object, .. } | Expr::Index { object, .. } => self.root_var(object),
            _ => None,
        }
    }

    /// Reporta un error y devuelve `false` si `target` no se puede modificar.
    pub(super) fn check_mutable(&mut self, target: &Expr, span: Span) -> bool {
        let Some((root, v)) = self.root_var(target) else { return true };
        if v.kind == VarKind::Builtin {
            self.error(format!("no se puede modificar `{}`: es parte de la biblioteca estándar", root), span, None);
        } else if v.kind == VarKind::Imported {
            self.error(
                format!("no se puede modificar `{}`: fue importado de otro módulo", root),
                span,
                Some("exportá una función que lo modifique, o hacé una copia local".into()),
            );
        } else if !matches!(v.kind, VarKind::Var | VarKind::LoopVar) {
            self.error(format!("no se puede modificar `{}`: es una función", root), span, None);
        } else if !v.mutable {
            let hint = match root.split_once("::") {
                Some((ns, name)) => format!("declarala con `let mut {}` dentro de `namespace {}`", name, ns),
                None if v.kind == VarKind::LoopVar => format!("declarala con `for let {} in ...`", root),
                None => format!("declarala con `let mut {}`", root),
            };
            self.error(format!("no se puede modificar `{}` porque es inmutable", root), span, Some(hint));
        } else {
            return true;
        }
        false
    }

    fn check_assign(&mut self, op: &str, target: &Expr, value: &Expr) -> Type {
        let tt = self.check_expr(target, None);
        let mutable = self.check_mutable(target, target.span());
        if op == "=" {
            let vt = self.check_expr(value, Some(&tt));
            // Si no se puede modificar, el error de tipos no agrega nada
            if mutable {
                // `let mut q = p;` con `p: { ok: true }`: `q.ok` no puede cambiar
                let hint = match (target, &tt) {
                    (Expr::Member { name, .. }, Type::Lit(_)) => Some(format!(
                        "`{}` tiene el tipo literal `{}`; si querés poder cambiarlo, anotá el tipo \
                         de la variable, ej: `let mut x: {{ {}: {} }} = ...`",
                        name,
                        print(&tt),
                        name,
                        print(&widen(&tt))
                    )),
                    _ => None,
                };
                self.expect_assignable(&vt, &tt, value.span(), hint);
            }
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
        if let Ok(i) = name.parse::<usize>() {
            if members.iter().all(|m| matches!(m, Type::Tuple(_))) {
                let mut types = Vec::new();
                for m in &members {
                    let Type::Tuple(ts) = m else { continue };
                    let Some(t) = ts.get(i) else {
                        self.error(
                            format!("la tupla `{}` no tiene el elemento `{}`", print(m), i),
                            span,
                            Some(format!(
                                "tiene {} elemento(s): del `.0` al `.{}`",
                                ts.len(),
                                ts.len().saturating_sub(1)
                            )),
                        );
                        return ERR;
                    };
                    types.push(t.clone());
                }
                return mk_union(types);
            }
            if members.iter().all(|m| matches!(m, Type::Array(_))) {
                self.error(
                    format!("los arreglos no tienen el campo `{}`", name),
                    span,
                    Some(format!("para acceder a un elemento usá `[{}]`", name)),
                );
                return ERR;
            }
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
}
