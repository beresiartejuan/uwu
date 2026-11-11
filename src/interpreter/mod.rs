//! El intérprete: recorre el AST ya chequeado y lo ejecuta.

use crate::diagnostics::RuntimeError;
use crate::modules::{Dep, Module};
use crate::stdlib::{self, GLOBALS};
use crate::syntax::ast::*;
use crate::syntax::Span;
use crate::types::{fmt_num, Type, TypeCtx};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

mod method;
mod native;
mod value;

pub use value::*;

const MAX_DEPTH: usize = 10_000;

pub struct Interpreter<'p, 'o> {
    out: &'o mut dyn FnMut(&str),
    tc: TypeCtx,
    pattern_types: HashMap<usize, Type>,
    script_args: &'p [String],
    depth: usize,
    /// Estado del generador de `math::random` (xorshift64*).
    rng: u64,
    /// Para `time::elapsed()`.
    start: std::time::Instant,
    /// Procesos de `proc::spawn`, para poder hacer `proc::wait(pid)`.
    children: HashMap<u32, std::process::Child>,
}

impl<'p, 'o> Interpreter<'p, 'o> {
    pub fn new(
        out: &'o mut dyn FnMut(&str),
        tc: TypeCtx,
        pattern_types: HashMap<usize, Type>,
        script_args: &'p [String],
    ) -> Self {
        Self {
            out,
            tc,
            pattern_types,
            script_args,
            depth: 0,
            rng: native::initial_seed(),
            start: std::time::Instant::now(),
            children: HashMap::new(),
        }
    }

    /// Ejecuta los módulos en el orden de `modules::load`: cada uno ve lo que
    /// exportan sus dependencias. Al final corre `main` del archivo de entrada.
    pub fn run(&mut self, modules: &'p [Module]) -> Result<(), RuntimeError> {
        let globals = Self::global_env();
        let env = Env::new(Some(Rc::clone(&globals)));
        self.run_modules(modules, &globals, &env, true).map(|_| ())
    }

    /// El entorno con las funciones del prelude (`print`, `panic`, `assert`).
    pub fn global_env() -> Rc<Env<'p>> {
        let globals = Env::new(None);
        for (name, b) in GLOBALS {
            globals.define(name, Value::Builtin(*b));
        }
        globals
    }

    /// REPL: los tipos cambian con cada entrada.
    pub fn set_types(&mut self, tc: TypeCtx, pattern_types: HashMap<usize, Type>) {
        self.tc = tc;
        self.pattern_types = pattern_types;
    }

    /// REPL: ejecuta una entrada en `env`, que se conserva entre entradas.
    /// Devuelve el valor de la última expresión, si la entrada termina en una.
    pub fn run_repl(
        &mut self,
        modules: &'p [Module],
        globals: &Rc<Env<'p>>,
        env: &Rc<Env<'p>>,
    ) -> Result<Option<String>, RuntimeError> {
        let v = self.run_modules(modules, globals, env, false)?;
        Ok(v.filter(|v| !matches!(v, Value::Unit)).map(|v| display(&v, false)))
    }

    fn run_modules(
        &mut self,
        modules: &'p [Module],
        globals: &Rc<Env<'p>>,
        entry_env: &Rc<Env<'p>>,
        call_main: bool,
    ) -> Result<Option<Value<'p>>, RuntimeError> {
        let result = (|| -> Result<Option<Value<'p>>, Control<'p>> {
            let mut exports: Vec<Vec<(String, Value<'p>)>> = Vec::with_capacity(modules.len());
            let mut last = None;
            for (i, m) in modules.iter().enumerate() {
                let is_entry = i + 1 == modules.len();
                let env = if is_entry { Rc::clone(entry_env) } else { Env::new(Some(Rc::clone(globals))) };
                for d in &m.deps {
                    if let Dep::File(j) = d.dep {
                        for (name, v) in &exports[j] {
                            env.define(name, v.clone());
                        }
                    }
                }
                let stmts = &m.program.stmts;
                self.hoist(stmts, &env);
                for s in stmts {
                    match s {
                        Stmt::Expr { expr, semi: false, .. } if is_entry => last = Some(self.eval(expr, &env)?),
                        s => {
                            last = None;
                            self.exec_stmt(s, &env)?;
                        }
                    }
                }

                let mut ex: Vec<(String, Value<'p>)> = Vec::new();
                for s in stmts {
                    let (names, span): (Vec<String>, Span) = match s {
                        Stmt::Let { exported: true, name, span, .. } => (vec![name.clone()], *span),
                        Stmt::Destructure { exported: true, pattern, span, .. } => {
                            (pattern.names().into_iter().map(String::from).collect(), *span)
                        }
                        Stmt::Fn(d) if d.exported => (vec![d.name.clone()], d.span),
                        Stmt::Namespace(n) if n.exported => (vec![ns_key(&n.name)], n.span),
                        _ => continue,
                    };
                    for name in names {
                        let v = env.get(&name, span)?;
                        ex.retain(|(n, _)| *n != name);
                        ex.push((name, v));
                    }
                }
                for d in m.deps.iter().filter(|d| d.reexport) {
                    if let Dep::File(j) = d.dep {
                        ex.extend(exports[j].iter().cloned());
                    }
                }
                exports.push(ex);

                if is_entry && call_main {
                    let main = stmts.iter().find_map(|s| match s {
                        Stmt::Fn(d) if d.name == "main" => Some(d),
                        _ => None,
                    });
                    if let Some(d) = main {
                        let f = env.get("main", d.span)?;
                        self.call_value(&f, vec![], d.span)?;
                    }
                }
            }
            Ok(last)
        })();
        match result {
            Ok(v) => Ok(v),
            Err(Control::Err(e)) => Err(e),
            Err(Control::Return(_)) | Err(Control::Break) | Err(Control::Continue) => Ok(None),
        }
    }

    fn hoist(&self, stmts: &'p [Stmt], env: &Rc<Env<'p>>) {
        for s in stmts {
            if let Stmt::Fn(d) = s {
                let closure = ClosureV {
                    name: d.name.clone(),
                    params: &d.params,
                    body: FnBody::Block(&d.body),
                    env: Rc::clone(env),
                };
                env.define(&d.name, Value::Closure(Rc::new(closure)));
            }
            if let Stmt::Namespace(n) = s {
                // sus funciones ven a las demás sin prefijo, y al resto del archivo
                let ns_env = Env::new(Some(Rc::clone(env)));
                self.hoist(&n.stmts, &ns_env);
                env.define(&ns_key(&n.name), Value::Namespace(ns_env));
            }
        }
    }

    fn exec_stmt(&mut self, s: &'p Stmt, env: &Rc<Env<'p>>) -> Result<(), Control<'p>> {
        match s {
            Stmt::Let { name, init, .. } => {
                let v = self.eval(init, env)?;
                env.define(name, v);
                Ok(())
            }
            Stmt::Destructure { pattern, init, span, .. } => {
                let v = self.eval(init, env)?;
                self.bind(pattern, &v, env, *span)
            }
            Stmt::Namespace(n) => {
                if let Some(Value::Namespace(ns_env)) = env.own(&ns_key(&n.name)) {
                    for s in &n.stmts {
                        self.exec_stmt(s, &ns_env)?;
                    }
                }
                Ok(())
            }
            Stmt::Expr { expr, .. } => {
                self.eval(expr, env)?;
                Ok(())
            }
            Stmt::Using { ns, name, .. } => {
                if let Some(Value::Namespace(ns_env)) = env.lookup(&ns_key(ns)) {
                    let members: Vec<(String, Value<'p>)> =
                        ns_env.vars.borrow().iter().map(|(n, v)| (n.clone(), v.clone())).collect();
                    for (n, v) in members {
                        if name.as_deref().is_none_or(|x| x == n) {
                            env.define(&n, v);
                        }
                    }
                    return Ok(());
                }
                for (n, m) in stdlib::module(ns).unwrap_or_default() {
                    if name.as_deref().is_none_or(|x| x == *n) {
                        env.define(n, member_value(*m));
                    }
                }
                Ok(())
            }
            _ => Ok(()), // fn y type ya se procesaron
        }
    }

    /// `let` / `for` con un patrón: el checker ya garantizó que coincide.
    fn bind(&mut self, p: &Pattern, v: &Value<'p>, env: &Rc<Env<'p>>, span: Span) -> Result<(), Control<'p>> {
        if self.match_pattern(p, v, env) {
            Ok(())
        } else {
            Err(rt(format!("el valor {} no coincide con el patrón", display(v, false)), span))
        }
    }

    fn eval_block(&mut self, b: &'p Block, env: &Rc<Env<'p>>) -> EResult<'p> {
        self.hoist(&b.stmts, env);
        for s in &b.stmts {
            self.exec_stmt(s, env)?;
        }
        match &b.tail {
            Some(tail) => self.eval(tail, env),
            None => Ok(Value::Unit),
        }
    }

    pub fn call_value(&mut self, f: &Value<'p>, args: Vec<Value<'p>>, span: Span) -> EResult<'p> {
        if let Value::Builtin(b) = f {
            return self.call_builtin(*b, args, span);
        }
        let Value::Closure(c) = f else {
            return Err(rt("se intentó llamar a algo que no es una función", span));
        };
        if self.depth >= MAX_DEPTH {
            return Err(rt("desbordamiento de pila (¿recursión infinita?)", span));
        }
        let env = Env::new(Some(Rc::clone(&c.env)));
        for (i, p) in c.params.iter().enumerate() {
            let v = args.get(i).cloned().unwrap_or(Value::Unit);
            match &p.pattern {
                Some(pat) => self.bind(pat, &v, &env, span)?,
                None => env.define(&p.name, v),
            }
        }
        self.depth += 1;
        let result = match c.body {
            FnBody::Block(b) => self.eval_block(b, &env),
            FnBody::Expr(e) => self.eval(e, &env),
        };
        self.depth -= 1;
        match result {
            Err(Control::Return(v)) => Ok(v),
            other => other,
        }
    }

    pub(crate) fn num(&self, v: &Value<'p>, span: Span) -> Result<f64, Control<'p>> {
        match v {
            Value::Num(n) => Ok(*n),
            _ => Err(rt("se esperaba un número", span)),
        }
    }

    fn truthy(&self, v: &Value<'p>) -> bool {
        match v {
            Value::Bool(b) => *b,
            Value::Unit => false,
            Value::Num(n) => *n != 0.0,
            Value::Str(s) => !s.is_empty(),
            _ => true,
        }
    }

    pub fn eval(&mut self, e: &'p Expr, env: &Rc<Env<'p>>) -> EResult<'p> {
        match e {
            Expr::Num { value, .. } => Ok(Value::Num(*value)),
            Expr::Str { value, .. } => Ok(vstr(value.clone())),
            Expr::Template { parts, .. } => {
                let mut out = String::new();
                for part in parts {
                    match part {
                        StrPart::Text(t) => out.push_str(t),
                        StrPart::Expr(e) => out.push_str(&display(&self.eval(e, env)?, true)),
                    }
                }
                Ok(vstr(out))
            }
            Expr::Bool { value, .. } => Ok(Value::Bool(*value)),
            Expr::Ident { name, span } => env.get(name, *span),
            Expr::Path { ns, name, span } => {
                if let Some(Value::Namespace(ns_env)) = env.lookup(&ns_key(ns)) {
                    return ns_env.own(name).ok_or_else(|| rt(format!("`{}::{}` no existe", ns, name), *span));
                }
                match stdlib::lookup(ns, name) {
                    Some(m) => Ok(member_value(m)),
                    None => Err(rt(format!("`{}::{}` no existe", ns, name), *span)),
                }
            }
            Expr::Array { items, .. } => {
                let mut out = Vec::with_capacity(items.len());
                for i in items {
                    out.push(self.eval(i, env)?);
                }
                Ok(varr(out))
            }
            Expr::Tuple { items, .. } => {
                let mut out = Vec::with_capacity(items.len());
                for i in items {
                    out.push(self.eval(i, env)?);
                }
                Ok(vtuple(out))
            }
            Expr::Object { fields, .. } => {
                let mut out: Vec<(String, Value<'p>)> = Vec::with_capacity(fields.len());
                for f in fields {
                    let v = self.eval(&f.value, env)?;
                    obj_set(&mut out, &f.name, v);
                }
                Ok(Value::Object(Rc::new(RefCell::new(out))))
            }
            Expr::Unary { op, expr, span } => {
                let v = self.eval(expr, env)?;
                if op == "-" {
                    Ok(Value::Num(-self.num(&v, *span)?))
                } else {
                    Ok(Value::Bool(!self.truthy(&v)))
                }
            }
            Expr::Binary { op, left, right, span } => self.eval_binary(op, left, right, *span, env),
            Expr::Range { start, end, span } => {
                let s = self.eval(start, env)?;
                let e2 = self.eval(end, env)?;
                Ok(Value::Range(self.num(&s, *span)?, self.num(&e2, *span)?))
            }
            Expr::Assign { op, target, value, span } => {
                let mut v = self.eval(value, env)?;
                if op != "=" {
                    let cur = self.eval(target, env)?;
                    let arith_op = &op[..1];
                    v = self.arith(arith_op, &cur, &v, *span)?;
                }
                self.assign(target, v, env)?;
                Ok(Value::Unit)
            }
            Expr::Call { callee, args, span, .. } => self.eval_call(callee, args, *span, env),
            Expr::Member { object, name, span } => {
                let obj = self.eval(object, env)?;
                match &obj {
                    Value::Object(fields) => Ok(obj_field(&fields.borrow(), name).unwrap_or(Value::Unit)),
                    Value::Tuple(items) => name
                        .parse::<usize>()
                        .ok()
                        .and_then(|i| items.borrow().get(i).cloned())
                        .ok_or_else(|| rt(format!("la tupla no tiene el elemento `{}`", name), *span)),
                    _ => Err(rt(format!("el valor no tiene el campo `{}`", name), *span)),
                }
            }
            Expr::Index { object, index, .. } => {
                let obj = self.eval(object, env)?;
                let iv = self.eval(index, env)?;
                let i = self.num(&iv, index.span())?;
                match &obj {
                    Value::Str(s) => {
                        let chars: Vec<char> = s.chars().collect();
                        self.check_index(i, chars.len(), index.span())?;
                        Ok(vstr(chars[i as usize].to_string()))
                    }
                    Value::Array(items) => {
                        let items = items.borrow();
                        self.check_index(i, items.len(), index.span())?;
                        Ok(items[i as usize].clone())
                    }
                    _ => Err(rt("no se puede indexar este valor", index.span())),
                }
            }
            Expr::Block { block, .. } => {
                let inner = Env::new(Some(Rc::clone(env)));
                self.eval_block(block, &inner)
            }
            Expr::If { cond, then, else_, .. } => {
                let c = self.eval(cond, env)?;
                if self.truthy(&c) {
                    let inner = Env::new(Some(Rc::clone(env)));
                    self.eval_block(then, &inner)
                } else if let Some(else_) = else_ {
                    self.eval(else_, env)
                } else {
                    Ok(Value::Unit)
                }
            }
            Expr::While { cond, body, .. } => {
                loop {
                    let c = self.eval(cond, env)?;
                    if !self.truthy(&c) {
                        break;
                    }
                    if self.loop_body(body, env)? {
                        break;
                    }
                }
                Ok(Value::Unit)
            }
            Expr::Loop { body, .. } => {
                loop {
                    if self.loop_body(body, env)? {
                        break;
                    }
                }
                Ok(Value::Unit)
            }
            Expr::For { pattern, iter, body, .. } => {
                let it = self.eval(iter, env)?;
                let run = |this: &mut Self, v: Value<'p>| -> Result<bool, Control<'p>> {
                    let inner = Env::new(Some(Rc::clone(env)));
                    this.bind(pattern, &v, &inner, pattern.span())?;
                    this.loop_body(body, &inner)
                };
                match &it {
                    Value::Range(start, end) => {
                        let mut i = *start;
                        while i < *end {
                            if run(self, Value::Num(i))? {
                                break;
                            }
                            i += 1.0;
                        }
                    }
                    Value::Str(s) => {
                        for c in s.chars().collect::<Vec<char>>() {
                            if run(self, vstr(c.to_string()))? {
                                break;
                            }
                        }
                    }
                    Value::Array(items) => {
                        let snapshot: Vec<Value<'p>> = items.borrow().clone();
                        for v in snapshot {
                            if run(self, v)? {
                                break;
                            }
                        }
                    }
                    _ => return Err(rt("no se puede recorrer este valor", iter.span())),
                }
                Ok(Value::Unit)
            }
            Expr::Match { subject, arms, span } => {
                let v = self.eval(subject, env)?;
                for arm in arms {
                    let arm_env = Env::new(Some(Rc::clone(env)));
                    if self.match_pattern(&arm.pattern, &v, &arm_env) {
                        if let Some(guard) = &arm.guard {
                            let g = self.eval(guard, &arm_env)?;
                            if !self.truthy(&g) {
                                continue;
                            }
                        }
                        return self.eval(&arm.body, &arm_env);
                    }
                }
                Err(rt(format!("ningún brazo del `match` coincidió con {}", display(&v, false)), *span))
            }
            Expr::Closure { params, body, .. } => {
                let closure =
                    ClosureV { name: "closure".into(), params, body: FnBody::Expr(body), env: Rc::clone(env) };
                Ok(Value::Closure(Rc::new(closure)))
            }
            Expr::Return { value, .. } => {
                let v = match value {
                    Some(v) => self.eval(v, env)?,
                    None => Value::Unit,
                };
                Err(Control::Return(v))
            }
            Expr::Break { .. } => Err(Control::Break),
            Expr::Continue { .. } => Err(Control::Continue),
        }
    }

    /// Devuelve `true` si el cuerpo pidió `break`.
    fn loop_body(&mut self, body: &'p Block, env: &Rc<Env<'p>>) -> Result<bool, Control<'p>> {
        let inner = Env::new(Some(Rc::clone(env)));
        match self.eval_block(body, &inner) {
            Ok(_) => Ok(false),
            Err(Control::Break) => Ok(true),
            Err(Control::Continue) => Ok(false),
            Err(e) => Err(e),
        }
    }

    fn check_index(&self, i: f64, len: usize, span: Span) -> Result<(), Control<'p>> {
        if i.fract() != 0.0 || i < 0.0 || i >= len as f64 {
            return Err(rt(
                format!("índice fuera de rango: el largo es {} pero el índice es {}", len, fmt_num(i)),
                span,
            ));
        }
        Ok(())
    }

    fn assign(&mut self, target: &'p Expr, value: Value<'p>, env: &Rc<Env<'p>>) -> Result<(), Control<'p>> {
        match target {
            Expr::Ident { name, span } => env.set(name, value, *span),
            Expr::Path { ns, name, span } => match env.lookup(&ns_key(ns)) {
                Some(Value::Namespace(ns_env)) => {
                    ns_env.define(name, value);
                    Ok(())
                }
                _ => Err(rt(format!("no se puede asignar a `{}::{}`", ns, name), *span)),
            },
            Expr::Member { object, name, span } => {
                let obj = self.eval(object, env)?;
                match &obj {
                    Value::Object(fields) => obj_set(&mut fields.borrow_mut(), name, value),
                    Value::Tuple(items) => {
                        let i = name.parse::<usize>().ok().filter(|i| *i < items.borrow().len());
                        let Some(i) = i else {
                            return Err(rt(format!("la tupla no tiene el elemento `{}`", name), *span));
                        };
                        items.borrow_mut()[i] = value;
                    }
                    _ => {}
                }
                Ok(())
            }
            Expr::Index { object, index, .. } => {
                let obj = self.eval(object, env)?;
                let iv = self.eval(index, env)?;
                let i = self.num(&iv, index.span())?;
                if let Value::Array(items) = &obj {
                    let len = items.borrow().len();
                    self.check_index(i, len, index.span())?;
                    items.borrow_mut()[i as usize] = value;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn arith(&self, op: &str, l: &Value<'p>, r: &Value<'p>, span: Span) -> EResult<'p> {
        match op {
            "+" => {
                if matches!(l, Value::Str(_)) || matches!(r, Value::Str(_)) {
                    return Ok(vstr(format!("{}{}", display(l, true), display(r, true))));
                }
                Ok(Value::Num(self.num(l, span)? + self.num(r, span)?))
            }
            "-" => Ok(Value::Num(self.num(l, span)? - self.num(r, span)?)),
            "*" => Ok(Value::Num(self.num(l, span)? * self.num(r, span)?)),
            "/" => {
                let rv = self.num(r, span)?;
                if rv == 0.0 {
                    return Err(rt("división por cero", span));
                }
                Ok(Value::Num(self.num(l, span)? / rv))
            }
            "%" => {
                let rv = self.num(r, span)?;
                if rv == 0.0 {
                    return Err(rt("división por cero", span));
                }
                Ok(Value::Num(self.num(l, span)? % rv))
            }
            _ => Err(rt(format!("operador desconocido {}", op), span)),
        }
    }

    fn eval_binary(&mut self, op: &str, left: &'p Expr, right: &'p Expr, span: Span, env: &Rc<Env<'p>>) -> EResult<'p> {
        if op == "&&" {
            let l = self.eval(left, env)?;
            if !self.truthy(&l) {
                return Ok(Value::Bool(false));
            }
            let r = self.eval(right, env)?;
            return Ok(Value::Bool(self.truthy(&r)));
        }
        if op == "||" {
            let l = self.eval(left, env)?;
            if self.truthy(&l) {
                return Ok(Value::Bool(true));
            }
            let r = self.eval(right, env)?;
            return Ok(Value::Bool(self.truthy(&r)));
        }
        let l = self.eval(left, env)?;
        let r = self.eval(right, env)?;
        match op {
            "==" => Ok(Value::Bool(deep_equal(&l, &r))),
            "!=" => Ok(Value::Bool(!deep_equal(&l, &r))),
            "<" | ">" | "<=" | ">=" => {
                let result = match (&l, &r) {
                    (Value::Num(a), Value::Num(b)) => match op {
                        "<" => a < b,
                        ">" => a > b,
                        "<=" => a <= b,
                        _ => a >= b,
                    },
                    (Value::Str(a), Value::Str(b)) => match op {
                        "<" => a < b,
                        ">" => a > b,
                        "<=" => a <= b,
                        _ => a >= b,
                    },
                    _ => false,
                };
                Ok(Value::Bool(result))
            }
            _ => self.arith(op, &l, &r, span),
        }
    }

    fn eval_call(&mut self, callee: &'p Expr, args: &'p [Expr], span: Span, env: &Rc<Env<'p>>) -> EResult<'p> {
        if let Expr::Member { object, name, .. } = callee {
            let obj = self.eval(object, env)?;
            let mut arg_vals = Vec::with_capacity(args.len());
            for a in args {
                arg_vals.push(self.eval(a, env)?);
            }
            if let Value::Object(fields) = &obj {
                if let Some(f) = obj_field(&fields.borrow(), name) {
                    return self.call_value(&f, arg_vals, span);
                }
            }
            return self.call_method(&obj, name, arg_vals, span);
        }
        let f = self.eval(callee, env)?;
        let mut arg_vals = Vec::with_capacity(args.len());
        for a in args {
            arg_vals.push(self.eval(a, env)?);
        }
        self.call_value(&f, arg_vals, span)
    }

    // ---------- patrones ----------

    fn match_pattern(&self, p: &Pattern, v: &Value<'p>, env: &Rc<Env<'p>>) -> bool {
        match p {
            Pattern::Wild { id, .. } => match self.pattern_types.get(id) {
                Some(t) => self.value_is(v, t),
                None => true,
            },
            Pattern::Bind { id, name, .. } => {
                if let Some(t) = self.pattern_types.get(id) {
                    if !self.value_is(v, t) {
                        return false;
                    }
                }
                env.define(name, v.clone());
                true
            }
            Pattern::Lit { value, .. } => match (value, v) {
                (LitVal::Num(a), Value::Num(b)) => a == b,
                (LitVal::Str(a), Value::Str(b)) => a == &***b,
                (LitVal::Bool(a), Value::Bool(b)) => a == b,
                _ => false,
            },
            Pattern::Object { fields, .. } => {
                let Value::Object(vf) = v else { return false };
                fields.iter().all(|(name, fp)| {
                    let field = obj_field(&vf.borrow(), name);
                    field.is_some_and(|fv| self.match_pattern(fp, &fv, env))
                })
            }
            Pattern::Tuple { items, .. } => {
                let Value::Tuple(vs) = v else { return false };
                let vs = vs.borrow().clone();
                vs.len() == items.len() && items.iter().zip(&vs).all(|(ip, iv)| self.match_pattern(ip, iv, env))
            }
        }
    }

    /// Chequeo de tipo en ejecución, para patrones como `n: number`.
    fn value_is(&self, v: &Value<'p>, t: &Type) -> bool {
        let r = self.tc.resolve(t);
        match &r {
            Type::Number => matches!(v, Value::Num(_)),
            Type::String => matches!(v, Value::Str(_)),
            Type::Boolean => matches!(v, Value::Bool(_)),
            Type::Lit(lv) => match (lv, v) {
                (LitVal::Num(a), Value::Num(b)) => a == b,
                (LitVal::Str(a), Value::Str(b)) => a == &***b,
                (LitVal::Bool(a), Value::Bool(b)) => a == b,
                _ => false,
            },
            Type::Void => matches!(v, Value::Unit),
            Type::Range => matches!(v, Value::Range(_, _)),
            Type::Array(elem) => match v {
                Value::Array(items) => items.borrow().iter().all(|x| self.value_is(x, elem)),
                _ => false,
            },
            Type::Tuple(ts) => match v {
                Value::Tuple(vs) => {
                    let vs = vs.borrow();
                    vs.len() == ts.len() && vs.iter().zip(ts).all(|(x, t)| self.value_is(x, t))
                }
                _ => false,
            },
            Type::Object(fields) => match v {
                Value::Object(vf) => {
                    fields.iter().all(|(n, ft)| obj_field(&vf.borrow(), n).is_some_and(|fv| self.value_is(&fv, ft)))
                }
                _ => false,
            },
            Type::Union(members) => members.iter().any(|m| self.value_is(v, m)),
            Type::Fn { .. } => matches!(v, Value::Closure(_) | Value::Builtin(_)),
            Type::Never => false,
            _ => true, // parámetros de tipo: se borran en ejecución
        }
    }
}
