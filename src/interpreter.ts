import type { Block, Expr, Pattern, Program, Stmt } from "./ast";
import { RuntimeError } from "./diagnostics";
import type { Span } from "./lexer";
import type { Type, TypeCtx } from "./types";

// ======================= valores =======================

export class ObjectValue {
  constructor(public fields: Map<string, Value>) {}
}

export class Closure {
  constructor(public name: string, public params: string[], public body: Block | Expr, public env: Env) {}
}

export class Builtin {
  constructor(public name: string, public call: (args: Value[], span: Span) => Value) {}
}

export class RangeValue {
  constructor(public start: number, public end: number) {}
}

export type Value = number | string | boolean | undefined | Value[] | ObjectValue | Closure | Builtin | RangeValue;

class Env {
  vars = new Map<string, Value>();
  constructor(public parent: Env | null) {}

  private find(name: string): Env | null {
    let env: Env | null = this;
    while (env && !env.vars.has(name)) env = env.parent;
    return env;
  }

  get(name: string, span: Span): Value {
    const env = this.find(name);
    if (!env) throw new RuntimeError(`\`${name}\` se usó antes de ser inicializada`, span);
    return env.vars.get(name);
  }

  set(name: string, value: Value, span: Span) {
    const env = this.find(name);
    if (!env) throw new RuntimeError(`\`${name}\` se usó antes de ser inicializada`, span);
    env.vars.set(name, value);
  }

  define(name: string, value: Value) {
    this.vars.set(name, value);
  }
}

class ReturnSignal {
  constructor(public value: Value) {}
}
class BreakSignal {}
class ContinueSignal {}

// ======================= utilidades =======================

export function display(v: Value, top = true): string {
  if (typeof v === "string") return top ? v : JSON.stringify(v);
  if (typeof v === "number" || typeof v === "boolean") return String(v);
  if (v === undefined) return "()";
  if (Array.isArray(v)) return `[${v.map((x) => display(x, false)).join(", ")}]`;
  if (v instanceof ObjectValue) {
    if (v.fields.size === 0) return "{}";
    return `{ ${[...v.fields].map(([k, x]) => `${k}: ${display(x, false)}`).join(", ")} }`;
  }
  if (v instanceof RangeValue) return `${v.start}..${v.end}`;
  return `<fn ${v.name}>`;
}

export function deepEqual(a: Value, b: Value): boolean {
  if (Array.isArray(a) && Array.isArray(b)) {
    return a.length === b.length && a.every((x, i) => deepEqual(x, b[i]));
  }
  if (a instanceof ObjectValue && b instanceof ObjectValue) {
    return a.fields.size === b.fields.size &&
      [...a.fields].every(([k, x]) => b.fields.has(k) && deepEqual(x, b.fields.get(k)));
  }
  if (a instanceof RangeValue && b instanceof RangeValue) return a.start === b.start && a.end === b.end;
  return a === b;
}

// ======================= intérprete =======================

export interface InterpreterOptions {
  out: (line: string) => void;
  tc: TypeCtx;
  patternTypes: WeakMap<Pattern, Type>;
}

export class Interpreter {
  private globals = new Env(null);

  constructor(private opts: InterpreterOptions) {
    const g = this.globals;
    g.define("print", new Builtin("print", (args) => {
      this.opts.out(args.map((a) => display(a)).join(" "));
      return undefined;
    }));
    g.define("panic", new Builtin("panic", (args, span) => {
      throw new RuntimeError(`pánico: ${display(args[0])}`, span);
    }));
    g.define("assert", new Builtin("assert", (args, span) => {
      if (args[0] !== true) throw new RuntimeError("la aserción falló", span);
      return undefined;
    }));
  }

  run(program: Program) {
    try {
      this.hoist(program.stmts, this.globals);
      for (const s of program.stmts) this.execStmt(s, this.globals);
      const main = program.stmts.find((s) => s.k === "fn" && s.decl.name === "main");
      if (main) this.callValue(this.globals.get("main", main.span), [], main.span);
    } catch (e) {
      if (e instanceof RangeError) {
        throw new RuntimeError("desbordamiento de pila (¿recursión infinita?)", { start: 0, end: 0, line: 1, col: 1 });
      }
      throw e;
    }
  }

  private hoist(stmts: Stmt[], env: Env) {
    for (const s of stmts) {
      if (s.k === "fn") env.define(s.decl.name, new Closure(s.decl.name, s.decl.params.map((p) => p.name), s.decl.body, env));
    }
  }

  private execStmt(s: Stmt, env: Env) {
    switch (s.k) {
      case "let":
        env.define(s.name, this.eval(s.init, env));
        return;
      case "expr":
        this.eval(s.expr, env);
        return;
      default:
        return; // fn y type ya se procesaron
    }
  }

  private evalBlock(b: Block, env: Env): Value {
    this.hoist(b.stmts, env);
    for (const s of b.stmts) this.execStmt(s, env);
    return b.tail ? this.eval(b.tail, env) : undefined;
  }

  callValue(f: Value, args: Value[], span: Span): Value {
    if (f instanceof Builtin) return f.call(args, span);
    if (!(f instanceof Closure)) throw new RuntimeError("se intentó llamar a algo que no es una función", span);
    const env = new Env(f.env);
    f.params.forEach((p, i) => env.define(p, args[i]));
    try {
      return "stmts" in f.body ? this.evalBlock(f.body, env) : this.eval(f.body, env);
    } catch (e) {
      if (e instanceof ReturnSignal) return e.value;
      throw e;
    }
  }

  private num(v: Value): number {
    return v as number;
  }

  eval(e: Expr, env: Env): Value {
    switch (e.k) {
      case "num":
      case "str":
      case "bool":
        return e.value;
      case "ident":
        return env.get(e.name, e.span);
      case "array":
        return e.items.map((i) => this.eval(i, env));
      case "object":
        return new ObjectValue(new Map(e.fields.map((f) => [f.name, this.eval(f.value, env)])));
      case "unary": {
        const v = this.eval(e.expr, env);
        return e.op === "-" ? -this.num(v) : !v;
      }
      case "binary":
        return this.evalBinary(e.op, e.left, e.right, e.span, env);
      case "range":
        return new RangeValue(this.num(this.eval(e.start, env)), this.num(this.eval(e.end, env)));
      case "assign": {
        let value = this.eval(e.value, env);
        if (e.op !== "=") value = this.arith(e.op[0], this.eval(e.target, env), value, e.span);
        this.assign(e.target, value, env);
        return undefined;
      }
      case "call":
        return this.evalCall(e, env);
      case "member": {
        const obj = this.eval(e.object, env) as ObjectValue;
        return obj.fields.get(e.name);
      }
      case "index": {
        const obj = this.eval(e.object, env);
        const i = this.num(this.eval(e.index, env));
        if (typeof obj === "string") {
          const chars = [...obj];
          this.checkIndex(i, chars.length, e.index.span);
          return chars[i];
        }
        const a = obj as Value[];
        this.checkIndex(i, a.length, e.index.span);
        return a[i];
      }
      case "block":
        return this.evalBlock(e.block, new Env(env));
      case "if":
        if (this.eval(e.cond, env)) return this.evalBlock(e.then, new Env(env));
        return e.else_ ? this.eval(e.else_, env) : undefined;
      case "while":
        while (this.eval(e.cond, env)) {
          if (this.loopBody(e.body, env) === "break") break;
        }
        return undefined;
      case "loop":
        for (;;) {
          if (this.loopBody(e.body, env) === "break") break;
        }
        return undefined;
      case "for": {
        const it = this.eval(e.iter, env);
        const run = (v: Value) => {
          const inner = new Env(env);
          inner.define(e.name, v);
          return this.loopBody(e.body, inner);
        };
        if (it instanceof RangeValue) {
          for (let i = it.start; i < it.end; i++) if (run(i) === "break") break;
        } else {
          const items = typeof it === "string" ? [...it] : [...(it as Value[])];
          for (const v of items) if (run(v) === "break") break;
        }
        return undefined;
      }
      case "match": {
        const v = this.eval(e.subject, env);
        for (const arm of e.arms) {
          const armEnv = new Env(env);
          if (this.matchPattern(arm.pattern, v, armEnv) && (!arm.guard || this.eval(arm.guard, armEnv))) {
            return this.eval(arm.body, armEnv);
          }
        }
        throw new RuntimeError(`ningún brazo del \`match\` coincidió con ${display(v, false)}`, e.span);
      }
      case "closure":
        return new Closure("closure", e.params.map((p) => p.name), e.body, env);
      case "return":
        throw new ReturnSignal(e.value ? this.eval(e.value, env) : undefined);
      case "break":
        throw new BreakSignal();
      case "continue":
        throw new ContinueSignal();
    }
  }

  private loopBody(body: Block, env: Env): "break" | "continue" | void {
    try {
      this.evalBlock(body, new Env(env));
    } catch (err) {
      if (err instanceof BreakSignal) return "break";
      if (err instanceof ContinueSignal) return "continue";
      throw err;
    }
  }

  private checkIndex(i: number, len: number, span: Span) {
    if (!Number.isInteger(i) || i < 0 || i >= len) {
      throw new RuntimeError(`índice fuera de rango: el largo es ${len} pero el índice es ${i}`, span);
    }
  }

  private assign(target: Expr, value: Value, env: Env) {
    if (target.k === "ident") env.set(target.name, value, target.span);
    else if (target.k === "member") (this.eval(target.object, env) as ObjectValue).fields.set(target.name, value);
    else if (target.k === "index") {
      const a = this.eval(target.object, env) as Value[];
      const i = this.num(this.eval(target.index, env));
      this.checkIndex(i, a.length, target.index.span);
      a[i] = value;
    }
  }

  private arith(op: string, l: Value, r: Value, span: Span): Value {
    switch (op) {
      case "+":
        return typeof l === "string" || typeof r === "string" ? display(l) + display(r) : this.num(l) + this.num(r);
      case "-":
        return this.num(l) - this.num(r);
      case "*":
        return this.num(l) * this.num(r);
      case "/":
        if (r === 0) throw new RuntimeError("división por cero", span);
        return this.num(l) / this.num(r);
      case "%":
        if (r === 0) throw new RuntimeError("división por cero", span);
        return this.num(l) % this.num(r);
    }
    throw new RuntimeError(`operador desconocido ${op}`, span);
  }

  private evalBinary(op: string, left: Expr, right: Expr, span: Span, env: Env): Value {
    if (op === "&&") return !!this.eval(left, env) && !!this.eval(right, env);
    if (op === "||") return !!this.eval(left, env) || !!this.eval(right, env);
    const l = this.eval(left, env);
    const r = this.eval(right, env);
    switch (op) {
      case "==":
        return deepEqual(l, r);
      case "!=":
        return !deepEqual(l, r);
      case "<":
        return (l as number) < (r as number);
      case ">":
        return (l as number) > (r as number);
      case "<=":
        return (l as number) <= (r as number);
      case ">=":
        return (l as number) >= (r as number);
      default:
        return this.arith(op, l, r, span);
    }
  }

  private evalCall(e: Extract<Expr, { k: "call" }>, env: Env): Value {
    if (e.callee.k === "member") {
      const obj = this.eval(e.callee.object, env);
      const args = e.args.map((a) => this.eval(a, env));
      if (obj instanceof ObjectValue && obj.fields.has(e.callee.name)) {
        return this.callValue(obj.fields.get(e.callee.name), args, e.span);
      }
      return this.callMethod(obj, e.callee.name, args, e.span);
    }
    const f = this.eval(e.callee, env);
    return this.callValue(f, e.args.map((a) => this.eval(a, env)), e.span);
  }

  private callMethod(recv: Value, name: string, args: Value[], span: Span): Value {
    const call = (f: Value, ...xs: Value[]) => this.callValue(f, xs, span);
    if (Array.isArray(recv)) {
      switch (name) {
        case "len": return recv.length;
        case "push": recv.push(args[0]); return undefined;
        case "pop":
          if (recv.length === 0) throw new RuntimeError("`pop()` en un arreglo vacío", span);
          return recv.pop();
        case "contains": return recv.some((x) => deepEqual(x, args[0]));
        case "reverse": return [...recv].reverse();
        case "join": return recv.map((x) => display(x)).join(args[0] as string);
        case "map": return recv.map((x) => call(args[0], x));
        case "filter": return recv.filter((x) => call(args[0], x) === true);
        case "any": return recv.some((x) => call(args[0], x) === true);
        case "all": return recv.every((x) => call(args[0], x) === true);
        case "fold": return recv.reduce((acc: Value, x) => call(args[1], acc, x), args[0]);
        case "sum": return (recv as number[]).reduce((a, b) => a + b, 0);
      }
    } else if (typeof recv === "string") {
      switch (name) {
        case "len": return [...recv].length;
        case "upper": return recv.toUpperCase();
        case "lower": return recv.toLowerCase();
        case "trim": return recv.trim();
        case "contains": return recv.includes(args[0] as string);
        case "starts_with": return recv.startsWith(args[0] as string);
        case "ends_with": return recv.endsWith(args[0] as string);
        case "split": return recv.split(args[0] as string);
        case "chars": return [...recv];
        case "repeat": return recv.repeat(Math.max(0, args[0] as number));
      }
    } else if (typeof recv === "number") {
      const n = args[0] as number;
      switch (name) {
        case "to_string": return display(recv);
        case "abs": return Math.abs(recv);
        case "floor": return Math.floor(recv);
        case "ceil": return Math.ceil(recv);
        case "round": return Math.round(recv);
        case "sqrt": return Math.sqrt(recv);
        case "pow": return recv ** n;
        case "min": return Math.min(recv, n);
        case "max": return Math.max(recv, n);
      }
    } else if (typeof recv === "boolean" && name === "to_string") {
      return String(recv);
    } else if (recv instanceof RangeValue && name === "to_array") {
      return Array.from({ length: Math.max(0, recv.end - recv.start) }, (_, i) => recv.start + i);
    }
    throw new RuntimeError(`método desconocido \`${name}\``, span);
  }

  // ---------- patrones ----------

  private matchPattern(p: Pattern, v: Value, env: Env): boolean {
    switch (p.k) {
      case "wild": {
        const t = this.opts.patternTypes.get(p);
        return !t || this.valueIs(v, t);
      }
      case "bind": {
        const t = this.opts.patternTypes.get(p);
        if (t && !this.valueIs(v, t)) return false;
        env.define(p.name, v);
        return true;
      }
      case "lit":
        return v === p.value;
      case "object":
        return v instanceof ObjectValue && p.fields.every((f) =>
          v.fields.has(f.name) && this.matchPattern(f.pattern, v.fields.get(f.name), env));
    }
  }

  /** Chequeo de tipo en ejecución, para patrones como `n: number`. */
  private valueIs(v: Value, t: Type): boolean {
    const r = this.opts.tc.resolve(t);
    switch (r.k) {
      case "number":
      case "string":
      case "boolean":
        return typeof v === r.k;
      case "lit":
        return v === r.value;
      case "void":
        return v === undefined;
      case "range":
        return v instanceof RangeValue;
      case "array":
        return Array.isArray(v) && v.every((x) => this.valueIs(x, r.elem));
      case "object":
        return v instanceof ObjectValue && [...r.fields].every(([n, ft]) => v.fields.has(n) && this.valueIs(v.fields.get(n), ft));
      case "union":
        return r.members.some((m) => this.valueIs(v, m));
      case "fn":
        return v instanceof Closure || v instanceof Builtin;
      case "never":
        return false;
      default:
        return true; // parámetros de tipo: se borran en ejecución
    }
  }
}
