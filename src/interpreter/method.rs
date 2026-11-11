//! Los métodos de arreglos, strings, números y rangos.

use super::*;

impl<'p> Interpreter<'p, '_> {
    pub(super) fn call_method(
        &mut self,
        recv: &Value<'p>,
        name: &str,
        args: Vec<Value<'p>>,
        span: Span,
    ) -> EResult<'p> {
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
                "enumerate" => {
                    let out = items
                        .borrow()
                        .iter()
                        .enumerate()
                        .map(|(i, x)| vtuple(vec![Value::Num(i as f64), x.clone()]))
                        .collect();
                    Ok(varr(out))
                }
                "zip" => {
                    let Some(Value::Array(other)) = args.first() else {
                        return Err(rt("`zip` necesita un arreglo", span));
                    };
                    let other = other.borrow().clone();
                    let out = items.borrow().iter().zip(other).map(|(a, b)| vtuple(vec![a.clone(), b])).collect();
                    Ok(varr(out))
                }
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
}
