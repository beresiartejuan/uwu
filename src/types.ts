/**
 * Sistema de tipos de uwu: el de TypeScript, pero sólido.
 * - primitivos: number, string, boolean, void, never
 * - tipos literales: "circulo", 42, true
 * - uniones: A | B
 * - objetos estructurales: { x: number, y: number }
 * - arreglos: T[]
 * - funciones: (a: number) => string
 * - alias genéricos y recursivos: type Lista<T> = { tipo: "vacia" } | { tipo: "nodo", valor: T, resto: Lista<T> };
 */

export type Type =
  | { k: "number" }
  | { k: "string" }
  | { k: "boolean" }
  | { k: "void" }
  | { k: "never" }
  | { k: "range" }
  | { k: "err" } // tipo de recuperación tras un error: compatible con todo
  | { k: "lit"; value: string | number | boolean }
  | { k: "array"; elem: Type }
  | { k: "object"; fields: Map<string, Type> }
  | { k: "union"; members: Type[] }
  | { k: "fn"; params: Type[]; ret: Type; typeParams: string[]; variadic?: boolean }
  | { k: "param"; name: string }
  | { k: "ref"; name: string; args: Type[] };

export const NUMBER: Type = { k: "number" };
export const STRING: Type = { k: "string" };
export const BOOLEAN: Type = { k: "boolean" };
export const VOID: Type = { k: "void" };
export const NEVER: Type = { k: "never" };
export const RANGE: Type = { k: "range" };
export const ERR: Type = { k: "err" };

export const lit = (value: string | number | boolean): Type => ({ k: "lit", value });
export const arr = (elem: Type): Type => ({ k: "array", elem });
export const fn = (params: Type[], ret: Type, typeParams: string[] = []): Type => ({ k: "fn", params, ret, typeParams });
export const param = (name: string): Type => ({ k: "param", name });

export interface Alias {
  params: string[];
  body: Type;
}

export function print(t: Type): string {
  switch (t.k) {
    case "lit":
      return typeof t.value === "string" ? JSON.stringify(t.value) : String(t.value);
    case "array": {
      const inner = print(t.elem);
      return t.elem.k === "union" || t.elem.k === "fn" ? `(${inner})[]` : `${inner}[]`;
    }
    case "object":
      if (t.fields.size === 0) return "{}";
      return `{ ${[...t.fields].map(([n, ft]) => `${n}: ${print(ft)}`).join(", ")} }`;
    case "union":
      return t.members.map(print).join(" | ");
    case "fn": {
      const tp = t.typeParams.length ? `<${t.typeParams.map(cleanName).join(", ")}>` : "";
      return `${tp}(${t.params.map(print).join(", ")}) => ${print(t.ret)}`;
    }
    case "param":
      return cleanName(t.name);
    case "ref":
      return t.args.length ? `${t.name}<${t.args.map(print).join(", ")}>` : t.name;
    case "err":
      return "{desconocido}";
    default:
      return t.k;
  }
}

// Los parámetros de tipo se renombran internamente (`T'3`) para evitar colisiones.
const cleanName = (n: string) => n.split("'")[0];
export const isFresh = (n: string) => n.includes("'");

export function subst(t: Type, map: Map<string, Type>): Type {
  if (map.size === 0) return t;
  switch (t.k) {
    case "param":
      return map.get(t.name) ?? t;
    case "array":
      return arr(subst(t.elem, map));
    case "object":
      return { k: "object", fields: new Map([...t.fields].map(([n, ft]) => [n, subst(ft, map)])) };
    case "union":
      return mkUnion(t.members.map((m) => subst(m, map)));
    case "fn":
      return { ...t, params: t.params.map((p) => subst(p, map)), ret: subst(t.ret, map) };
    case "ref":
      return { k: "ref", name: t.name, args: t.args.map((a) => subst(a, map)) };
    default:
      return t;
  }
}

export function containsParam(t: Type, pred: (name: string) => boolean): boolean {
  switch (t.k) {
    case "param":
      return pred(t.name);
    case "array":
      return containsParam(t.elem, pred);
    case "object":
      return [...t.fields.values()].some((f) => containsParam(f, pred));
    case "union":
      return t.members.some((m) => containsParam(m, pred));
    case "fn":
      return t.params.some((p) => containsParam(p, pred)) || containsParam(t.ret, pred);
    case "ref":
      return t.args.some((a) => containsParam(a, pred));
    default:
      return false;
  }
}

const baseOf = (v: string | number | boolean) => typeof v as "string" | "number" | "boolean";

/** Construye una unión normalizada: aplana, quita `never`, deduplica y absorbe literales. */
export function mkUnion(types: Type[]): Type {
  const flat: Type[] = [];
  const push = (t: Type) => {
    if (t.k === "union") t.members.forEach(push);
    else if (t.k !== "never") flat.push(t);
  };
  types.forEach(push);
  if (flat.some((t) => t.k === "err")) return ERR;
  const seen = new Set<string>();
  let out = flat.filter((t) => {
    const key = print(t);
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
  const has = (k: string) => out.some((t) => t.k === k);
  out = out.filter((t) => !(t.k === "lit" && has(baseOf(t.value))));
  const hasTrue = out.some((t) => t.k === "lit" && t.value === true);
  const hasFalse = out.some((t) => t.k === "lit" && t.value === false);
  if (hasTrue && hasFalse) {
    out = out.filter((t) => !(t.k === "lit" && typeof t.value === "boolean"));
    out.push(BOOLEAN);
  }
  if (out.length === 0) return NEVER;
  if (out.length === 1) return out[0];
  return { k: "union", members: out };
}

/** `let x = 5` → number (no el literal 5), igual que TS en contextos mutables. */
export function widen(t: Type): Type {
  switch (t.k) {
    case "lit":
      return { k: baseOf(t.value) };
    case "array":
      return arr(widen(t.elem));
    case "object":
      return { k: "object", fields: new Map([...t.fields].map(([n, ft]) => [n, widen(ft)])) };
    case "union":
      return mkUnion(t.members.map(widen));
    default:
      return t;
  }
}

export function hasNeverArray(t: Type): boolean {
  if (t.k === "array") return t.elem.k === "never" || hasNeverArray(t.elem);
  if (t.k === "object") return [...t.fields.values()].some(hasNeverArray);
  if (t.k === "union") return t.members.some(hasNeverArray);
  return false;
}

export class TypeCtx {
  aliases = new Map<string, Alias>();

  /** Expande alias (`Opcion<number>`) hasta llegar a un tipo concreto. */
  resolve(t: Type): Type {
    let depth = 0;
    while (t.k === "ref") {
      const alias = this.aliases.get(t.name);
      if (!alias || depth++ > 64) return ERR;
      const args = t.args;
      t = alias.params.length
        ? subst(alias.body, new Map(alias.params.map((p, i) => [p, args[i] ?? ERR])))
        : alias.body;
    }
    return t;
  }

  /** Miembros de una unión, resueltos. `boolean` se expande a `true | false` si se pide. */
  flatten(t: Type, expandBool = true, depth = 0): Type[] {
    const r = this.resolve(t);
    if (depth > 64) return [ERR];
    if (r.k === "union") return r.members.flatMap((m) => this.flatten(m, expandBool, depth + 1));
    if (r.k === "boolean" && expandBool) return [lit(true), lit(false)];
    return [r];
  }

  /** ¿Un valor de tipo `src` puede usarse donde se espera `dst`? */
  isAssignable(src: Type, dst: Type, seen = new Set<string>()): boolean {
    if (src.k === "err" || dst.k === "err" || src.k === "never") return true;
    if (src.k === "ref" || dst.k === "ref") {
      if (src.k === "ref" && dst.k === "ref" && src.name === dst.name && src.args.length === dst.args.length) {
        const dargs = dst.args;
        if (src.args.every((a, i) => this.isAssignable(a, dargs[i], seen))) return true;
      }
      // tipos recursivos: asumimos verdadero mientras lo estamos comprobando
      const key = `${print(src)} <: ${print(dst)}`;
      if (seen.has(key)) return true;
      seen.add(key);
      src = this.resolve(src);
      dst = this.resolve(dst);
      if (src.k === "err" || dst.k === "err" || src.k === "never") return true;
    }
    if (src.k === "union") return src.members.every((m) => this.isAssignable(m, dst, seen));
    if (dst.k === "union") {
      if (dst.members.some((m) => this.isAssignable(src, m, seen))) return true;
      if (src.k === "boolean") return this.isAssignable(lit(true), dst, seen) && this.isAssignable(lit(false), dst, seen);
      return false;
    }
    switch (dst.k) {
      case "number":
      case "string":
      case "boolean":
        return src.k === dst.k || (src.k === "lit" && baseOf(src.value) === dst.k);
      case "lit":
        return src.k === "lit" && src.value === dst.value;
      case "void":
      case "range":
        return src.k === dst.k;
      case "array":
        return src.k === "array" && this.isAssignable(src.elem, dst.elem, seen);
      case "object":
        if (src.k !== "object") return false;
        for (const [name, ft] of dst.fields) {
          const sf = src.fields.get(name);
          if (!sf || !this.isAssignable(sf, ft, seen)) return false;
        }
        return true;
      case "fn":
        if (src.k !== "fn" || src.params.length > dst.params.length) return false;
        return src.params.every((p, i) => this.isAssignable(dst.params[i], p, seen)) &&
          (dst.ret.k === "void" || this.isAssignable(src.ret, dst.ret, seen));
      case "param":
        return src.k === "param" && src.name === dst.name;
      default:
        return false;
    }
  }

  /** ¿Existe algún valor que pertenezca a ambos tipos? Se usa para `==`. */
  overlaps(a: Type, b: Type): boolean {
    const fa = this.flatten(a);
    const fb = this.flatten(b);
    return fa.some((x) => fb.some((y) => this.isAssignable(x, y) || this.isAssignable(y, x)));
  }
}
