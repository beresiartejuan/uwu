//! `match`: estrechamiento de tipos, exhaustividad y patrones de `let` y `for`.

use super::*;

impl Checker {
    pub(super) fn check_match(&mut self, subject: &Expr, arms: &[Arm], _span: Span, expected: Option<&Type>) -> Type {
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

    pub(super) fn prep_pattern(&mut self, p: &Pattern) {
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
            Pattern::Tuple { items, .. } => {
                for p in items {
                    self.prep_pattern(p);
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
    pub(super) fn might_match(&self, p: &Pattern, m: &Type) -> bool {
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
            Pattern::Tuple { items, .. } => {
                let Type::Tuple(ts) = m else { return false };
                ts.len() == items.len()
                    && items
                        .iter()
                        .zip(ts)
                        .all(|(ip, t)| self.tc.flatten(t, true).iter().any(|x| self.might_match(ip, x)))
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
            Pattern::Tuple { items, .. } => {
                let Type::Tuple(ts) = m else { return false };
                ts.len() == items.len()
                    && items.iter().zip(ts).all(|(ip, t)| self.tc.flatten(t, true).iter().all(|x| self.covers(ip, x)))
            }
        }
    }

    /// Lo que queda de `m` después de quitar lo que cubre `p` (para el chequeo de exhaustividad).
    pub(super) fn subtract(&self, p: &Pattern, m: &Type) -> Vec<Type> {
        if self.covers(p, m) {
            return vec![];
        }
        if !self.might_match(p, m) {
            return vec![m.clone()];
        }
        // Las tuplas se tratan como objetos con campos "0", "1", ...
        let indexed = |n: usize| (0..n).map(|i| i.to_string());
        type Split<'a> = (Vec<(String, &'a Pattern)>, Vec<(String, Type)>, bool);
        let (pf, mf, is_tuple): Split = match (p, m) {
            (Pattern::Object { fields, .. }, Type::Object(mf)) => {
                (fields.iter().map(|(n, fp)| (n.clone(), fp)).collect(), mf.clone(), false)
            }
            (Pattern::Tuple { items, .. }, Type::Tuple(ts)) => {
                (indexed(items.len()).zip(items).collect(), indexed(ts.len()).zip(ts.iter().cloned()).collect(), true)
            }
            _ => return vec![m.clone()],
        };
        let mf = &mf;
        let with_field = |name: &str, t: Type| -> Type {
            let mut fields = mf.clone();
            if let Some(entry) = fields.iter_mut().find(|(n, _)| n == name) {
                entry.1 = t;
            } else {
                fields.push((name.to_string(), t));
            }
            if is_tuple {
                Type::Tuple(fields.into_iter().map(|(_, t)| t).collect())
            } else {
                Type::Object(fields)
            }
        };
        // Partimos el objeto por cada campo que sea una unión
        for (name, _) in &pf {
            let Some(ft) = obj_get(mf, name) else { continue };
            let parts = self.tc.flatten(ft, true);
            if parts.len() > 1 {
                return parts.into_iter().flat_map(|x| self.subtract(p, &with_field(name, x))).collect();
            }
        }
        let uncovered: Vec<&(String, &Pattern)> = pf
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
    pub(super) fn bind_pattern(&mut self, p: &Pattern, t: &Type) {
        let members: Vec<Type> = self.tc.flatten(t, true).into_iter().filter(|m| self.might_match(p, m)).collect();
        if members.is_empty() {
            let msg = match (p, self.tc.resolve(t)) {
                (Pattern::Tuple { items, .. }, Type::Tuple(ts)) if ts.len() != items.len() => format!(
                    "el patrón tiene {} elemento(s), pero la tupla `{}` tiene {}",
                    items.len(),
                    print(t),
                    ts.len()
                ),
                (Pattern::Object { fields, .. }, Type::Object(tf)) => {
                    match fields.iter().find(|(n, _)| obj_get(&tf, n).is_none()) {
                        Some((n, _)) => format!("el tipo `{}` no tiene el campo `{}`", print(t), n),
                        None => format!("este patrón nunca coincide con un valor de tipo `{}`", print(t)),
                    }
                }
                _ => format!("este patrón nunca coincide con un valor de tipo `{}`", print(t)),
            };
            self.error(msg, p.span(), None);
        }
        match p {
            Pattern::Wild { .. } | Pattern::Lit { .. } => {}
            Pattern::Bind { name, mutable, .. } => {
                let ty = match self.pat_type(p) {
                    Some(ty) => ty,
                    None if *mutable => widen(t),
                    None => t.clone(),
                };
                self.define(name, VarInfo { ty, mutable: *mutable, kind: VarKind::Var });
            }
            Pattern::Tuple { items, .. } => {
                for (i, ip) in items.iter().enumerate() {
                    let item_types: Vec<Type> = members
                        .iter()
                        .filter_map(|m| if let Type::Tuple(ts) = m { ts.get(i).cloned() } else { None })
                        .collect();
                    let it = if item_types.is_empty() { ERR } else { mk_union(item_types) };
                    self.bind_pattern(ip, &it);
                }
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
