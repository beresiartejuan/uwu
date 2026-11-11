use crate::ast::*;
use crate::diagnostics::RuntimeError;
use crate::lexer::Span;
use crate::modules::{Dep, Module};
use crate::stdlib::{self, builtin_name, BuiltinFn, Member, GLOBALS};
use crate::types::{fmt_num, quote_str, Type, TypeCtx};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

const MAX_DEPTH: usize = 10_000;

// ======================= valores =======================

pub struct ClosureV<'p> {
    pub name: String,
    pub params: Vec<String>,
    pub body: FnBody<'p>,
    pub env: Rc<Env<'p>>,
}

#[derive(Clone, Copy)]
pub enum FnBody<'p> {
    Block(&'p Block),
    Expr(&'p Expr),
}

#[derive(Clone)]
pub enum Value<'p> {
    Num(f64),
    Str(Rc<String>),
    Bool(bool),
    Unit,
    Array(Rc<RefCell<Vec<Value<'p>>>>),
    Object(Rc<RefCell<Vec<(String, Value<'p>)>>>),
    Range(f64, f64),
    Closure(Rc<ClosureV<'p>>),
    Builtin(BuiltinFn),
}

pub(crate) fn vstr<'p>(s: impl Into<String>) -> Value<'p> {
    Value::Str(Rc::new(s.into()))
}

pub(crate) fn varr<'p>(items: Vec<Value<'p>>) -> Value<'p> {
    Value::Array(Rc::new(RefCell::new(items)))
}

pub(crate) fn vobj<'p>(fields: Vec<(&str, Value<'p>)>) -> Value<'p> {
    Value::Object(Rc::new(RefCell::new(fields.into_iter().map(|(n, v)| (n.to_string(), v)).collect())))
}

fn member_value<'p>(m: Member) -> Value<'p> {
    match m {
        Member::Fn(b) => Value::Builtin(b),
        Member::Num(n) => Value::Num(n),
        Member::Str(s) => vstr(s),
    }
}

pub struct Env<'p> {
    vars: RefCell<HashMap<String, Value<'p>>>,
    parent: Option<Rc<Env<'p>>>,
}

impl<'p> Env<'p> {
    pub fn new(parent: Option<Rc<Env<'p>>>) -> Rc<Self> {
        Rc::new(Self { vars: RefCell::new(HashMap::new()), parent })
    }

    fn find(self: &Rc<Self>, name: &str) -> Option<Rc<Env<'p>>> {
        let mut env = Rc::clone(self);
        loop {
            if env.vars.borrow().contains_key(name) {
                return Some(env);
            }
            let parent = env.parent.clone()?;
            env = parent;
        }
    }

    fn get(self: &Rc<Self>, name: &str, span: Span) -> Result<Value<'p>, Control<'p>> {
        match self.find(name) {
            Some(env) => Ok(env.vars.borrow().get(name).cloned().unwrap_or(Value::Unit)),
            None => Err(rt(format!("`{}` se usó antes de ser inicializada", name), span)),
        }
    }

    fn set(self: &Rc<Self>, name: &str, value: Value<'p>, span: Span) -> Result<(), Control<'p>> {
        match self.find(name) {
            Some(env) => {
                env.vars.borrow_mut().insert(name.to_string(), value);
                Ok(())
            }
            None => Err(rt(format!("`{}` se usó antes de ser inicializada", name), span)),
        }
    }

    fn define(&self, name: &str, value: Value<'p>) {
        self.vars.borrow_mut().insert(name.to_string(), value);
    }
}

pub enum Control<'p> {
    Return(Value<'p>),
    Break,
    Continue,
    Err(RuntimeError),
}

pub(crate) fn rt<'p>(message: impl Into<String>, span: Span) -> Control<'p> {
    Control::Err(RuntimeError::new(message, span))
}

pub(crate) type EResult<'p> = Result<Value<'p>, Control<'p>>;

// ======================= utilidades =======================

pub fn display(v: &Value, top: bool) -> String {
    match v {
        Value::Str(s) => {
            if top {
                s.to_string()
            } else {
                quote_str(s)
            }
        }
        Value::Num(n) => fmt_num(*n),
        Value::Bool(b) => b.to_string(),
        Value::Unit => "()".into(),
        Value::Array(items) => {
            let inner: Vec<String> = items.borrow().iter().map(|x| display(x, false)).collect();
            format!("[{}]", inner.join(", "))
        }
        Value::Object(fields) => {
            let fields = fields.borrow();
            if fields.is_empty() {
                "{}".into()
            } else {
                let inner: Vec<String> = fields.iter().map(|(k, x)| format!("{}: {}", k, display(x, false))).collect();
                format!("{{ {} }}", inner.join(", "))
            }
        }
        Value::Range(start, end) => format!("{}..{}", fmt_num(*start), fmt_num(*end)),
        Value::Closure(c) => format!("<fn {}>", c.name),
        Value::Builtin(b) => format!("<fn {}>", builtin_name(*b)),
    }
}

pub fn deep_equal<'p>(a: &Value<'p>, b: &Value<'p>) -> bool {
    match (a, b) {
        (Value::Array(x), Value::Array(y)) => {
            let x = x.borrow();
            let y = y.borrow();
            x.len() == y.len() && x.iter().zip(y.iter()).all(|(a, b)| deep_equal(a, b))
        }
        (Value::Object(x), Value::Object(y)) => {
            let x = x.borrow();
            let y = y.borrow();
            x.len() == y.len()
                && x.iter().all(|(k, v)| y.iter().find(|(n, _)| n == k).is_some_and(|(_, w)| deep_equal(v, w)))
        }
        (Value::Range(a1, a2), Value::Range(b1, b2)) => a1 == b1 && a2 == b2,
        (Value::Num(x), Value::Num(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Unit, Value::Unit) => true,
        (Value::Closure(x), Value::Closure(y)) => Rc::ptr_eq(x, y),
        (Value::Builtin(x), Value::Builtin(y)) => x == y,
        _ => false,
    }
}

fn to_base(n: i64, base: u32) -> String {
    let mut digits = Vec::new();
    let mut x = n.unsigned_abs();
    loop {
        digits.push(std::char::from_digit((x % base as u64) as u32, base).unwrap());
        x /= base as u64;
        if x == 0 {
            break;
        }
    }
    if n < 0 {
        digits.push('-');
    }
    digits.iter().rev().collect()
}

/// `slice(desde, hasta)` como en JS: los negativos cuentan desde el final.
fn slice_bounds(args: &[Value], len: usize) -> (usize, usize) {
    let idx = |i: usize, default: usize| -> usize {
        match args.get(i) {
            Some(Value::Num(n)) if *n < 0.0 => (len as f64 + n.trunc()).max(0.0) as usize,
            Some(Value::Num(n)) => (n.trunc() as usize).min(len),
            _ => default,
        }
    };
    let a = idx(0, 0);
    let b = idx(1, len);
    (a.min(b), b)
}

fn obj_field<'p>(fields: &[(String, Value<'p>)], name: &str) -> Option<Value<'p>> {
    fields.iter().find(|(n, _)| n == name).map(|(_, v)| v.clone())
}

fn obj_set<'p>(fields: &mut Vec<(String, Value<'p>)>, name: &str, value: Value<'p>) {
    if let Some(entry) = fields.iter_mut().find(|(n, _)| n == name) {
        entry.1 = value;
    } else {
        fields.push((name.to_string(), value));
    }
}

// ======================= intérprete =======================

pub struct Interpreter<'p, 'o> {
    pub(crate) out: &'o mut dyn FnMut(&str),
    tc: &'p TypeCtx,
    pattern_types: &'p HashMap<usize, Type>,
    pub(crate) script_args: &'p [String],
    depth: usize,
    /// Estado del generador de `math::aleatorio` (xorshift64*).
    pub(crate) rng: u64,
    /// Para `time::reloj()`.
    pub(crate) start: std::time::Instant,
    /// Procesos de `proc::lanzar`, para poder hacer `proc::esperar(pid)`.
    pub(crate) children: HashMap<u32, std::process::Child>,
}

impl<'p, 'o> Interpreter<'p, 'o> {
    pub fn new(
        out: &'o mut dyn FnMut(&str),
        tc: &'p TypeCtx,
        pattern_types: &'p HashMap<usize, Type>,
        script_args: &'p [String],
    ) -> Self {
        Self {
            out,
            tc,
            pattern_types,
            script_args,
            depth: 0,
            rng: crate::native::initial_seed(),
            start: std::time::Instant::now(),
            children: HashMap::new(),
        }
    }

    /// Ejecuta los módulos en el orden de `modules::load`: cada uno ve lo que
    /// exportan sus dependencias. Al final corre `main` del archivo de entrada.
    pub fn run(&mut self, modules: &'p [Module]) -> Result<(), RuntimeError> {
        let globals = Env::new(None);
        for (name, b) in GLOBALS {
            globals.define(name, Value::Builtin(*b));
        }
        let result = (|| -> Result<(), Control<'p>> {
            let mut exports: Vec<Vec<(String, Value<'p>)>> = Vec::with_capacity(modules.len());
            for (i, m) in modules.iter().enumerate() {
                let env = Env::new(Some(Rc::clone(&globals)));
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
                    self.exec_stmt(s, &env)?;
                }

                let mut ex: Vec<(String, Value<'p>)> = Vec::new();
                for s in stmts {
                    let (name, span) = match s {
                        Stmt::Let { exported: true, name, span, .. } => (name, *span),
                        Stmt::Fn(d) if d.exported => (&d.name, d.span),
                        _ => continue,
                    };
                    let v = env.get(name, span)?;
                    ex.retain(|(n, _)| n != name);
                    ex.push((name.clone(), v));
                }
                for d in m.deps.iter().filter(|d| d.reexport) {
                    if let Dep::File(j) = d.dep {
                        ex.extend(exports[j].iter().cloned());
                    }
                }
                exports.push(ex);

                if i + 1 == modules.len() {
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
            Ok(())
        })();
        match result {
            Ok(()) => Ok(()),
            Err(Control::Err(e)) => Err(e),
            Err(Control::Return(_)) | Err(Control::Break) | Err(Control::Continue) => Ok(()),
        }
    }

    fn hoist(&self, stmts: &'p [Stmt], env: &Rc<Env<'p>>) {
        for s in stmts {
            if let Stmt::Fn(d) = s {
                let closure = ClosureV {
                    name: d.name.clone(),
                    params: d.params.iter().map(|p| p.name.clone()).collect(),
                    body: FnBody::Block(&d.body),
                    env: Rc::clone(env),
                };
                env.define(&d.name, Value::Closure(Rc::new(closure)));
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
            Stmt::Expr { expr, .. } => {
                self.eval(expr, env)?;
                Ok(())
            }
            Stmt::Using { ns, name, .. } => {
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
        self.depth += 1;
        let env = Env::new(Some(Rc::clone(&c.env)));
        for (i, p) in c.params.iter().enumerate() {
            env.define(p, args.get(i).cloned().unwrap_or(Value::Unit));
        }
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
            Expr::Bool { value, .. } => Ok(Value::Bool(*value)),
            Expr::Ident { name, span } => env.get(name, *span),
            Expr::Path { ns, name, span } => match stdlib::lookup(ns, name) {
                Some(m) => Ok(member_value(m)),
                None => Err(rt(format!("`{}::{}` no existe", ns, name), *span)),
            },
            Expr::Array { items, .. } => {
                let mut out = Vec::with_capacity(items.len());
                for i in items {
                    out.push(self.eval(i, env)?);
                }
                Ok(varr(out))
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
            Expr::For { name, iter, body, .. } => {
                let it = self.eval(iter, env)?;
                let run = |this: &mut Self, v: Value<'p>| -> Result<bool, Control<'p>> {
                    let inner = Env::new(Some(Rc::clone(env)));
                    inner.define(name, v);
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
                let closure = ClosureV {
                    name: "closure".into(),
                    params: params.iter().map(|p| p.name.clone()).collect(),
                    body: FnBody::Expr(body),
                    env: Rc::clone(env),
                };
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
            Expr::Member { object, name, .. } => {
                let obj = self.eval(object, env)?;
                if let Value::Object(fields) = &obj {
                    obj_set(&mut fields.borrow_mut(), name, value);
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

    fn call_method(&mut self, recv: &Value<'p>, name: &str, args: Vec<Value<'p>>, span: Span) -> EResult<'p> {
        match recv {
            Value::Array(items) => match name {
                "len" => Ok(Value::Num(items.borrow().len() as f64)),
                "push" => {
                    items.borrow_mut().push(args.into_iter().next().unwrap_or(Value::Unit));
                    Ok(Value::Unit)
                }
                "pop" => {
                    let v = items.borrow_mut().pop();
                    v.ok_or_else(|| rt("`pop()` en un arreglo vacío", span))
                }
                "contains" => {
                    let target = args.first().cloned().unwrap_or(Value::Unit);
                    Ok(Value::Bool(items.borrow().iter().any(|x| deep_equal(x, &target))))
                }
                "reverse" => {
                    let mut out = items.borrow().clone();
                    out.reverse();
                    Ok(varr(out))
                }
                "join" => {
                    let sep = match args.first() {
                        Some(Value::Str(s)) => s.to_string(),
                        _ => String::new(),
                    };
                    let parts: Vec<String> = items.borrow().iter().map(|x| display(x, true)).collect();
                    Ok(vstr(parts.join(&sep)))
                }
                "map" => {
                    let f = args.into_iter().next().unwrap_or(Value::Unit);
                    let snapshot: Vec<Value<'p>> = items.borrow().clone();
                    let mut out = Vec::with_capacity(snapshot.len());
                    for x in snapshot {
                        out.push(self.call_value(&f, vec![x], span)?);
                    }
                    Ok(varr(out))
                }
                "filter" => {
                    let f = args.into_iter().next().unwrap_or(Value::Unit);
                    let snapshot: Vec<Value<'p>> = items.borrow().clone();
                    let mut out = Vec::new();
                    for x in snapshot {
                        let keep = self.call_value(&f, vec![x.clone()], span)?;
                        if matches!(keep, Value::Bool(true)) {
                            out.push(x);
                        }
                    }
                    Ok(varr(out))
                }
                "any" | "all" => {
                    let f = args.into_iter().next().unwrap_or(Value::Unit);
                    let snapshot: Vec<Value<'p>> = items.borrow().clone();
                    let mut result = name == "all";
                    for x in snapshot {
                        let v = self.call_value(&f, vec![x], span)?;
                        let hit = matches!(v, Value::Bool(true));
                        if name == "any" && hit {
                            result = true;
                            break;
                        }
                        if name == "all" && !hit {
                            result = false;
                            break;
                        }
                    }
                    Ok(Value::Bool(result))
                }
                "fold" => {
                    let mut it = args.into_iter();
                    let mut acc = it.next().unwrap_or(Value::Unit);
                    let f = it.next().unwrap_or(Value::Unit);
                    let snapshot: Vec<Value<'p>> = items.borrow().clone();
                    for x in snapshot {
                        acc = self.call_value(&f, vec![acc, x], span)?;
                    }
                    Ok(acc)
                }
                "sum" => {
                    let mut total = 0.0;
                    for x in items.borrow().iter() {
                        total += self.num(x, span)?;
                    }
                    Ok(Value::Num(total))
                }
                "sort" => {
                    let mut out = items.borrow().clone();
                    out.sort_by(|a, b| match (a, b) {
                        (Value::Num(x), Value::Num(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
                        (Value::Str(x), Value::Str(y)) => x.cmp(y),
                        _ => std::cmp::Ordering::Equal,
                    });
                    Ok(varr(out))
                }
                "slice" => {
                    let items = items.borrow();
                    let (a, b) = slice_bounds(&args, items.len());
                    Ok(varr(items[a..b].to_vec()))
                }
                "index_of" => {
                    let target = args.first().cloned().unwrap_or(Value::Unit);
                    let pos = items.borrow().iter().position(|x| deep_equal(x, &target));
                    Ok(Value::Num(pos.map_or(-1.0, |p| p as f64)))
                }
                "is_empty" => Ok(Value::Bool(items.borrow().is_empty())),
                _ => Err(rt(format!("método desconocido `{}`", name), span)),
            },
            Value::Str(s) => {
                let arg_str = || -> String {
                    match args.first() {
                        Some(Value::Str(a)) => a.to_string(),
                        _ => String::new(),
                    }
                };
                match name {
                    "len" => Ok(Value::Num(s.chars().count() as f64)),
                    "upper" => Ok(vstr(s.to_uppercase())),
                    "lower" => Ok(vstr(s.to_lowercase())),
                    "trim" => Ok(vstr(s.trim().to_string())),
                    "contains" => Ok(Value::Bool(s.contains(&arg_str()))),
                    "starts_with" => Ok(Value::Bool(s.starts_with(&arg_str()))),
                    "ends_with" => Ok(Value::Bool(s.ends_with(&arg_str()))),
                    "split" => {
                        let sep = arg_str();
                        let parts: Vec<Value<'p>> = if sep.is_empty() {
                            s.chars().map(|c| vstr(c.to_string())).collect()
                        } else {
                            s.split(&sep).map(vstr).collect()
                        };
                        Ok(varr(parts))
                    }
                    "chars" => Ok(varr(s.chars().map(|c| vstr(c.to_string())).collect())),
                    "repeat" => {
                        let n = match args.first() {
                            Some(Value::Num(n)) => (*n).max(0.0) as usize,
                            _ => 0,
                        };
                        Ok(vstr(s.repeat(n)))
                    }
                    "lines" => Ok(varr(s.lines().map(vstr).collect())),
                    "replace" => {
                        let to = match args.get(1) {
                            Some(Value::Str(a)) => a.to_string(),
                            _ => String::new(),
                        };
                        let from = arg_str();
                        Ok(vstr(if from.is_empty() { s.to_string() } else { s.replace(&from, &to) }))
                    }
                    "slice" => {
                        let chars: Vec<char> = s.chars().collect();
                        let (a, b) = slice_bounds(&args, chars.len());
                        Ok(vstr(chars[a..b].iter().collect::<String>()))
                    }
                    "index_of" => {
                        // posición en caracteres, no en bytes
                        let pos = s.find(&arg_str()).map(|byte| s[..byte].chars().count());
                        Ok(Value::Num(pos.map_or(-1.0, |p| p as f64)))
                    }
                    "is_empty" => Ok(Value::Bool(s.is_empty())),
                    "to_number" => {
                        let parsed = s.trim().parse::<f64>().ok().filter(|n| n.is_finite());
                        Ok(match parsed {
                            Some(n) => vobj(vec![("ok", Value::Bool(true)), ("valor", Value::Num(n))]),
                            None => vobj(vec![("ok", Value::Bool(false))]),
                        })
                    }
                    _ => Err(rt(format!("método desconocido `{}`", name), span)),
                }
            }
            Value::Num(n) => {
                let arg_num = || -> f64 {
                    match args.first() {
                        Some(Value::Num(a)) => *a,
                        _ => 0.0,
                    }
                };
                match name {
                    "to_string" => Ok(vstr(fmt_num(*n))),
                    "abs" => Ok(Value::Num(n.abs())),
                    "floor" => Ok(Value::Num(n.floor())),
                    "ceil" => Ok(Value::Num(n.ceil())),
                    // Math.round de JS: redondea hacia +infinito en los .5
                    "round" => Ok(Value::Num((n + 0.5).floor())),
                    "sqrt" => Ok(Value::Num(n.sqrt())),
                    "pow" => Ok(Value::Num(n.powf(arg_num()))),
                    "min" => Ok(Value::Num(n.min(arg_num()))),
                    "max" => Ok(Value::Num(n.max(arg_num()))),
                    "to_base" => {
                        let base = arg_num();
                        if !(2.0..=36.0).contains(&base) || base.fract() != 0.0 {
                            return Err(rt(
                                format!("base inválida {}: tiene que ser un entero entre 2 y 36", fmt_num(base)),
                                span,
                            ));
                        }
                        Ok(vstr(to_base(n.trunc() as i64, base as u32)))
                    }
                    _ => Err(rt(format!("método desconocido `{}`", name), span)),
                }
            }
            Value::Bool(b) if name == "to_string" => Ok(vstr(b.to_string())),
            Value::Range(start, end) if name == "to_array" => {
                let mut out = Vec::new();
                let mut i = *start;
                while i < *end {
                    out.push(Value::Num(i));
                    i += 1.0;
                }
                Ok(varr(out))
            }
            _ => Err(rt(format!("método desconocido `{}`", name), span)),
        }
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
