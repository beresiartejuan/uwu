import type { Span } from "./lexer";

// ---------- Tipos (tal como se escriben en el código) ----------

export type TypeNode =
  | { k: "name"; name: string; args: TypeNode[]; span: Span }
  | { k: "lit"; value: string | number | boolean; span: Span }
  | { k: "array"; elem: TypeNode; span: Span }
  | { k: "object"; fields: { name: string; type: TypeNode }[]; span: Span }
  | { k: "union"; members: TypeNode[]; span: Span }
  | { k: "fn"; params: TypeNode[]; ret: TypeNode; span: Span };

// ---------- Expresiones ----------

export type Expr =
  | { k: "num"; value: number; span: Span }
  | { k: "str"; value: string; span: Span }
  | { k: "bool"; value: boolean; span: Span }
  | { k: "ident"; name: string; span: Span }
  | { k: "array"; items: Expr[]; span: Span }
  | { k: "object"; fields: { name: string; value: Expr; span: Span }[]; span: Span }
  | { k: "unary"; op: "-" | "!"; expr: Expr; span: Span }
  | { k: "binary"; op: string; left: Expr; right: Expr; span: Span }
  | { k: "range"; start: Expr; end: Expr; span: Span }
  | { k: "assign"; op: string; target: Expr; value: Expr; span: Span }
  | { k: "call"; callee: Expr; typeArgs: TypeNode[]; args: Expr[]; span: Span }
  | { k: "member"; object: Expr; name: string; span: Span }
  | { k: "index"; object: Expr; index: Expr; span: Span }
  | { k: "block"; block: Block; span: Span }
  | { k: "if"; cond: Expr; then: Block; else_: Expr | null; span: Span }
  | { k: "while"; cond: Expr; body: Block; span: Span }
  | { k: "loop"; body: Block; span: Span }
  | { k: "for"; name: string; iter: Expr; body: Block; span: Span }
  | { k: "match"; subject: Expr; arms: Arm[]; span: Span }
  | { k: "closure"; params: Param[]; body: Expr; span: Span }
  | { k: "return"; value: Expr | null; span: Span }
  | { k: "break"; span: Span }
  | { k: "continue"; span: Span };

export interface Block {
  stmts: Stmt[];
  tail: Expr | null; // última expresión sin `;` → valor del bloque
  span: Span;
}

export type Stmt =
  | { k: "let"; mut: boolean; name: string; type: TypeNode | null; init: Expr; span: Span }
  | { k: "expr"; expr: Expr; semi: boolean; span: Span }
  | { k: "fn"; decl: FnDecl; span: Span }
  | { k: "type"; decl: TypeDecl; span: Span };

export interface Param {
  name: string;
  type: TypeNode | null;
  span: Span;
}

export interface FnDecl {
  name: string;
  typeParams: string[];
  params: Param[];
  ret: TypeNode | null;
  body: Block;
  span: Span;
}

export interface TypeDecl {
  name: string;
  params: string[];
  type: TypeNode;
  span: Span;
}

// ---------- Patrones (match) ----------

export type Pattern =
  | { k: "wild"; type: TypeNode | null; span: Span }
  | { k: "bind"; name: string; type: TypeNode | null; span: Span }
  | { k: "lit"; value: string | number | boolean; span: Span }
  | { k: "object"; fields: { name: string; pattern: Pattern }[]; span: Span };

export interface Arm {
  pattern: Pattern;
  guard: Expr | null;
  body: Expr;
  span: Span;
}

export interface Program {
  stmts: Stmt[];
}
