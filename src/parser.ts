import type { Arm, Block, Expr, FnDecl, Param, Pattern, Program, Stmt, TypeDecl, TypeNode } from "./ast";
import { UwuError } from "./diagnostics";
import type { Span, Token } from "./lexer";

const BINARY_PRECEDENCE: Record<string, number> = {
  "..": 1,
  "||": 2,
  "&&": 3,
  "==": 4, "!=": 4, "<": 4, ">": 4, "<=": 4, ">=": 4,
  "+": 5, "-": 5,
  "*": 6, "/": 6, "%": 6,
};

const ASSIGN_OPS = new Set(["=", "+=", "-=", "*=", "/="]);
const BLOCK_LIKE = new Set(["block", "if", "while", "loop", "for", "match"]);
const TYPE_NAMES = new Set(["number", "string", "boolean", "void", "never"]);

export function join(a: Span, b: Span): Span {
  return { start: a.start, end: b.end, line: a.line, col: a.col };
}

function describe(t: Token): string {
  if (t.kind === "eof") return "el final del archivo";
  if (t.kind === "str") return `\`"${t.value}"\``;
  return `\`${t.value}\``;
}

export class Parser {
  private pos = 0;

  constructor(private tokens: Token[]) {}

  // ---------- utilidades ----------

  private peek(offset = 0): Token {
    return this.tokens[Math.min(this.pos + offset, this.tokens.length - 1)];
  }

  private at(value: string, offset = 0): boolean {
    const t = this.peek(offset);
    return (t.kind === "op" || t.kind === "kw") && t.value === value;
  }

  private next(): Token {
    const t = this.peek();
    if (t.kind !== "eof") this.pos++;
    return t;
  }

  private eat(value: string): Token | null {
    return this.at(value) ? this.next() : null;
  }

  private expect(value: string, context = ""): Token {
    if (this.at(value)) return this.next();
    return this.fail(`se esperaba \`${value}\`${context ? " " + context : ""}, se encontró ${describe(this.peek())}`);
  }

  private ident(what = "un nombre"): Token {
    const t = this.peek();
    if (t.kind === "ident") return this.next();
    const hint = t.kind === "kw" ? `\`${t.value}\` es una palabra reservada` : undefined;
    return this.fail(`se esperaba ${what}, se encontró ${describe(t)}`, t.span, hint);
  }

  private fail(message: string, span: Span = this.peek().span, hint?: string): never {
    throw new UwuError([{ message, span, hint }]);
  }

  private prev(): Span {
    return this.tokens[Math.max(0, this.pos - 1)].span;
  }

  // ---------- programa y sentencias ----------

  parseProgram(): Program {
    const { stmts, tail } = this.parseStmtList("eof");
    if (tail) stmts.push({ k: "expr", expr: tail, semi: false, span: tail.span });
    return { stmts };
  }

  private isEnd(end: string): boolean {
    return this.peek().kind === "eof" || (end !== "eof" && this.at(end));
  }

  private parseStmtList(end: string): { stmts: Stmt[]; tail: Expr | null } {
    const stmts: Stmt[] = [];
    let tail: Expr | null = null;
    while (!this.isEnd(end)) {
      if (this.eat(";")) continue;
      if (this.at("let")) {
        stmts.push(this.parseLet());
        continue;
      }
      if (this.at("fn") && this.peek(1).kind === "ident") {
        const decl = this.parseFn();
        stmts.push({ k: "fn", decl, span: decl.span });
        continue;
      }
      if (this.at("type")) {
        const decl = this.parseTypeDecl();
        stmts.push({ k: "type", decl, span: decl.span });
        continue;
      }
      // Igual que en Rust: un `if`/`match`/bloque al inicio de una sentencia no necesita `;`
      const blockLike = (this.at("{") && !this.isObjectLiteralStart()) ||
        ["if", "while", "loop", "for", "match"].some((v) => this.at(v));
      const expr = blockLike ? this.parseBlockLike() : this.parseExpr();
      if (this.eat(";")) {
        stmts.push({ k: "expr", expr, semi: true, span: expr.span });
        continue;
      }
      if (this.isEnd(end)) {
        tail = expr;
        break;
      }
      if (blockLike) {
        stmts.push({ k: "expr", expr, semi: false, span: expr.span });
        continue;
      }
      this.fail(`se esperaba \`;\`, se encontró ${describe(this.peek())}`, this.prev(), "agregá `;` al final de la sentencia");
    }
    return { stmts, tail };
  }

  private parseBlock(): Block {
    const open = this.expect("{");
    const { stmts, tail } = this.parseStmtList("}");
    const close = this.expect("}", "para cerrar el bloque");
    return { stmts, tail, span: join(open.span, close.span) };
  }

  private parseLet(): Stmt {
    const start = this.expect("let").span;
    const mut = !!this.eat("mut");
    const name = this.ident("el nombre de la variable");
    const type = this.eat(":") ? this.parseType() : null;
    this.expect("=", `después de \`${name.value}\``);
    const init = this.parseExpr();
    this.expect(";", "al final del `let`");
    return { k: "let", mut, name: name.value, type, init, span: join(start, init.span) };
  }

  private parseTypeParams(): string[] {
    const params: string[] = [];
    if (this.eat("<")) {
      do params.push(this.ident("un parámetro de tipo").value);
      while (this.eat(","));
      this.expect(">");
    }
    return params;
  }

  private parseFn(): FnDecl {
    const start = this.expect("fn").span;
    const name = this.ident("el nombre de la función");
    const typeParams = this.parseTypeParams();
    this.expect("(");
    const params: Param[] = [];
    while (!this.at(")")) {
      const p = this.ident("el nombre de un parámetro");
      if (!this.at(":")) {
        this.fail(`falta el tipo del parámetro \`${p.value}\``, p.span, `escribilo así: \`${p.value}: number\``);
      }
      this.next();
      params.push({ name: p.value, type: this.parseType(), span: p.span });
      if (!this.eat(",")) break;
    }
    this.expect(")");
    const ret = this.eat(":") ? this.parseType() : null;
    const body = this.parseBlock();
    return { name: name.value, typeParams, params, ret, body, span: join(start, name.span) };
  }

  private parseTypeDecl(): TypeDecl {
    const start = this.expect("type").span;
    const name = this.ident("el nombre del tipo");
    const params = this.parseTypeParams();
    this.expect("=");
    const type = this.parseType();
    this.expect(";", "al final de la declaración de tipo");
    return { name: name.value, params, type, span: join(start, name.span) };
  }

  // ---------- expresiones ----------

  parseExpr(): Expr {
    const left = this.parseBinary(0);
    const t = this.peek();
    if (t.kind === "op" && ASSIGN_OPS.has(t.value)) {
      if (!["ident", "member", "index"].includes(left.k)) {
        this.fail("no se puede asignar a esta expresión", left.span);
      }
      this.next();
      const value = this.parseExpr();
      return { k: "assign", op: t.value, target: left, value, span: join(left.span, value.span) };
    }
    return left;
  }

  private parseBinary(minPrec: number): Expr {
    let left = this.parseUnary();
    for (;;) {
      const t = this.peek();
      if (t.kind !== "op") break;
      const prec = BINARY_PRECEDENCE[t.value];
      if (prec === undefined || prec < minPrec) break;
      this.next();
      const right = this.parseBinary(prec + 1);
      const span = join(left.span, right.span);
      left = t.value === ".."
        ? { k: "range", start: left, end: right, span }
        : { k: "binary", op: t.value, left, right, span };
    }
    return left;
  }

  private parseUnary(): Expr {
    if (this.at("-") || this.at("!")) {
      const op = this.next();
      const expr = this.parseUnary();
      return { k: "unary", op: op.value as "-" | "!", expr, span: join(op.span, expr.span) };
    }
    return this.parsePostfix(this.parsePrimary());
  }

  private parseArgs(): Expr[] {
    this.expect("(");
    const args: Expr[] = [];
    while (!this.at(")")) {
      args.push(this.parseExpr());
      if (!this.eat(",")) break;
    }
    this.expect(")", "para cerrar la lista de argumentos");
    return args;
  }

  private parsePostfix(expr: Expr): Expr {
    for (;;) {
      if (this.at("(")) {
        const args = this.parseArgs();
        expr = { k: "call", callee: expr, typeArgs: [], args, span: join(expr.span, this.prev()) };
      } else if (this.at("::") && this.at("<", 1)) {
        // turbofish: identidad::<number>(5)
        this.next();
        this.next();
        const typeArgs: TypeNode[] = [];
        do typeArgs.push(this.parseType());
        while (this.eat(","));
        this.expect(">");
        const args = this.parseArgs();
        expr = { k: "call", callee: expr, typeArgs, args, span: join(expr.span, this.prev()) };
      } else if (this.at(".")) {
        this.next();
        const name = this.ident("el nombre de un campo o método");
        expr = { k: "member", object: expr, name: name.value, span: join(expr.span, name.span) };
      } else if (this.at("[")) {
        this.next();
        const index = this.parseExpr();
        const close = this.expect("]");
        expr = { k: "index", object: expr, index, span: join(expr.span, close.span) };
      } else {
        return expr;
      }
    }
  }

  private isObjectLiteralStart(): boolean {
    if (!this.at("{")) return false;
    if (this.at("}", 1)) return true;
    const t = this.peek(1);
    return (t.kind === "ident" || t.kind === "str") && (this.at(":", 2) || this.at(",", 2));
  }

  private parsePrimary(): Expr {
    const t = this.peek();
    switch (t.kind) {
      case "num":
        this.next();
        return { k: "num", value: parseFloat(t.value), span: t.span };
      case "str":
        this.next();
        return { k: "str", value: t.value, span: t.span };
      case "ident":
        this.next();
        return { k: "ident", name: t.value, span: t.span };
      case "eof":
        return this.fail("se esperaba una expresión, pero el archivo terminó");
    }
    switch (t.value) {
      case "true":
      case "false":
        this.next();
        return { k: "bool", value: t.value === "true", span: t.span };
      case "(": {
        this.next();
        const e = this.parseExpr();
        this.expect(")");
        return e;
      }
      case "[": {
        this.next();
        const items: Expr[] = [];
        while (!this.at("]")) {
          items.push(this.parseExpr());
          if (!this.eat(",")) break;
        }
        const close = this.expect("]", "para cerrar el arreglo");
        return { k: "array", items, span: join(t.span, close.span) };
      }
      case "{":
        return this.isObjectLiteralStart() ? this.parseObject() : this.parseBlockLike();
      case "if":
      case "while":
      case "loop":
      case "for":
      case "match":
        return this.parseBlockLike();
      case "|":
      case "||":
        return this.parseClosure();
      case "return": {
        this.next();
        const ends = [";", "}", ",", ")"].some((v) => this.at(v)) || this.peek().kind === "eof";
        const value = ends ? null : this.parseExpr();
        return { k: "return", value, span: value ? join(t.span, value.span) : t.span };
      }
      case "break":
        this.next();
        return { k: "break", span: t.span };
      case "continue":
        this.next();
        return { k: "continue", span: t.span };
    }
    return this.fail(`se esperaba una expresión, se encontró ${describe(t)}`);
  }

  private parseObject(): Expr {
    const open = this.expect("{");
    const fields: { name: string; value: Expr; span: Span }[] = [];
    while (!this.at("}")) {
      const key = this.peek();
      if (key.kind !== "ident" && key.kind !== "str") this.fail(`se esperaba el nombre de un campo, se encontró ${describe(key)}`);
      this.next();
      const value: Expr = this.eat(":") ? this.parseExpr() : { k: "ident", name: key.value, span: key.span };
      fields.push({ name: key.value, value, span: key.span });
      if (!this.eat(",")) break;
    }
    const close = this.expect("}", "para cerrar el objeto");
    return { k: "object", fields, span: join(open.span, close.span) };
  }

  private parseBlockLike(): Expr {
    const t = this.peek();
    switch (t.value) {
      case "{": {
        const block = this.parseBlock();
        return { k: "block", block, span: block.span };
      }
      case "if":
        return this.parseIf();
      case "while": {
        this.next();
        const cond = this.parseExpr();
        const body = this.parseBlock();
        return { k: "while", cond, body, span: join(t.span, body.span) };
      }
      case "loop": {
        this.next();
        const body = this.parseBlock();
        return { k: "loop", body, span: join(t.span, body.span) };
      }
      case "for": {
        this.next();
        const name = this.ident("el nombre de la variable del `for`");
        this.expect("in");
        const iter = this.parseExpr();
        const body = this.parseBlock();
        return { k: "for", name: name.value, iter, body, span: join(t.span, body.span) };
      }
      case "match":
        return this.parseMatch();
    }
    return this.fail(`se esperaba un bloque, se encontró ${describe(t)}`);
  }

  private parseIf(): Expr {
    const start = this.expect("if").span;
    const cond = this.parseExpr();
    const then = this.parseBlock();
    let else_: Expr | null = null;
    if (this.eat("else")) {
      if (this.at("if")) else_ = this.parseIf();
      else {
        const b = this.parseBlock();
        else_ = { k: "block", block: b, span: b.span };
      }
    }
    return { k: "if", cond, then, else_, span: join(start, (else_ ?? then).span) };
  }

  private parseMatch(): Expr {
    const start = this.expect("match").span;
    const subject = this.parseExpr();
    this.expect("{", "para abrir los brazos del `match`");
    const arms: Arm[] = [];
    while (!this.at("}")) {
      const pattern = this.parsePattern();
      const guard = this.eat("if") ? this.parseExpr() : null;
      this.expect("=>", "después del patrón");
      const body = this.parseExpr();
      arms.push({ pattern, guard, body, span: join(pattern.span, body.span) });
      if (!this.eat(",") && !this.at("}") && !BLOCK_LIKE.has(body.k)) {
        this.fail(`se esperaba \`,\` entre los brazos del \`match\`, se encontró ${describe(this.peek())}`);
      }
    }
    const close = this.expect("}");
    return { k: "match", subject, arms, span: join(start, close.span) };
  }

  private parseClosure(): Expr {
    const start = this.peek().span;
    const params: Param[] = [];
    if (!this.eat("||")) {
      this.expect("|");
      while (!this.at("|")) {
        const name = this.ident("el nombre de un parámetro");
        // sin uniones acá: `|x: number | string|` sería ambiguo
        const type = this.eat(":") ? this.parsePostfixType() : null;
        params.push({ name: name.value, type, span: name.span });
        if (!this.eat(",")) break;
      }
      this.expect("|");
    }
    const body = this.parseExpr();
    return { k: "closure", params, body, span: join(start, body.span) };
  }

  // ---------- patrones ----------

  private parsePattern(): Pattern {
    const t = this.peek();
    if (t.kind === "ident") {
      this.next();
      const type = this.eat(":") ? this.parseType() : null;
      if (t.value === "_") return { k: "wild", type, span: t.span };
      if (TYPE_NAMES.has(t.value) && !type) {
        this.fail(`\`${t.value}\` es un tipo, no se puede usar como nombre de variable`, t.span,
          `para filtrar por tipo escribí \`x: ${t.value}\``);
      }
      return { k: "bind", name: t.value, type, span: t.span };
    }
    if (t.kind === "num" || t.kind === "str") {
      this.next();
      return { k: "lit", value: t.kind === "num" ? parseFloat(t.value) : t.value, span: t.span };
    }
    if (this.at("-") && this.peek(1).kind === "num") {
      this.next();
      const n = this.next();
      return { k: "lit", value: -parseFloat(n.value), span: join(t.span, n.span) };
    }
    if (this.at("true") || this.at("false")) {
      this.next();
      return { k: "lit", value: t.value === "true", span: t.span };
    }
    if (this.at("{")) {
      this.next();
      const fields: { name: string; pattern: Pattern }[] = [];
      while (!this.at("}")) {
        const name = this.ident("el nombre de un campo");
        const pattern: Pattern = this.eat(":")
          ? this.parsePattern()
          : { k: "bind", name: name.value, type: null, span: name.span };
        fields.push({ name: name.value, pattern });
        if (!this.eat(",")) break;
      }
      const close = this.expect("}");
      return { k: "object", fields, span: join(t.span, close.span) };
    }
    return this.fail(`se esperaba un patrón, se encontró ${describe(t)}`);
  }

  // ---------- tipos ----------

  parseType(): TypeNode {
    const start = this.peek().span;
    this.eat("|"); // permite `| A | B` al estilo TS
    const members = [this.parsePostfixType()];
    while (this.eat("|")) members.push(this.parsePostfixType());
    if (members.length === 1) return members[0];
    return { k: "union", members, span: join(start, members[members.length - 1].span) };
  }

  private parsePostfixType(): TypeNode {
    let t = this.parsePrimaryType();
    while (this.at("[") && this.at("]", 1)) {
      this.next();
      const close = this.next();
      t = { k: "array", elem: t, span: join(t.span, close.span) };
    }
    return t;
  }

  private isFnType(): boolean {
    // ¿`(` empieza un tipo función `(a: number) => string`?
    let depth = 0;
    for (let i = this.pos; i < this.tokens.length; i++) {
      const t = this.tokens[i];
      if (t.kind === "op" && t.value === "(") depth++;
      else if (t.kind === "op" && t.value === ")") {
        depth--;
        if (depth === 0) {
          const n = this.tokens[i + 1];
          return n !== undefined && n.kind === "op" && n.value === "=>";
        }
      } else if (t.kind === "eof") return false;
    }
    return false;
  }

  private parsePrimaryType(): TypeNode {
    const t = this.peek();
    if (this.at("(")) {
      if (this.isFnType()) {
        this.next();
        const params: TypeNode[] = [];
        while (!this.at(")")) {
          if (this.peek().kind === "ident" && this.at(":", 1)) {
            this.next();
            this.next();
          }
          params.push(this.parseType());
          if (!this.eat(",")) break;
        }
        this.expect(")");
        this.expect("=>");
        const ret = this.parseType();
        return { k: "fn", params, ret, span: join(t.span, ret.span) };
      }
      this.next();
      const inner = this.parseType();
      this.expect(")");
      return inner;
    }
    if (this.at("{")) {
      this.next();
      const fields: { name: string; type: TypeNode }[] = [];
      while (!this.at("}")) {
        const key = this.peek();
        if (key.kind !== "ident" && key.kind !== "str") this.fail(`se esperaba el nombre de un campo, se encontró ${describe(key)}`);
        this.next();
        this.expect(":", `después de \`${key.value}\``);
        fields.push({ name: key.value, type: this.parseType() });
        if (!this.eat(",") && !this.eat(";")) break;
      }
      const close = this.expect("}");
      return { k: "object", fields, span: join(t.span, close.span) };
    }
    if (t.kind === "str") {
      this.next();
      return { k: "lit", value: t.value, span: t.span };
    }
    if (t.kind === "num") {
      this.next();
      return { k: "lit", value: parseFloat(t.value), span: t.span };
    }
    if (this.at("-") && this.peek(1).kind === "num") {
      this.next();
      const n = this.next();
      return { k: "lit", value: -parseFloat(n.value), span: join(t.span, n.span) };
    }
    if (this.at("true") || this.at("false")) {
      this.next();
      return { k: "lit", value: t.value === "true", span: t.span };
    }
    if (t.kind === "ident") {
      this.next();
      const args: TypeNode[] = [];
      if (this.eat("<")) {
        do args.push(this.parseType());
        while (this.eat(","));
        this.expect(">");
      }
      return { k: "name", name: t.value, args, span: join(t.span, this.prev()) };
    }
    return this.fail(`se esperaba un tipo, se encontró ${describe(t)}`);
  }
}

export function parse(tokens: Token[]): Program {
  return new Parser(tokens).parseProgram();
}
