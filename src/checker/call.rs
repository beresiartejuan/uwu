//! Llamadas a funciones y métodos, inferencia de genéricos y closures.

use super::*;

impl Checker {
    pub(super) fn check_call(&mut self, callee: &Expr, type_args: &[TypeNode], args: &[Expr], span: Span) -> Type {
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

    pub(super) fn method_type(&self, recv: &Type, name: &str) -> Option<(Vec<Type>, Type, Vec<String>)> {
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
                    "enumerate" => Some(t_fn(vec![], arr(Type::Tuple(vec![NUMBER, e])))),
                    "zip" => Some(t_fn_gen(vec![arr(u)], arr(Type::Tuple(vec![e, param("U")])), vec!["U".into()])),
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

    pub(super) fn check_method_call(&mut self, object: &Expr, name: &str, args: &[Expr], span: Span) -> Type {
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

    pub(super) fn check_closure(&mut self, params: &[ParamNode], body: &Expr, expected: Option<&Type>) -> Type {
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
}
