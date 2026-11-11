//! Los valores en ejecución, los entornos y funciones auxiliares sobre valores.

use crate::diagnostics::RuntimeError;
use crate::stdlib::{builtin_name, BuiltinFn, Member};
use crate::syntax::ast::*;
use crate::syntax::Span;
use crate::types::{fmt_num, quote_str};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct ClosureV<'p> {
    pub name: String,
    pub params: &'p [ParamNode],
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
    Tuple(Rc<RefCell<Vec<Value<'p>>>>),
    Object(Rc<RefCell<Vec<(String, Value<'p>)>>>),
    Range(f64, f64),
    Closure(Rc<ClosureV<'p>>),
    Builtin(BuiltinFn),
    /// Un `namespace` declarado en uwu. Se guarda en el entorno como `nombre::`.
    Namespace(Rc<Env<'p>>),
}

pub(crate) fn vstr<'p>(s: impl Into<String>) -> Value<'p> {
    Value::Str(Rc::new(s.into()))
}

pub(crate) fn varr<'p>(items: Vec<Value<'p>>) -> Value<'p> {
    Value::Array(Rc::new(RefCell::new(items)))
}

pub(crate) fn vtuple<'p>(items: Vec<Value<'p>>) -> Value<'p> {
    Value::Tuple(Rc::new(RefCell::new(items)))
}

/// Nombre con el que se guarda un namespace en el entorno (no choca con ninguna variable).
pub(super) fn ns_key(name: &str) -> String {
    format!("{}::", name)
}

pub(crate) fn vobj<'p>(fields: Vec<(&str, Value<'p>)>) -> Value<'p> {
    Value::Object(Rc::new(RefCell::new(fields.into_iter().map(|(n, v)| (n.to_string(), v)).collect())))
}

pub(super) fn member_value<'p>(m: Member) -> Value<'p> {
    match m {
        Member::Fn(b) => Value::Builtin(b),
        Member::Num(n) => Value::Num(n),
        Member::Str(s) => vstr(s),
    }
}

pub struct Env<'p> {
    pub(super) vars: RefCell<HashMap<String, Value<'p>>>,
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

    pub(super) fn get(self: &Rc<Self>, name: &str, span: Span) -> Result<Value<'p>, Control<'p>> {
        match self.find(name) {
            Some(env) => Ok(env.vars.borrow().get(name).cloned().unwrap_or(Value::Unit)),
            None => Err(rt(format!("`{}` se usó antes de ser inicializada", name), span)),
        }
    }

    pub(super) fn set(self: &Rc<Self>, name: &str, value: Value<'p>, span: Span) -> Result<(), Control<'p>> {
        match self.find(name) {
            Some(env) => {
                env.vars.borrow_mut().insert(name.to_string(), value);
                Ok(())
            }
            None => Err(rt(format!("`{}` se usó antes de ser inicializada", name), span)),
        }
    }

    pub(super) fn define(&self, name: &str, value: Value<'p>) {
        self.vars.borrow_mut().insert(name.to_string(), value);
    }

    pub(super) fn lookup(self: &Rc<Self>, name: &str) -> Option<Value<'p>> {
        self.find(name).and_then(|env| env.vars.borrow().get(name).cloned())
    }

    /// Sólo en este entorno, sin mirar los de afuera (para los miembros de un namespace).
    pub(super) fn own(&self, name: &str) -> Option<Value<'p>> {
        self.vars.borrow().get(name).cloned()
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
        Value::Tuple(items) => {
            let inner: Vec<String> = items.borrow().iter().map(|x| display(x, false)).collect();
            if inner.len() == 1 {
                format!("({},)", inner[0])
            } else {
                format!("({})", inner.join(", "))
            }
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
        Value::Namespace(_) => "<namespace>".into(),
    }
}

pub fn deep_equal<'p>(a: &Value<'p>, b: &Value<'p>) -> bool {
    match (a, b) {
        (Value::Array(x), Value::Array(y)) | (Value::Tuple(x), Value::Tuple(y)) => {
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

pub(super) fn to_base(n: i64, base: u32) -> String {
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
pub(super) fn slice_bounds(args: &[Value], len: usize) -> (usize, usize) {
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

pub(super) fn obj_field<'p>(fields: &[(String, Value<'p>)], name: &str) -> Option<Value<'p>> {
    fields.iter().find(|(n, _)| n == name).map(|(_, v)| v.clone())
}

pub(super) fn obj_set<'p>(fields: &mut Vec<(String, Value<'p>)>, name: &str, value: Value<'p>) {
    if let Some(entry) = fields.iter_mut().find(|(n, _)| n == name) {
        entry.1 = value;
    } else {
        fields.push((name.to_string(), value));
    }
}
