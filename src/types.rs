//! Sistema de tipos de uwu: el de TypeScript, pero sólido.
//! - primitivos: number, string, boolean, void, never
//! - tipos literales: "circulo", 42, true
//! - uniones: A | B
//! - objetos estructurales: { x: number, y: number }
//! - arreglos: T[]
//! - tuplas: (number, string)
//! - funciones: (a: number) => string
//! - alias genéricos y recursivos: type Lista<T> = { tipo: "vacia" } | { tipo: "nodo", valor: T, resto: Lista<T> };

use crate::syntax::ast::LitVal;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    Number,
    String,
    Boolean,
    Void,
    Never,
    Range,
    /// Tipo de recuperación tras un error: compatible con todo.
    Err,
    Lit(LitVal),
    Array(Box<Type>),
    Tuple(Vec<Type>),
    Object(Vec<(String, Type)>),
    Union(Vec<Type>),
    Fn {
        params: Vec<Type>,
        ret: Box<Type>,
        type_params: Vec<String>,
        variadic: bool,
    },
    Param(String),
    Ref {
        name: String,
        args: Vec<Type>,
    },
}

pub const NUMBER: Type = Type::Number;
pub const STRING: Type = Type::String;
pub const BOOLEAN: Type = Type::Boolean;
pub const VOID: Type = Type::Void;
pub const NEVER: Type = Type::Never;
pub const RANGE: Type = Type::Range;
pub const ERR: Type = Type::Err;

pub fn lit(value: LitVal) -> Type {
    Type::Lit(value)
}

pub fn arr(elem: Type) -> Type {
    Type::Array(Box::new(elem))
}

pub fn t_fn(params: Vec<Type>, ret: Type) -> Type {
    Type::Fn { params, ret: Box::new(ret), type_params: vec![], variadic: false }
}

pub fn t_fn_gen(params: Vec<Type>, ret: Type, type_params: Vec<String>) -> Type {
    Type::Fn { params, ret: Box::new(ret), type_params, variadic: false }
}

pub fn t_obj(fields: Vec<(&str, Type)>) -> Type {
    Type::Object(fields.into_iter().map(|(n, t)| (n.to_string(), t)).collect())
}

pub fn param(name: impl Into<String>) -> Type {
    Type::Param(name.into())
}

#[derive(Clone, Debug)]
pub struct Alias {
    pub params: Vec<String>,
    pub body: Type,
}

/// Formatea un número como lo hace JavaScript: sin `.0` para enteros.
pub fn fmt_num(n: f64) -> String {
    if n == 0.0 {
        return "0".into();
    }
    if n.is_nan() {
        return "NaN".into();
    }
    if n.is_infinite() {
        return if n > 0.0 { "Infinity".into() } else { "-Infinity".into() };
    }
    if n.fract() == 0.0 && n.abs() < 1e15 {
        return format!("{}", n as i64);
    }
    format!("{}", n)
}

/// Escapa un string como JSON.stringify.
pub fn quote_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

pub fn lit_str(v: &LitVal) -> String {
    match v {
        LitVal::Str(s) => quote_str(s),
        LitVal::Num(n) => fmt_num(*n),
        LitVal::Bool(b) => b.to_string(),
    }
}

pub fn print(t: &Type) -> String {
    match t {
        Type::Lit(v) => lit_str(v),
        Type::Array(elem) => {
            let inner = print(elem);
            if matches!(**elem, Type::Union(_) | Type::Fn { .. }) {
                format!("({})[]", inner)
            } else {
                format!("{}[]", inner)
            }
        }
        Type::Tuple(items) => match items.as_slice() {
            [one] => format!("({},)", print(one)),
            _ => format!("({})", items.iter().map(print).collect::<Vec<_>>().join(", ")),
        },
        Type::Object(fields) => {
            if fields.is_empty() {
                "{}".into()
            } else {
                let inner: Vec<String> = fields.iter().map(|(n, ft)| format!("{}: {}", n, print(ft))).collect();
                format!("{{ {} }}", inner.join(", "))
            }
        }
        Type::Union(members) => members.iter().map(print).collect::<Vec<_>>().join(" | "),
        Type::Fn { params, ret, type_params, .. } => {
            let tp = if type_params.is_empty() {
                String::new()
            } else {
                format!("<{}>", type_params.iter().map(|n| clean_name(n)).collect::<Vec<_>>().join(", "))
            };
            format!("{}({}) => {}", tp, params.iter().map(print).collect::<Vec<_>>().join(", "), print(ret))
        }
        Type::Param(name) => clean_name(name).to_string(),
        Type::Ref { name, args } => {
            let name = alias_name(name);
            if args.is_empty() {
                name.to_string()
            } else {
                format!("{}<{}>", name, args.iter().map(print).collect::<Vec<_>>().join(", "))
            }
        }
        Type::Err => "{desconocido}".into(),
        Type::Number => "number".into(),
        Type::String => "string".into(),
        Type::Boolean => "boolean".into(),
        Type::Void => "void".into(),
        Type::Never => "never".into(),
        Type::Range => "range".into(),
    }
}

// Los parámetros de tipo se renombran internamente (`T'3`) para evitar colisiones.
pub fn clean_name(n: &str) -> &str {
    n.split('\'').next().unwrap_or(n)
}

pub fn is_fresh(n: &str) -> bool {
    n.contains('\'')
}

// Los alias se guardan como `Nombre#módulo` para que cada archivo tenga sus propios tipos.
pub fn alias_key(name: &str, module: usize) -> String {
    format!("{}#{}", name, module)
}

pub fn alias_name(key: &str) -> &str {
    key.split('#').next().unwrap_or(key)
}

/// Identidad exacta de un tipo (a diferencia de `print`, distingue alias homónimos).
fn type_key(t: &Type) -> String {
    format!("{:?}", t)
}

pub fn subst(t: &Type, map: &HashMap<String, Type>) -> Type {
    if map.is_empty() {
        return t.clone();
    }
    match t {
        Type::Param(name) => map.get(name).cloned().unwrap_or_else(|| t.clone()),
        Type::Array(elem) => arr(subst(elem, map)),
        Type::Tuple(items) => Type::Tuple(items.iter().map(|t| subst(t, map)).collect()),
        Type::Object(fields) => Type::Object(fields.iter().map(|(n, ft)| (n.clone(), subst(ft, map))).collect()),
        Type::Union(members) => mk_union(members.iter().map(|m| subst(m, map)).collect()),
        Type::Fn { params, ret, type_params, variadic } => Type::Fn {
            params: params.iter().map(|p| subst(p, map)).collect(),
            ret: Box::new(subst(ret, map)),
            type_params: type_params.clone(),
            variadic: *variadic,
        },
        Type::Ref { name, args } => {
            Type::Ref { name: name.clone(), args: args.iter().map(|a| subst(a, map)).collect() }
        }
        _ => t.clone(),
    }
}

pub fn contains_param(t: &Type, pred: &dyn Fn(&str) -> bool) -> bool {
    match t {
        Type::Param(name) => pred(name),
        Type::Array(elem) => contains_param(elem, pred),
        Type::Tuple(items) => items.iter().any(|t| contains_param(t, pred)),
        Type::Object(fields) => fields.iter().any(|(_, f)| contains_param(f, pred)),
        Type::Union(members) => members.iter().any(|m| contains_param(m, pred)),
        Type::Fn { params, ret, .. } => params.iter().any(|p| contains_param(p, pred)) || contains_param(ret, pred),
        Type::Ref { args, .. } => args.iter().any(|a| contains_param(a, pred)),
        _ => false,
    }
}

fn base_of(v: &LitVal) -> Type {
    match v {
        LitVal::Num(_) => Type::Number,
        LitVal::Str(_) => Type::String,
        LitVal::Bool(_) => Type::Boolean,
    }
}

/// Construye una unión normalizada: aplana, quita `never`, deduplica y absorbe literales.
pub fn mk_union(types: Vec<Type>) -> Type {
    fn push(t: Type, flat: &mut Vec<Type>) {
        match t {
            Type::Union(members) => members.into_iter().for_each(|m| push(m, flat)),
            Type::Never => {}
            t => flat.push(t),
        }
    }
    let mut flat: Vec<Type> = Vec::new();
    for t in types {
        push(t, &mut flat);
    }
    if flat.iter().any(|t| matches!(t, Type::Err)) {
        return ERR;
    }
    let mut seen = HashSet::new();
    let mut out: Vec<Type> = Vec::new();
    for t in flat {
        let key = type_key(&t);
        if seen.insert(key) {
            out.push(t);
        }
    }
    let has_num = out.iter().any(|t| matches!(t, Type::Number));
    let has_str = out.iter().any(|t| matches!(t, Type::String));
    let has_bool = out.iter().any(|t| matches!(t, Type::Boolean));
    out.retain(|t| {
        !matches!(t, Type::Lit(v) if match v {
            LitVal::Num(_) => has_num,
            LitVal::Str(_) => has_str,
            LitVal::Bool(_) => has_bool,
        })
    });
    let has_true = out.iter().any(|t| matches!(t, Type::Lit(LitVal::Bool(true))));
    let has_false = out.iter().any(|t| matches!(t, Type::Lit(LitVal::Bool(false))));
    if has_true && has_false {
        out.retain(|t| !matches!(t, Type::Lit(LitVal::Bool(_))));
        out.push(BOOLEAN);
    }
    match out.len() {
        0 => NEVER,
        1 => out.pop().unwrap(),
        _ => Type::Union(out),
    }
}

/// `let x = 5` → number (no el literal 5), igual que TS en contextos mutables.
pub fn widen(t: &Type) -> Type {
    match t {
        Type::Lit(v) => base_of(v),
        Type::Array(elem) => arr(widen(elem)),
        Type::Tuple(items) => Type::Tuple(items.iter().map(widen).collect()),
        Type::Object(fields) => Type::Object(fields.iter().map(|(n, ft)| (n.clone(), widen(ft))).collect()),
        Type::Union(members) => mk_union(members.iter().map(widen).collect()),
        _ => t.clone(),
    }
}

pub fn has_never_array(t: &Type) -> bool {
    match t {
        Type::Array(elem) => matches!(**elem, Type::Never) || has_never_array(elem),
        Type::Object(fields) => fields.iter().any(|(_, f)| has_never_array(f)),
        Type::Tuple(items) => items.iter().any(has_never_array),
        Type::Union(members) => members.iter().any(has_never_array),
        _ => false,
    }
}

pub fn obj_get<'a>(fields: &'a [(String, Type)], name: &str) -> Option<&'a Type> {
    fields.iter().find(|(n, _)| n == name).map(|(_, t)| t)
}

#[derive(Clone, Default)]
pub struct TypeCtx {
    pub aliases: HashMap<String, Alias>,
}

impl TypeCtx {
    pub fn new() -> Self {
        Self::default()
    }

    /// Expande alias (`Opcion<number>`) hasta llegar a un tipo concreto.
    pub fn resolve(&self, t: &Type) -> Type {
        let mut t = t.clone();
        let mut depth = 0;
        while let Type::Ref { name, args } = &t {
            let Some(alias) = self.aliases.get(name) else { return ERR };
            if depth > 64 {
                return ERR;
            }
            depth += 1;
            t = if alias.params.is_empty() {
                alias.body.clone()
            } else {
                let map: HashMap<String, Type> = alias
                    .params
                    .iter()
                    .enumerate()
                    .map(|(i, p)| (p.clone(), args.get(i).cloned().unwrap_or(ERR)))
                    .collect();
                subst(&alias.body, &map)
            };
        }
        t
    }

    /// Miembros de una unión, resueltos. `boolean` se expande a `true | false` si se pide.
    pub fn flatten(&self, t: &Type, expand_bool: bool) -> Vec<Type> {
        self.flatten_depth(t, expand_bool, 0)
    }

    fn flatten_depth(&self, t: &Type, expand_bool: bool, depth: u32) -> Vec<Type> {
        let r = self.resolve(t);
        if depth > 64 {
            return vec![ERR];
        }
        match r {
            Type::Union(members) => {
                members.iter().flat_map(|m| self.flatten_depth(m, expand_bool, depth + 1)).collect()
            }
            Type::Boolean if expand_bool => vec![lit(LitVal::Bool(true)), lit(LitVal::Bool(false))],
            r => vec![r],
        }
    }

    /// ¿Un valor de tipo `src` puede usarse donde se espera `dst`?
    pub fn is_assignable(&self, src: &Type, dst: &Type) -> bool {
        self.assignable(src, dst, &mut HashSet::new())
    }

    fn assignable(&self, src: &Type, dst: &Type, seen: &mut HashSet<String>) -> bool {
        if matches!(src, Type::Err) || matches!(dst, Type::Err) || matches!(src, Type::Never) {
            return true;
        }
        let mut src = src.clone();
        let mut dst = dst.clone();
        if matches!(src, Type::Ref { .. }) || matches!(dst, Type::Ref { .. }) {
            if let (Type::Ref { name: sn, args: sa }, Type::Ref { name: dn, args: da }) = (&src, &dst) {
                if sn == dn && sa.len() == da.len() && sa.iter().zip(da).all(|(a, b)| self.assignable(a, b, seen)) {
                    return true;
                }
            }
            // tipos recursivos: asumimos verdadero mientras lo estamos comprobando
            let key = format!("{} <: {}", type_key(&src), type_key(&dst));
            if seen.contains(&key) {
                return true;
            }
            seen.insert(key);
            src = self.resolve(&src);
            dst = self.resolve(&dst);
            if matches!(src, Type::Err) || matches!(dst, Type::Err) || matches!(src, Type::Never) {
                return true;
            }
        }
        if let Type::Union(members) = &src {
            return members.iter().all(|m| self.assignable(m, &dst, seen));
        }
        if let Type::Union(members) = &dst {
            if members.iter().any(|m| self.assignable(&src, m, seen)) {
                return true;
            }
            if matches!(src, Type::Boolean) {
                return self.assignable(&lit(LitVal::Bool(true)), &dst, seen)
                    && self.assignable(&lit(LitVal::Bool(false)), &dst, seen);
            }
            return false;
        }
        match &dst {
            Type::Number | Type::String | Type::Boolean => {
                src == dst || matches!(&src, Type::Lit(v) if base_of(v) == dst)
            }
            Type::Lit(dv) => matches!(&src, Type::Lit(sv) if sv == dv),
            Type::Void | Type::Range => src == dst,
            Type::Array(de) => matches!(&src, Type::Array(se) if self.assignable(se, de, seen)),
            Type::Tuple(ditems) => matches!(&src, Type::Tuple(sitems)
                if sitems.len() == ditems.len()
                    && sitems.iter().zip(ditems).all(|(s, d)| self.assignable(s, d, seen))),
            Type::Object(dfields) => {
                let Type::Object(sfields) = &src else { return false };
                dfields.iter().all(|(name, ft)| obj_get(sfields, name).is_some_and(|sf| self.assignable(sf, ft, seen)))
            }
            Type::Fn { params: dp, ret: dr, .. } => {
                let Type::Fn { params: sp, ret: sr, .. } = &src else { return false };
                if sp.len() > dp.len() {
                    return false;
                }
                sp.iter().enumerate().all(|(i, p)| self.assignable(&dp[i], p, seen))
                    && (matches!(**dr, Type::Void) || self.assignable(sr, dr, seen))
            }
            Type::Param(dn) => matches!(&src, Type::Param(sn) if sn == dn),
            _ => false,
        }
    }

    /// ¿Existe algún valor que pertenezca a ambos tipos? Se usa para `==`.
    pub fn overlaps(&self, a: &Type, b: &Type) -> bool {
        let fa = self.flatten(a, true);
        let fb = self.flatten(b, true);
        fa.iter().any(|x| fb.iter().any(|y| self.is_assignable(x, y) || self.is_assignable(y, x)))
    }
}
