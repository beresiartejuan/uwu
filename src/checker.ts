import type { Arm, Block, Expr, FnDecl, Pattern, Program, Stmt, TypeNode } from "./ast";
import type { Diagnostic } from "./diagnostics";
import type { Span } from "./lexer";
import {
  arr, BOOLEAN, containsParam, ERR, fn, hasNeverArray, isFresh, lit, mkUnion, NEVER, NUMBER,
  param, print, RANGE, STRING, subst, Type, TypeCtx, VOID, widen,
} from "./types";

interface VarInfo {
  type: Type;
  mut: boolean;
  kind: "var" | "fn" | "builtin";
}

class Scope {
  vars = new Map<string, VarInfo>();
  constructor(public parent: Scope | null) {}
  lookup(name: string): VarInfo | undefined {
    return this.vars.get(name) ?? this.parent?.lookup(name);
  }
  names(): string[] {
    return [...this.vars.keys(), ...(this.parent?.names() ?? [])];
  }
}

interface FnCtx {
  ret: Type | null; // null = inferir (closures)
  inferred: Type[];
}

const PRIMITIVES: Record<string, Type> = {
  number: NUMBER, string: STRING, boolean: BOOLEAN, void: VOID, never: { k: "never" },
};

const TYPE_HINTS: Record<string, string> = {
  int: "number", float: "number", i32: "number", i64: "number", f64: "number", u32: "number",
  usize: "number", str: "string", String: "string", bool: "boolean", Number: "number", Boolean: "boolean",
};

export const BUILTINS: Record<string, Type> = {
  print: { k: "fn", params: [ERR], ret: VOID, typeParams: [], variadic: true },
  panic: fn([STRING], NEVER),
  assert: fn([BOOLEAN], VOID),
};

const MUTATING_METHODS = new Set(["push", "pop"]);

export class Checker {
  diagnostics: Diagnostic[] = [];
  tc = new TypeCtx();
  /** Tipos de los patrones `x: T`, que el intérprete necesita para chequear en ejecución. */
  patternTypes = new WeakMap<Pattern, Type>();

  private scope: Scope;
  private typeParamScopes: Set<string>[] = [];
  private fnStack: FnCtx[] = [];
  private loopStack: ({ hasBreak: boolean } | null)[] = [];
  private signatures = new WeakMap<FnDecl, Type>();
  private freshCounter = 0;

  constructor() {
    this.scope = new Scope(null);
    for (const [name, type] of Object.entries(BUILTINS)) {
      this.scope.vars.set(name, { type, mut: false, kind: "builtin" });
    }
  }

  private error(message: string, span: Span, hint?: string) {
    this.diagnostics.push({ message, span, hint, severity: "error" });
  }

  private warn(message: string, span: Span, hint?: string) {
    this.diagnostics.push({ message, span, hint, severity: "warning" });
  }

  private expectAssignable(src: Type, dst: Type, span: Span, hint?: string): boolean {
    if (this.tc.isAssignable(src, dst)) return true;
    this.error(`tipos incompatibles: se esperaba \`${print(dst)}\`, se encontró \`${print(src)}\``, span, hint);
    return false;
  }

  // ======================= programa =======================

  check(program: Program): Diagnostic[] {
    this.hoist(program.stmts);
    for (const s of program.stmts) this.checkStmt(s);
    const main = program.stmts.find((s) => s.k === "fn" && s.decl.name === "main");
    if (main && main.k === "fn" && main.decl.params.length > 0) {
      this.error("`main` no puede recibir parámetros", main.decl.span);
    }
    return this.diagnostics;
  }

  /** Registra tipos y funciones de un bloque antes de chequearlo (permite recursión mutua). */
  private hoist(stmts: Stmt[]) {
    const typeDecls = stmts.flatMap((s) => (s.k === "type" ? [s.decl] : []));
    for (const d of typeDecls) {
      if (this.tc.aliases.has(d.name) || PRIMITIVES[d.name]) {
        this.error(`el tipo \`${d.name}\` ya está definido`, d.span);
        continue;
      }
      this.tc.aliases.set(d.name, { params: d.params, body: ERR });
    }
    for (const d of typeDecls) {
      const alias = this.tc.aliases.get(d.name);
      if (!alias) continue;
      alias.body = this.withTypeParams(d.params, () => this.toType(d.type));
      if (this.tc.resolve({ k: "ref", name: d.name, args: d.params.map(() => ERR) }).k === "err" && alias.body.k !== "err") {
        this.error(`el tipo \`${d.name}\` se refiere a sí mismo sin estructura`, d.span,
          "un tipo recursivo tiene que pasar por un objeto o un arreglo, ej: `{ siguiente: Lista }`");
        alias.body = ERR;
      }
    }
    for (const s of stmts) {
      if (s.k !== "fn") continue;
      const d = s.decl;
      if (this.scope.vars.has(d.name)) {
        this.error(`\`${d.name}\` ya está definido en este bloque`, d.span);
        continue;
      }
      const sig = this.withTypeParams(d.typeParams, () =>
        fn(d.params.map((p) => (p.type ? this.toType(p.type) : ERR)), d.ret ? this.toType(d.ret) : VOID, d.typeParams));
      this.signatures.set(d, sig);
      this.scope.vars.set(d.name, { type: sig, mut: false, kind: "fn" });
    }
  }

  private withTypeParams<T>(params: string[], f: () => T): T {
    this.typeParamScopes.push(new Set(params));
    try {
      return f();
    } finally {
      this.typeParamScopes.pop();
    }
  }

  private toType(node: TypeNode): Type {
    switch (node.k) {
      case "name": {
        const prim = PRIMITIVES[node.name];
        if (prim) {
          if (node.args.length) this.error(`\`${node.name}\` no recibe parámetros de tipo`, node.span);
          return prim;
        }
        if (this.typeParamScopes.some((s) => s.has(node.name))) return param(node.name);
        const alias = this.tc.aliases.get(node.name);
        if (alias) {
          if (alias.params.length !== node.args.length) {
            this.error(`\`${node.name}\` espera ${alias.params.length} parámetro(s) de tipo, recibió ${node.args.length}`, node.span);
            return ERR;
          }
          return { k: "ref", name: node.name, args: node.args.map((a) => this.toType(a)) };
        }
        const suggestion = TYPE_HINTS[node.name];
        this.error(`tipo desconocido \`${node.name}\``, node.span, suggestion ? `en uwu se escribe \`${suggestion}\`` : undefined);
        return ERR;
      }
      case "lit":
        return lit(node.value);
      case "array":
        return arr(this.toType(node.elem));
      case "object": {
        const fields = new Map<string, Type>();
        for (const f of node.fields) {
          if (fields.has(f.name)) this.error(`campo \`${f.name}\` repetido`, node.span);
          fields.set(f.name, this.toType(f.type));
        }
        return { k: "object", fields };
      }
      case "union":
        return mkUnion(node.members.map((m) => this.toType(m)));
      case "fn":
        return fn(node.params.map((p) => this.toType(p)), this.toType(node.ret));
    }
  }

  // ======================= sentencias =======================

  private checkStmt(s: Stmt): Type {
    switch (s.k) {
      case "let": {
        const ann = s.type ? this.toType(s.type) : null;
        const init = this.checkExpr(s.init, ann ?? undefined);
        let type: Type;
        if (ann) {
          this.expectAssignable(init, ann, s.init.span);
          type = ann;
        } else {
          type = widen(init);
          if (hasNeverArray(type)) {
            this.error(`no se puede inferir el tipo de \`${s.name}\``, s.init.span,
              `agregá una anotación, ej: \`let ${s.mut ? "mut " : ""}${s.name}: number[] = [];\``);
            type = ERR;
          }
          if (type.k === "void") this.warn(`\`${s.name}\` tiene tipo \`void\`: la expresión no produce ningún valor`, s.init.span);
        }
        this.scope.vars.set(s.name, { type, mut: s.mut, kind: "var" });
        return init.k === "never" ? NEVER : VOID;
      }
      case "expr":
        return this.checkExpr(s.expr);
      case "fn":
        this.checkFnDecl(s.decl);
        return VOID;
      case "type":
        return VOID;
    }
  }

  private checkFnDecl(d: FnDecl) {
    const sig = this.signatures.get(d);
    if (!sig || sig.k !== "fn") return;
    this.withTypeParams(d.typeParams, () => {
      const saved = this.scope;
      this.scope = new Scope(saved);
      d.params.forEach((p, i) => {
        if (this.scope.vars.has(p.name)) this.error(`parámetro \`${p.name}\` repetido`, p.span);
        this.scope.vars.set(p.name, { type: sig.params[i], mut: false, kind: "var" });
      });
      this.fnStack.push({ ret: sig.ret, inferred: [] });
      this.loopStack.push(null);
      const body = this.checkBlock(d.body, sig.ret);
      this.loopStack.pop();
      this.fnStack.pop();
      this.scope = saved;

      if (sig.ret.k === "void" || this.tc.isAssignable(body, sig.ret)) return;
      if (d.body.tail) {
        this.expectAssignable(body, sig.ret, d.body.tail.span);
        return;
      }
      const last = d.body.stmts[d.body.stmts.length - 1];
      const hint = last && last.k === "expr" && last.semi
        ? "quitá el `;` de la última expresión: en uwu, la última expresión de un bloque es su valor"
        : "la última expresión del bloque (sin `;`) es el valor de retorno";
      this.error(`la función \`${d.name}\` debe devolver \`${print(sig.ret)}\`, pero su cuerpo no produce un valor`, d.span, hint);
    });
  }

  private checkBlock(b: Block, expected?: Type): Type {
    const saved = this.scope;
    this.scope = new Scope(saved);
    this.hoist(b.stmts);
    let diverges = false;
    for (const s of b.stmts) {
      if (diverges && s.k !== "fn" && s.k !== "type") {
        this.warn("código inalcanzable", s.span);
        diverges = false; // avisamos una sola vez
        this.checkStmt(s);
        diverges = true;
        continue;
      }
      if (this.checkStmt(s).k === "never") diverges = true;
    }
    let result = b.tail ? this.checkExpr(b.tail, expected) : VOID;
    if (diverges) result = NEVER;
    this.scope = saved;
    return result;
  }

  // ======================= expresiones =======================

  checkExpr(e: Expr, expected?: Type): Type {
    switch (e.k) {
      case "num":
      case "str":
      case "bool":
        return lit(e.value);
      case "ident": {
        const v = this.scope.lookup(e.name);
        if (!v) {
          const similar = this.scope.names().find((n) => levenshtein(n, e.name) <= 2);
          this.error(`no existe \`${e.name}\` en este ámbito`, e.span, similar ? `¿quisiste decir \`${similar}\`?` : undefined);
          return ERR;
        }
        return v.type;
      }
      case "array":
        return this.checkArray(e.items, expected);
      case "object":
        return this.checkObject(e, expected);
      case "unary": {
        const t = this.checkExpr(e.expr);
        if (e.op === "-") {
          if (!this.tc.isAssignable(t, NUMBER)) this.error(`no se puede negar un valor de tipo \`${print(t)}\``, e.span);
          return NUMBER;
        }
        if (!this.tc.isAssignable(t, BOOLEAN)) this.error(`\`!\` necesita un \`boolean\`, se encontró \`${print(t)}\``, e.span);
        return BOOLEAN;
      }
      case "binary":
        return this.checkBinary(e.op, e.left, e.right, e.span);
      case "range": {
        this.expectAssignable(this.checkExpr(e.start), NUMBER, e.start.span);
        this.expectAssignable(this.checkExpr(e.end), NUMBER, e.end.span);
        return RANGE;
      }
      case "assign":
        return this.checkAssign(e.op, e.target, e.value);
      case "call":
        return e.callee.k === "member"
          ? this.checkMethodCall(e.callee.object, e.callee.name, e.args, e.span)
          : this.checkCall(e.callee, e.typeArgs, e.args, e.span);
      case "member":
        return this.checkMember(e.object, e.name, e.span);
      case "index": {
        const t = this.tc.resolve(this.checkExpr(e.object));
        this.expectAssignable(this.checkExpr(e.index), NUMBER, e.index.span);
        if (t.k === "err") return ERR;
        if (t.k === "array") return t.elem;
        if (this.tc.isAssignable(t, STRING)) return STRING;
        this.error(`no se puede indexar un valor de tipo \`${print(t)}\``, e.object.span);
        return ERR;
      }
      case "block":
        return this.checkBlock(e.block, expected);
      case "if": {
        this.expectAssignable(this.checkExpr(e.cond), BOOLEAN, e.cond.span, "uwu no convierte valores a boolean automáticamente");
        const a = this.checkBlock(e.then, expected);
        if (!e.else_) return VOID;
        const b = this.checkExpr(e.else_, expected);
        return mkUnion([a, b]);
      }
      case "while": {
        this.expectAssignable(this.checkExpr(e.cond), BOOLEAN, e.cond.span);
        this.loopStack.push({ hasBreak: false });
        this.checkBlock(e.body);
        this.loopStack.pop();
        return VOID;
      }
      case "loop": {
        const ctx = { hasBreak: false };
        this.loopStack.push(ctx);
        this.checkBlock(e.body);
        this.loopStack.pop();
        return ctx.hasBreak ? VOID : NEVER;
      }
      case "for": {
        const it = this.tc.resolve(this.checkExpr(e.iter));
        let elem: Type = ERR;
        if (it.k === "array") elem = it.elem;
        else if (it.k === "range") elem = NUMBER;
        else if (this.tc.isAssignable(it, STRING)) elem = STRING;
        else if (it.k !== "err") this.error(`no se puede recorrer un valor de tipo \`${print(it)}\``, e.iter.span, "`for` recorre arreglos, strings y rangos como `0..10`");
        const saved = this.scope;
        this.scope = new Scope(saved);
        this.scope.vars.set(e.name, { type: elem, mut: false, kind: "var" });
        this.loopStack.push({ hasBreak: false });
        this.checkBlock(e.body);
        this.loopStack.pop();
        this.scope = saved;
        return VOID;
      }
      case "match":
        return this.checkMatch(e.subject, e.arms, e.span, expected);
      case "closure":
        return this.checkClosure(e, expected);
      case "return": {
        const ctx = this.fnStack[this.fnStack.length - 1];
        if (!ctx) {
          this.error("`return` fuera de una función", e.span);
          return NEVER;
        }
        const vt = e.value ? this.checkExpr(e.value, ctx.ret ?? undefined) : VOID;
        if (ctx.ret) {
          if (ctx.ret.k !== "void" || e.value) this.expectAssignable(vt, ctx.ret, e.value?.span ?? e.span);
        } else ctx.inferred.push(vt);
        return NEVER;
      }
      case "break":
      case "continue": {
        const loop = this.loopStack[this.loopStack.length - 1];
        if (!loop) this.error(`\`${e.k}\` fuera de un bucle`, e.span);
        else if (e.k === "break") loop.hasBreak = true;
        return NEVER;
      }
    }
  }

  private checkArray(items: Expr[], expected?: Type): Type {
    const exp = expected ? this.tc.resolve(expected) : null;
    if (exp && exp.k === "array") {
      for (const item of items) this.expectAssignable(this.checkExpr(item, exp.elem), exp.elem, item.span);
      return exp;
    }
    if (items.length === 0) return arr(NEVER);
    return arr(widen(mkUnion(items.map((i) => this.checkExpr(i)))));
  }

  private checkObject(e: Extract<Expr, { k: "object" }>, expected?: Type): Type {
    // Si esperamos una unión de objetos, cada campo recibe la unión de sus tipos posibles
    const expMembers = expected ? this.tc.flatten(expected, false).filter((m) => m.k === "object") : [];
    const fields = new Map<string, Type>();
    for (const f of e.fields) {
      if (fields.has(f.name)) this.error(`campo \`${f.name}\` repetido`, f.span);
      const fieldExp = expMembers.flatMap((m) => (m.k === "object" && m.fields.has(f.name) ? [m.fields.get(f.name)!] : []));
      fields.set(f.name, this.checkExpr(f.value, fieldExp.length ? mkUnion(fieldExp) : undefined));
    }
    return { k: "object", fields };
  }

  private checkBinary(op: string, left: Expr, right: Expr, span: Span): Type {
    // Los literales de objeto/arreglo se ensanchan: `{ a: 1 } == { a: 2 }` es una comparación válida
    const operand = (e: Expr) => {
      const t = this.checkExpr(e);
      return e.k === "object" || e.k === "array" ? widen(t) : t;
    };
    const l = operand(left);
    const r = operand(right);
    if (l.k === "err" || r.k === "err") {
      return ["==", "!=", "<", ">", "<=", ">=", "&&", "||"].includes(op) ? BOOLEAN : ERR;
    }
    const isNum = (t: Type) => this.tc.isAssignable(t, NUMBER);
    const isStr = (t: Type) => this.tc.isAssignable(t, STRING);
    const isBool = (t: Type) => this.tc.isAssignable(t, BOOLEAN);
    const isPrim = (t: Type) => isNum(t) || isStr(t) || isBool(t);
    const L = print(l);
    const R = print(r);
    switch (op) {
      case "+":
        if (isNum(l) && isNum(r)) return NUMBER;
        if ((isStr(l) && isPrim(r)) || (isStr(r) && isPrim(l))) return STRING;
        this.error(`no se puede sumar \`${L}\` con \`${R}\``, span);
        return ERR;
      case "-":
      case "*":
      case "/":
      case "%":
        if (isNum(l) && isNum(r)) return NUMBER;
        this.error(`\`${op}\` necesita dos \`number\`, se encontró \`${L}\` y \`${R}\``, span);
        return ERR;
      case "<":
      case ">":
      case "<=":
      case ">=":
        if (!((isNum(l) && isNum(r)) || (isStr(l) && isStr(r)))) {
          this.error(`no se puede comparar \`${L}\` con \`${R}\``, span, "sólo se comparan dos `number` o dos `string`");
        }
        return BOOLEAN;
      case "==":
      case "!=":
        if (!this.tc.overlaps(l, r)) {
          this.error(`esta comparación siempre da ${op === "==" ? "false" : "true"}: \`${L}\` y \`${R}\` no tienen valores en común`, span);
        }
        return BOOLEAN;
      case "&&":
      case "||":
        if (!isBool(l)) this.error(`\`${op}\` necesita un \`boolean\`, se encontró \`${L}\``, left.span);
        if (!isBool(r)) this.error(`\`${op}\` necesita un \`boolean\`, se encontró \`${R}\``, right.span);
        return BOOLEAN;
    }
    return ERR;
  }

  /** Para `a.b[0].c = x` devuelve `a`, que es lo que tiene que ser `mut`. */
  private rootIdent(e: Expr): Extract<Expr, { k: "ident" }> | null {
    if (e.k === "ident") return e;
    if (e.k === "member" || e.k === "index") return this.rootIdent(e.object);
    return null;
  }

  private checkMutable(target: Expr, span: Span) {
    const root = this.rootIdent(target);
    if (!root) return;
    const v = this.scope.lookup(root.name);
    if (!v) return;
    if (v.kind !== "var") {
      this.error(`no se puede modificar \`${root.name}\`: es una función`, span);
    } else if (!v.mut) {
      this.error(`no se puede modificar \`${root.name}\` porque es inmutable`, span, `declarala con \`let mut ${root.name}\``);
    }
  }

  private checkAssign(op: string, target: Expr, value: Expr): Type {
    const tt = this.checkExpr(target);
    this.checkMutable(target, target.span);
    if (op === "=") {
      this.expectAssignable(this.checkExpr(value, tt), tt, value.span);
      return VOID;
    }
    const vt = this.checkExpr(value);
    const ok = op === "+="
      ? (this.tc.isAssignable(tt, NUMBER) && this.tc.isAssignable(vt, NUMBER)) ||
        (this.tc.isAssignable(tt, STRING) && [NUMBER, STRING, BOOLEAN].some((p) => this.tc.isAssignable(vt, p)))
      : this.tc.isAssignable(tt, NUMBER) && this.tc.isAssignable(vt, NUMBER);
    if (!ok && tt.k !== "err" && vt.k !== "err") {
      this.error(`no se puede aplicar \`${op}\` a \`${print(tt)}\` y \`${print(vt)}\``, value.span);
    }
    return VOID;
  }

  private checkMember(object: Expr, name: string, span: Span): Type {
    const ot = this.checkExpr(object);
    const members = this.tc.flatten(ot, false);
    if (members.some((m) => m.k === "err")) return ERR;
    if (members.every((m) => m.k === "object" && m.fields.has(name))) {
      return mkUnion(members.map((m) => (m.k === "object" ? m.fields.get(name)! : ERR)));
    }
    if (members.some((m) => m.k === "object" && m.fields.has(name))) {
      this.error(`no todas las variantes de \`${print(ot)}\` tienen el campo \`${name}\``, span,
        "usá `match` para distinguir las variantes antes de acceder al campo");
      return ERR;
    }
    if (this.methodType(ot, name)) {
      this.error(`\`${name}\` es un método, no un campo`, span, `llamalo con paréntesis: \`.${name}()\``);
      return ERR;
    }
    const obj = members.length === 1 && members[0].k === "object" ? members[0] : null;
    const hint = obj && obj.k === "object" && obj.fields.size ? `campos disponibles: ${[...obj.fields.keys()].map((f) => `\`${f}\``).join(", ")}` : undefined;
    this.error(`el tipo \`${print(ot)}\` no tiene el campo \`${name}\``, span, hint);
    return ERR;
  }

  // ---------- llamadas ----------

  private checkCall(callee: Expr, typeArgs: TypeNode[], args: Expr[], span: Span): Type {
    const ct = this.checkExpr(callee);
    const ft = this.tc.resolve(ct);
    if (ft.k === "err") {
      args.forEach((a) => this.checkExpr(a));
      return ERR;
    }
    if (ft.k !== "fn") {
      this.error(`\`${print(ct)}\` no es una función`, callee.span);
      args.forEach((a) => this.checkExpr(a));
      return ERR;
    }
    const name = callee.k === "ident" ? callee.name : "la función";
    return this.applyFn(ft, typeArgs, args, span, name);
  }

  private applyFn(ft: Extract<Type, { k: "fn" }>, typeArgNodes: TypeNode[], args: Expr[], span: Span, name: string): Type {
    if (ft.variadic) {
      args.forEach((a) => this.checkExpr(a));
      return ft.ret;
    }
    // Renombramos los parámetros de tipo para que no choquen con los del llamador
    let params = ft.params;
    let ret = ft.ret;
    const free = new Set<string>();
    if (ft.typeParams.length) {
      const fresh = new Map<string, Type>();
      for (const tp of ft.typeParams) {
        const n = `${tp}'${this.freshCounter++}`;
        free.add(n);
        fresh.set(tp, param(n));
      }
      params = params.map((p) => subst(p, fresh));
      ret = subst(ret, fresh);
    }
    if (args.length !== params.length) {
      this.error(`\`${name}\` espera ${params.length} argumento(s), pero recibió ${args.length}`, span);
    }
    const bind = new Map<string, Type>();
    if (typeArgNodes.length) {
      if (typeArgNodes.length !== free.size) {
        this.error(`\`${name}\` tiene ${free.size} parámetro(s) de tipo, se indicaron ${typeArgNodes.length}`, span);
      } else [...free].forEach((n, i) => bind.set(n, this.toType(typeArgNodes[i])));
    }
    const argTypes = args.map((a, i) => {
      const pt = params[i];
      if (!pt) return this.checkExpr(a);
      const ps = subst(pt, bind);
      const hasFree = containsParam(ps, (n) => free.has(n));
      const at = this.checkExpr(a, hasFree && a.k !== "closure" ? undefined : ps);
      if (free.size) this.unify(pt, at, bind, free, 0);
      return at;
    });
    for (const n of free) {
      if (!bind.has(n)) {
        this.error(`no se pudo inferir el parámetro de tipo \`${n.split("'")[0]}\` de \`${name}\``, span,
          `indicalo explícitamente: \`${name}::<number>(...)\``);
        bind.set(n, ERR);
      }
    }
    args.forEach((a, i) => {
      if (params[i]) this.expectAssignable(argTypes[i], subst(params[i], bind), a.span);
    });
    return subst(ret, bind);
  }

  /** Inferencia de genéricos: encuentra T comparando el tipo del parámetro con el del argumento. */
  private unify(p: Type, a: Type, bind: Map<string, Type>, free: Set<string>, depth: number): void {
    if (depth > 32 || a.k === "err" || a.k === "never") return;
    if (p.k === "param" && free.has(p.name)) {
      const w = widen(a);
      const cur = bind.get(p.name);
      if (!cur) bind.set(p.name, w);
      else if (!this.tc.isAssignable(w, cur)) {
        bind.set(p.name, this.tc.isAssignable(cur, w) ? w : mkUnion([cur, w]));
      }
      return;
    }
    if (!containsParam(p, (n) => free.has(n))) return;
    if (p.k === "ref" && a.k === "ref" && p.name === a.name) {
      p.args.forEach((x, i) => a.args[i] && this.unify(x, a.args[i], bind, free, depth + 1));
      return;
    }
    const pr = this.tc.resolve(p);
    const ar = this.tc.resolve(a);
    if (pr.k === "array" && ar.k === "array") return this.unify(pr.elem, ar.elem, bind, free, depth + 1);
    if (pr.k === "object" && ar.k === "object") {
      for (const [n, ft] of pr.fields) {
        const af = ar.fields.get(n);
        if (af) this.unify(ft, af, bind, free, depth + 1);
      }
      return;
    }
    if (pr.k === "fn" && ar.k === "fn") {
      pr.params.forEach((x, i) => ar.params[i] && this.unify(x, ar.params[i], bind, free, depth + 1));
      this.unify(pr.ret, ar.ret, bind, free, depth + 1);
      return;
    }
    if (pr.k === "union") {
      if (ar.k === "union") {
        ar.members.forEach((m) => this.unify(pr, m, bind, free, depth + 1));
        return;
      }
      const errMap = new Map([...free].map((n) => [n, ERR] as [string, Type]));
      const concrete = pr.members.filter((m) => !(m.k === "param" && free.has(m.name)));
      const matching = concrete.filter((m) => this.tc.isAssignable(ar, subst(m, errMap)));
      if (matching.length) matching.forEach((m) => this.unify(m, ar, bind, free, depth + 1));
      else {
        const bare = pr.members.find((m) => m.k === "param" && free.has(m.name));
        if (bare) this.unify(bare, ar, bind, free, depth + 1);
      }
    }
  }

  // ---------- métodos ----------

  private methodType(recv: Type, name: string): Extract<Type, { k: "fn" }> | null {
    const base = this.tc.resolve(widen(mkUnion(this.tc.flatten(recv, false))));
    const U = param("U");
    const A = param("A");
    let table: Record<string, Type> = {};
    if (base.k === "array") {
      const E = base.elem;
      const pred = fn([E], BOOLEAN);
      table = {
        len: fn([], NUMBER), push: fn([E], VOID), pop: fn([], E), contains: fn([E], BOOLEAN),
        reverse: fn([], arr(E)), join: fn([STRING], STRING),
        map: fn([fn([E], U)], arr(U), ["U"]), filter: fn([pred], arr(E)),
        any: fn([pred], BOOLEAN), all: fn([pred], BOOLEAN),
        fold: fn([A, fn([A, E], A)], A, ["A"]),
      };
      if (this.tc.isAssignable(E, NUMBER)) table.sum = fn([], NUMBER);
    } else if (base.k === "string") {
      table = {
        len: fn([], NUMBER), upper: fn([], STRING), lower: fn([], STRING), trim: fn([], STRING),
        contains: fn([STRING], BOOLEAN), starts_with: fn([STRING], BOOLEAN), ends_with: fn([STRING], BOOLEAN),
        split: fn([STRING], arr(STRING)), chars: fn([], arr(STRING)), repeat: fn([NUMBER], STRING),
      };
    } else if (base.k === "number") {
      table = {
        to_string: fn([], STRING), abs: fn([], NUMBER), floor: fn([], NUMBER), ceil: fn([], NUMBER),
        round: fn([], NUMBER), sqrt: fn([], NUMBER), pow: fn([NUMBER], NUMBER),
        min: fn([NUMBER], NUMBER), max: fn([NUMBER], NUMBER),
      };
    } else if (base.k === "boolean") {
      table = { to_string: fn([], STRING) };
    } else if (base.k === "range") {
      table = { to_array: fn([], arr(NUMBER)) };
    }
    const m = table[name];
    return m && m.k === "fn" ? m : null;
  }

  private checkMethodCall(object: Expr, name: string, args: Expr[], span: Span): Type {
    const ot = this.checkExpr(object);
    const members = this.tc.flatten(ot, false);
    if (members.some((m) => m.k === "err")) {
      args.forEach((a) => this.checkExpr(a));
      return ERR;
    }
    // Un campo que guarda una función: obj.callback(1)
    if (members.every((m) => m.k === "object" && m.fields.has(name))) {
      const ft = this.tc.resolve(mkUnion(members.map((m) => (m.k === "object" ? m.fields.get(name)! : ERR))));
      if (ft.k === "fn") return this.applyFn(ft, [], args, span, name);
    }
    const method = this.methodType(ot, name);
    if (!method) {
      this.error(`el tipo \`${print(ot)}\` no tiene el método \`${name}\``, span);
      args.forEach((a) => this.checkExpr(a));
      return ERR;
    }
    if (MUTATING_METHODS.has(name)) this.checkMutable(object, span);
    return this.applyFn(method, [], args, span, `.${name}`);
  }

  // ---------- closures ----------

  private checkClosure(e: Extract<Expr, { k: "closure" }>, expected?: Type): Type {
    const exp = expected ? this.tc.resolve(expected) : null;
    const expFn = exp && exp.k === "fn" ? exp : null;
    const usable = (t: Type | undefined) => t && !containsParam(t, isFresh) ? t : undefined;
    const params = e.params.map((p, i) => {
      if (p.type) return this.toType(p.type);
      const fromCtx = usable(expFn?.params[i]);
      if (fromCtx) return fromCtx;
      this.error(`no se puede inferir el tipo del parámetro \`${p.name}\``, p.span, `anotalo: \`|${p.name}: number| ...\``);
      return ERR;
    });
    const expectedRet = usable(expFn?.ret) ?? null;
    const saved = this.scope;
    this.scope = new Scope(saved);
    e.params.forEach((p, i) => this.scope.vars.set(p.name, { type: params[i], mut: false, kind: "var" }));
    const ctx: FnCtx = { ret: expectedRet, inferred: [] };
    this.fnStack.push(ctx);
    this.loopStack.push(null);
    const body = this.checkExpr(e.body, expectedRet ?? undefined);
    this.loopStack.pop();
    this.fnStack.pop();
    this.scope = saved;
    let ret: Type;
    if (expectedRet) {
      if (expectedRet.k !== "void") this.expectAssignable(body, expectedRet, e.body.span);
      ret = expectedRet;
    } else ret = mkUnion([body, ...ctx.inferred]);
    return fn(params, ret);
  }

  // ======================= match =======================

  private checkMatch(subject: Expr, arms: Arm[], span: Span, expected?: Type): Type {
    const st = this.checkExpr(subject);
    let remaining = this.tc.flatten(st);
    const results: Type[] = [];
    for (const arm of arms) {
      this.prepPattern(arm.pattern);
      if (remaining.length === 0 && !remaining.some((m) => m.k === "err")) {
        this.warn("este brazo nunca se ejecuta: los anteriores ya cubren todos los casos", arm.pattern.span);
      }
      const saved = this.scope;
      this.scope = new Scope(saved);
      this.bindPattern(arm.pattern, st);
      if (arm.guard) this.expectAssignable(this.checkExpr(arm.guard), BOOLEAN, arm.guard.span);
      results.push(this.checkExpr(arm.body, expected));
      this.scope = saved;
      if (!arm.guard) remaining = remaining.flatMap((m) => this.subtract(arm.pattern, m));
    }
    if (remaining.length && !remaining.some((m) => m.k === "err")) {
      const missing = remaining.map((m) => `\`${print(m)}\``).join(", ");
      this.error(`\`match\` no exhaustivo: falta cubrir ${missing}`, subject.span,
        "agregá un brazo para esos casos, o uno comodín: `_ => ...`");
    }
    return mkUnion(results);
  }

  private prepPattern(p: Pattern) {
    if ((p.k === "wild" || p.k === "bind") && p.type) this.patternTypes.set(p, this.toType(p.type));
    if (p.k === "object") p.fields.forEach((f) => this.prepPattern(f.pattern));
  }

  private patType(p: Pattern): Type | null {
    return p.k === "wild" || p.k === "bind" ? this.patternTypes.get(p) ?? null : null;
  }

  /** ¿El patrón puede coincidir con algún valor de `m`? (m ya está resuelto y no es unión) */
  private mightMatch(p: Pattern, m: Type): boolean {
    if (m.k === "err") return true;
    switch (p.k) {
      case "wild":
      case "bind": {
        const t = this.patType(p);
        return !t || this.tc.flatten(t).some((x) => this.tc.isAssignable(m, x) || this.tc.isAssignable(x, m));
      }
      case "lit":
        return m.k === "lit" ? m.value === p.value : m.k === typeof p.value;
      case "object":
        return m.k === "object" && p.fields.every((f) =>
          m.fields.has(f.name) && this.tc.flatten(m.fields.get(f.name)!).some((x) => this.mightMatch(f.pattern, x)));
    }
  }

  /** ¿El patrón coincide con TODOS los valores de `m`? */
  private covers(p: Pattern, m: Type): boolean {
    if (m.k === "err") return true;
    switch (p.k) {
      case "wild":
      case "bind": {
        const t = this.patType(p);
        return !t || this.tc.isAssignable(m, t);
      }
      case "lit":
        return m.k === "lit" && m.value === p.value;
      case "object":
        return m.k === "object" && p.fields.every((f) =>
          m.fields.has(f.name) && this.tc.flatten(m.fields.get(f.name)!).every((x) => this.covers(f.pattern, x)));
    }
  }

  /** Lo que queda de `m` después de quitar lo que cubre `p` (para el chequeo de exhaustividad). */
  private subtract(p: Pattern, m: Type): Type[] {
    if (this.covers(p, m)) return [];
    if (!this.mightMatch(p, m) || p.k !== "object" || m.k !== "object") return [m];
    const withField = (name: string, t: Type): Type => {
      const fields = new Map(m.fields);
      fields.set(name, t);
      return { k: "object", fields };
    };
    // Partimos el objeto por cada campo que sea una unión
    for (const f of p.fields) {
      const parts = this.tc.flatten(m.fields.get(f.name)!);
      if (parts.length > 1) return parts.flatMap((x) => this.subtract(p, withField(f.name, x)));
    }
    const uncovered = p.fields.filter((f) => !this.covers(f.pattern, this.tc.resolve(m.fields.get(f.name)!)));
    if (uncovered.length === 1) {
      const f = uncovered[0];
      return this.subtract(f.pattern, this.tc.resolve(m.fields.get(f.name)!)).map((x) => withField(f.name, x));
    }
    return [m];
  }

  /** Declara las variables del patrón con el tipo más preciso posible (narrowing). */
  private bindPattern(p: Pattern, t: Type) {
    const members = this.tc.flatten(t).filter((m) => this.mightMatch(p, m));
    if (members.length === 0) {
      this.error(`este patrón nunca coincide con un valor de tipo \`${print(t)}\``, p.span);
    }
    switch (p.k) {
      case "wild":
        return;
      case "bind":
        this.scope.vars.set(p.name, { type: this.patType(p) ?? t, mut: false, kind: "var" });
        return;
      case "lit":
        return;
      case "object":
        for (const f of p.fields) {
          const fieldTypes = members.flatMap((m) => (m.k === "object" && m.fields.has(f.name) ? [m.fields.get(f.name)!] : []));
          this.bindPattern(f.pattern, fieldTypes.length ? mkUnion(fieldTypes) : ERR);
        }
    }
  }
}

function levenshtein(a: string, b: string): number {
  if (Math.abs(a.length - b.length) > 2) return 99;
  const dp = Array.from({ length: a.length + 1 }, (_, i) => [i, ...Array(b.length).fill(0)]);
  for (let j = 1; j <= b.length; j++) dp[0][j] = j;
  for (let i = 1; i <= a.length; i++) {
    for (let j = 1; j <= b.length; j++) {
      dp[i][j] = Math.min(dp[i - 1][j] + 1, dp[i][j - 1] + 1, dp[i - 1][j - 1] + (a[i - 1] === b[j - 1] ? 0 : 1));
    }
  }
  return dp[a.length][b.length];
}
