use crate::ast::*;
use crate::diagnostics::UwuError;
use crate::lexer::{Span, Token, TokenKind};

type PResult<T> = Result<T, UwuError>;

fn precedence(op: &str) -> Option<u8> {
    match op {
        ".." => Some(1),
        "||" => Some(2),
        "&&" => Some(3),
        "==" | "!=" | "<" | ">" | "<=" | ">=" => Some(4),
        "+" | "-" => Some(5),
        "*" | "/" | "%" => Some(6),
        _ => None,
    }
}

const ASSIGN_OPS: &[&str] = &["=", "+=", "-=", "*=", "/="];
const TYPE_NAMES: &[&str] = &["number", "string", "boolean", "void", "never"];

fn describe(t: &Token) -> String {
    match t.kind {
        TokenKind::Eof => "el final del archivo".into(),
        TokenKind::Str => format!("`\"{}\"`", t.value),
        _ => format!("`{}`", t.value),
    }
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    next_id: usize,
}

impl Parser {
    /// Los ids de funciones y patrones incluyen el archivo para ser únicos en todo el programa.
    pub fn new(tokens: Vec<Token>, file: usize) -> Self {
        Self { tokens, pos: 0, next_id: file << 32 }
    }

    // ---------- utilidades ----------

    fn fresh_id(&mut self) -> usize {
        self.next_id += 1;
        self.next_id
    }

    fn peek(&self, offset: usize) -> &Token {
        let i = (self.pos + offset).min(self.tokens.len() - 1);
        &self.tokens[i]
    }

    fn at(&self, value: &str, offset: usize) -> bool {
        let t = self.peek(offset);
        (t.kind == TokenKind::Op || t.kind == TokenKind::Kw) && t.value == value
    }

    fn next(&mut self) -> Token {
        let t = self.peek(0).clone();
        if t.kind != TokenKind::Eof {
            self.pos += 1;
        }
        t
    }

    fn eat(&mut self, value: &str) -> bool {
        if self.at(value, 0) {
            self.next();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, value: &str, context: &str) -> PResult<Token> {
        if self.at(value, 0) {
            return Ok(self.next());
        }
        let ctx = if context.is_empty() { String::new() } else { format!(" {}", context) };
        self.fail(
            format!("se esperaba `{}`{}, se encontró {}", value, ctx, describe(self.peek(0))),
            self.peek(0).span,
            None,
        )
    }

    fn ident(&mut self, what: &str) -> PResult<Token> {
        let t = self.peek(0);
        if t.kind == TokenKind::Ident {
            return Ok(self.next());
        }
        let hint = if t.kind == TokenKind::Kw { Some(format!("`{}` es una palabra reservada", t.value)) } else { None };
        let span = t.span;
        let msg = format!("se esperaba {}, se encontró {}", what, describe(self.peek(0)));
        self.fail(msg, span, hint)
    }

    fn fail<T>(&self, message: String, span: Span, hint: Option<String>) -> PResult<T> {
        Err(UwuError::new(message, span, hint))
    }

    fn prev(&self) -> Span {
        self.tokens[self.pos.saturating_sub(1)].span
    }

    // ---------- programa y sentencias ----------

    pub fn parse_program(&mut self) -> PResult<Program> {
        let mut module_name = None;
        if self.at_export_module() {
            let start = self.next().span;
            self.next();
            let name = self.ident("el nombre del módulo")?;
            self.expect(";", "al final de `export module`")?;
            module_name = Some((name.value, Span::join(start, name.span)));
        }
        let (mut stmts, tail) = self.parse_stmt_list("eof")?;
        if let Some(tail) = tail {
            let span = tail.span();
            stmts.push(Stmt::Expr { expr: tail, semi: false, span });
        }
        Ok(Program { stmts, module_name })
    }

    fn at_ident(&self, value: &str, offset: usize) -> bool {
        let t = self.peek(offset);
        t.kind == TokenKind::Ident && t.value == value
    }

    fn at_export_module(&self) -> bool {
        self.at("export", 0) && self.at_ident("module", 1)
    }

    // ---------- módulos ----------

    fn parse_import(&mut self, reexport: bool, start: Span) -> PResult<Stmt> {
        self.expect("import", "")?;
        let t = self.peek(0).clone();
        let target = if self.eat("<") {
            let name = self.ident("el nombre de una biblioteca estándar")?;
            self.expect(">", "para cerrar el nombre de la biblioteca")?;
            ImportTarget::Std(name.value)
        } else if t.kind == TokenKind::Str {
            self.next();
            ImportTarget::File(t.value)
        } else if t.kind == TokenKind::Ident {
            self.next();
            ImportTarget::Module(t.value)
        } else {
            return self.fail(
                format!(
                    "se esperaba `<biblioteca>`, `\"archivo.uwu\"` o el nombre de un módulo, se encontró {}",
                    describe(&t)
                ),
                t.span,
                Some("ejemplos: `import <fs>;`, `import \"util.uwu\";`, `import geometria;`".into()),
            );
        };
        self.expect(";", "al final del `import`")?;
        let span = Span::join(start, self.prev());
        Ok(Stmt::Import(ImportDecl { target, reexport, span }))
    }

    fn parse_export(&mut self) -> PResult<Stmt> {
        let start = self.peek(0).span;
        if self.at_export_module() {
            return self.fail(
                "`export module` tiene que ser la primera declaración del archivo".into(),
                start,
                Some("movelo a la primera línea".into()),
            );
        }
        self.next();
        if self.at("import", 0) {
            return self.parse_import(true, start);
        }
        if self.at("let", 0) {
            let mut stmt = self.parse_let()?;
            if let Stmt::Let { exported, .. } = &mut stmt {
                *exported = true;
            }
            return Ok(stmt);
        }
        if self.at("fn", 0) {
            let mut decl = self.parse_fn()?;
            decl.exported = true;
            return Ok(Stmt::Fn(decl));
        }
        if self.at("type", 0) {
            let mut decl = self.parse_type_decl()?;
            decl.exported = true;
            return Ok(Stmt::Type(decl));
        }
        self.fail(
            format!(
                "se esperaba `fn`, `let`, `type` o `import` después de `export`, se encontró {}",
                describe(self.peek(0))
            ),
            self.peek(0).span,
            None,
        )
    }

    fn parse_using(&mut self) -> PResult<Stmt> {
        let start = self.next().span; // `using`
        if self.at_ident("namespace", 0) && self.peek(1).kind == TokenKind::Ident {
            self.next();
            let ns = self.ident("el nombre de un módulo")?;
            self.expect(";", "al final del `using`")?;
            return Ok(Stmt::Using { ns: ns.value, name: None, span: Span::join(start, ns.span) });
        }
        let ns = self.ident("el nombre de un módulo")?;
        self.expect("::", "en `using módulo::nombre`")?;
        let name = self.ident("el nombre a traer")?;
        self.expect(";", "al final del `using`")?;
        Ok(Stmt::Using { ns: ns.value, name: Some(name.value), span: Span::join(start, name.span) })
    }

    fn is_end(&self, end: &str) -> bool {
        self.peek(0).kind == TokenKind::Eof || (end != "eof" && self.at(end, 0))
    }

    fn parse_stmt_list(&mut self, end: &str) -> PResult<(Vec<Stmt>, Option<Expr>)> {
        let mut stmts: Vec<Stmt> = Vec::new();
        let mut tail: Option<Expr> = None;
        let top_level = end == "eof";
        while !self.is_end(end) {
            if self.eat(";") {
                continue;
            }
            if self.at("import", 0) || self.at("export", 0) {
                if !top_level {
                    let kw = self.peek(0).value.clone();
                    return self.fail(
                        format!("`{}` sólo puede usarse en el nivel superior de un archivo", kw),
                        self.peek(0).span,
                        Some("movelo fuera de la función o el bloque".into()),
                    );
                }
                let stmt = if self.at("import", 0) {
                    let start = self.peek(0).span;
                    self.parse_import(false, start)?
                } else {
                    self.parse_export()?
                };
                stmts.push(stmt);
                continue;
            }
            if self.at_ident("using", 0) && self.peek(1).kind == TokenKind::Ident {
                stmts.push(self.parse_using()?);
                continue;
            }
            if self.at("let", 0) {
                stmts.push(self.parse_let()?);
                continue;
            }
            if self.at("fn", 0) && self.peek(1).kind == TokenKind::Ident {
                let decl = self.parse_fn()?;
                stmts.push(Stmt::Fn(decl));
                continue;
            }
            if self.at("type", 0) {
                let decl = self.parse_type_decl()?;
                stmts.push(Stmt::Type(decl));
                continue;
            }
            // Igual que en Rust: un `if`/`match`/bloque al inicio de una sentencia no necesita `;`
            let block_like = (self.at("{", 0) && !self.is_object_literal_start())
                || ["if", "while", "loop", "for", "match"].iter().any(|v| self.at(v, 0));
            let expr = if block_like { self.parse_block_like()? } else { self.parse_expr()? };
            if self.eat(";") {
                let span = expr.span();
                stmts.push(Stmt::Expr { expr, semi: true, span });
                continue;
            }
            if self.is_end(end) {
                tail = Some(expr);
                break;
            }
            if block_like {
                let span = expr.span();
                stmts.push(Stmt::Expr { expr, semi: false, span });
                continue;
            }
            return self.fail(
                format!("se esperaba `;`, se encontró {}", describe(self.peek(0))),
                self.prev(),
                Some("agregá `;` al final de la sentencia".into()),
            );
        }
        Ok((stmts, tail))
    }

    fn parse_block(&mut self) -> PResult<Block> {
        let open = self.expect("{", "")?;
        let (stmts, tail) = self.parse_stmt_list("}")?;
        let close = self.expect("}", "para cerrar el bloque")?;
        Ok(Block { stmts, tail: tail.map(Box::new), span: Span::join(open.span, close.span) })
    }

    fn parse_let(&mut self) -> PResult<Stmt> {
        let start = self.expect("let", "")?.span;
        let mutable = self.eat("mut");
        let name = self.ident("el nombre de la variable")?;
        let ty = if self.eat(":") { Some(self.parse_type()?) } else { None };
        self.expect("=", &format!("después de `{}`", name.value))?;
        let init = self.parse_expr()?;
        self.expect(";", "al final del `let`")?;
        let span = Span::join(start, init.span());
        Ok(Stmt::Let { mutable, exported: false, name: name.value, ty, init, span })
    }

    fn parse_type_params(&mut self) -> PResult<Vec<String>> {
        let mut params = Vec::new();
        if self.eat("<") {
            loop {
                params.push(self.ident("un parámetro de tipo")?.value);
                if !self.eat(",") {
                    break;
                }
            }
            self.expect(">", "")?;
        }
        Ok(params)
    }

    fn parse_fn(&mut self) -> PResult<FnDecl> {
        let start = self.expect("fn", "")?.span;
        let name = self.ident("el nombre de la función")?;
        let type_params = self.parse_type_params()?;
        self.expect("(", "")?;
        let mut params: Vec<ParamNode> = Vec::new();
        while !self.at(")", 0) {
            let p = self.ident("el nombre de un parámetro")?;
            if !self.at(":", 0) {
                return self.fail(
                    format!("falta el tipo del parámetro `{}`", p.value),
                    p.span,
                    Some(format!("escribilo así: `{}: number`", p.value)),
                );
            }
            self.next();
            let ty = self.parse_type()?;
            params.push(ParamNode { name: p.value, ty: Some(ty), span: p.span });
            if !self.eat(",") {
                break;
            }
        }
        self.expect(")", "")?;
        let ret = if self.eat(":") { Some(self.parse_type()?) } else { None };
        let body = self.parse_block()?;
        Ok(FnDecl {
            id: self.fresh_id(),
            exported: false,
            name: name.value,
            type_params,
            params,
            ret,
            body,
            span: Span::join(start, name.span),
        })
    }

    fn parse_type_decl(&mut self) -> PResult<TypeDecl> {
        let start = self.expect("type", "")?.span;
        let name = self.ident("el nombre del tipo")?;
        let params = self.parse_type_params()?;
        self.expect("=", "")?;
        let ty = self.parse_type()?;
        self.expect(";", "al final de la declaración de tipo")?;
        Ok(TypeDecl { exported: false, name: name.value, params, ty, span: Span::join(start, name.span) })
    }

    // ---------- expresiones ----------

    pub fn parse_expr(&mut self) -> PResult<Expr> {
        let left = self.parse_binary(0)?;
        let t = self.peek(0);
        if t.kind == TokenKind::Op && ASSIGN_OPS.contains(&t.value.as_str()) {
            if !matches!(left, Expr::Ident { .. } | Expr::Member { .. } | Expr::Index { .. }) {
                return self.fail("no se puede asignar a esta expresión".into(), left.span(), None);
            }
            let op = self.next().value;
            let value = self.parse_expr()?;
            let span = Span::join(left.span(), value.span());
            return Ok(Expr::Assign { op, target: Box::new(left), value: Box::new(value), span });
        }
        Ok(left)
    }

    fn parse_binary(&mut self, min_prec: u8) -> PResult<Expr> {
        let mut left = self.parse_unary()?;
        loop {
            let t = self.peek(0);
            if t.kind != TokenKind::Op {
                break;
            }
            let Some(prec) = precedence(&t.value) else { break };
            if prec < min_prec {
                break;
            }
            let op = self.next().value;
            let right = self.parse_binary(prec + 1)?;
            let span = Span::join(left.span(), right.span());
            left = if op == ".." {
                Expr::Range { start: Box::new(left), end: Box::new(right), span }
            } else {
                Expr::Binary { op, left: Box::new(left), right: Box::new(right), span }
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> PResult<Expr> {
        if self.at("-", 0) || self.at("!", 0) {
            let op = self.next();
            let expr = self.parse_unary()?;
            let span = Span::join(op.span, expr.span());
            return Ok(Expr::Unary { op: op.value, expr: Box::new(expr), span });
        }
        let primary = self.parse_primary()?;
        self.parse_postfix(primary)
    }

    fn parse_args(&mut self) -> PResult<Vec<Expr>> {
        self.expect("(", "")?;
        let mut args = Vec::new();
        while !self.at(")", 0) {
            args.push(self.parse_expr()?);
            if !self.eat(",") {
                break;
            }
        }
        self.expect(")", "para cerrar la lista de argumentos")?;
        Ok(args)
    }

    fn parse_postfix(&mut self, mut expr: Expr) -> PResult<Expr> {
        loop {
            if self.at("(", 0) {
                let args = self.parse_args()?;
                let span = Span::join(expr.span(), self.prev());
                expr = Expr::Call { callee: Box::new(expr), type_args: vec![], args, span };
            } else if self.at("::", 0) && self.at("<", 1) {
                // turbofish: identidad::<number>(5)
                self.next();
                self.next();
                let mut type_args = Vec::new();
                loop {
                    type_args.push(self.parse_type()?);
                    if !self.eat(",") {
                        break;
                    }
                }
                self.expect(">", "")?;
                let args = self.parse_args()?;
                let span = Span::join(expr.span(), self.prev());
                expr = Expr::Call { callee: Box::new(expr), type_args, args, span };
            } else if self.at("::", 0) && self.peek(1).kind == TokenKind::Ident {
                // fs::leer
                let Expr::Ident { name: ns, span: ns_span } = &expr else {
                    return self.fail(
                        "`::` sólo puede usarse después del nombre de un módulo, como en `fs::leer`".into(),
                        self.peek(0).span,
                        None,
                    );
                };
                let (ns, ns_span) = (ns.clone(), *ns_span);
                self.next();
                let member = self.next();
                expr = Expr::Path { ns, name: member.value, span: Span::join(ns_span, member.span) };
            } else if self.at(".", 0) {
                self.next();
                let name = self.ident("el nombre de un campo o método")?;
                let span = Span::join(expr.span(), name.span);
                expr = Expr::Member { object: Box::new(expr), name: name.value, span };
            } else if self.at("[", 0) {
                self.next();
                let index = self.parse_expr()?;
                let close = self.expect("]", "")?;
                let span = Span::join(expr.span(), close.span);
                expr = Expr::Index { object: Box::new(expr), index: Box::new(index), span };
            } else {
                return Ok(expr);
            }
        }
    }

    fn is_object_literal_start(&self) -> bool {
        if !self.at("{", 0) {
            return false;
        }
        if self.at("}", 1) {
            return true;
        }
        let t = self.peek(1);
        (t.kind == TokenKind::Ident || t.kind == TokenKind::Str) && (self.at(":", 2) || self.at(",", 2))
    }

    fn parse_primary(&mut self) -> PResult<Expr> {
        let t = self.peek(0).clone();
        match t.kind {
            TokenKind::Num => {
                self.next();
                return Ok(Expr::Num { value: t.value.parse().unwrap_or(0.0), span: t.span });
            }
            TokenKind::Str => {
                self.next();
                return Ok(Expr::Str { value: t.value, span: t.span });
            }
            TokenKind::Ident => {
                self.next();
                return Ok(Expr::Ident { name: t.value, span: t.span });
            }
            TokenKind::Eof => {
                return self.fail("se esperaba una expresión, pero el archivo terminó".into(), t.span, None);
            }
            _ => {}
        }
        match t.value.as_str() {
            "true" | "false" => {
                self.next();
                Ok(Expr::Bool { value: t.value == "true", span: t.span })
            }
            "(" => {
                self.next();
                let e = self.parse_expr()?;
                self.expect(")", "")?;
                Ok(e)
            }
            "[" => {
                self.next();
                let mut items = Vec::new();
                while !self.at("]", 0) {
                    items.push(self.parse_expr()?);
                    if !self.eat(",") {
                        break;
                    }
                }
                let close = self.expect("]", "para cerrar el arreglo")?;
                Ok(Expr::Array { items, span: Span::join(t.span, close.span) })
            }
            "{" => {
                if self.is_object_literal_start() {
                    self.parse_object()
                } else {
                    self.parse_block_like()
                }
            }
            "if" | "while" | "loop" | "for" | "match" => self.parse_block_like(),
            "|" | "||" => self.parse_closure(),
            "return" => {
                self.next();
                let ends = [";", "}", ",", ")"].iter().any(|v| self.at(v, 0)) || self.peek(0).kind == TokenKind::Eof;
                let value = if ends { None } else { Some(self.parse_expr()?) };
                let span = value.as_ref().map(|v| Span::join(t.span, v.span())).unwrap_or(t.span);
                Ok(Expr::Return { value: value.map(Box::new), span })
            }
            "break" => {
                self.next();
                Ok(Expr::Break { span: t.span })
            }
            "continue" => {
                self.next();
                Ok(Expr::Continue { span: t.span })
            }
            _ => self.fail(format!("se esperaba una expresión, se encontró {}", describe(&t)), t.span, None),
        }
    }

    fn parse_object(&mut self) -> PResult<Expr> {
        let open = self.expect("{", "")?;
        let mut fields: Vec<ObjField> = Vec::new();
        while !self.at("}", 0) {
            let key = self.peek(0).clone();
            if key.kind != TokenKind::Ident && key.kind != TokenKind::Str {
                return self.fail(
                    format!("se esperaba el nombre de un campo, se encontró {}", describe(&key)),
                    key.span,
                    None,
                );
            }
            self.next();
            let value = if self.eat(":") {
                self.parse_expr()?
            } else {
                Expr::Ident { name: key.value.clone(), span: key.span }
            };
            fields.push(ObjField { name: key.value, value, span: key.span });
            if !self.eat(",") {
                break;
            }
        }
        let close = self.expect("}", "para cerrar el objeto")?;
        Ok(Expr::Object { fields, span: Span::join(open.span, close.span) })
    }

    fn parse_block_like(&mut self) -> PResult<Expr> {
        let t = self.peek(0).clone();
        match t.value.as_str() {
            "{" => {
                let block = self.parse_block()?;
                let span = block.span;
                Ok(Expr::Block { block, span })
            }
            "if" => self.parse_if(),
            "while" => {
                self.next();
                let cond = self.parse_expr()?;
                let body = self.parse_block()?;
                let span = Span::join(t.span, body.span);
                Ok(Expr::While { cond: Box::new(cond), body, span })
            }
            "loop" => {
                self.next();
                let body = self.parse_block()?;
                let span = Span::join(t.span, body.span);
                Ok(Expr::Loop { body, span })
            }
            "for" => {
                self.next();
                let name = self.ident("el nombre de la variable del `for`")?;
                self.expect("in", "")?;
                let iter = self.parse_expr()?;
                let body = self.parse_block()?;
                let span = Span::join(t.span, body.span);
                Ok(Expr::For { name: name.value, iter: Box::new(iter), body, span })
            }
            "match" => self.parse_match(),
            _ => self.fail(format!("se esperaba un bloque, se encontró {}", describe(&t)), t.span, None),
        }
    }

    fn parse_if(&mut self) -> PResult<Expr> {
        let start = self.expect("if", "")?.span;
        let cond = self.parse_expr()?;
        let then = self.parse_block()?;
        let mut else_: Option<Expr> = None;
        if self.eat("else") {
            if self.at("if", 0) {
                else_ = Some(self.parse_if()?);
            } else {
                let b = self.parse_block()?;
                let span = b.span;
                else_ = Some(Expr::Block { block: b, span });
            }
        }
        let end_span = else_.as_ref().map(|e| e.span()).unwrap_or(then.span);
        Ok(Expr::If { cond: Box::new(cond), then, else_: else_.map(Box::new), span: Span::join(start, end_span) })
    }

    fn parse_match(&mut self) -> PResult<Expr> {
        let start = self.expect("match", "")?.span;
        let subject = self.parse_expr()?;
        self.expect("{", "para abrir los brazos del `match`")?;
        let mut arms: Vec<Arm> = Vec::new();
        while !self.at("}", 0) {
            let pattern = self.parse_pattern()?;
            let guard = if self.eat("if") { Some(self.parse_expr()?) } else { None };
            self.expect("=>", "después del patrón")?;
            let body = self.parse_expr()?;
            let span = Span::join(pattern.span(), body.span());
            let block_like = body.is_block_like();
            arms.push(Arm { pattern, guard, body, span });
            if !self.eat(",") && !self.at("}", 0) && !block_like {
                return self.fail(
                    format!("se esperaba `,` entre los brazos del `match`, se encontró {}", describe(self.peek(0))),
                    self.peek(0).span,
                    None,
                );
            }
        }
        let close = self.expect("}", "")?;
        Ok(Expr::Match { subject: Box::new(subject), arms, span: Span::join(start, close.span) })
    }

    fn parse_closure(&mut self) -> PResult<Expr> {
        let start = self.peek(0).span;
        let mut params: Vec<ParamNode> = Vec::new();
        if !self.eat("||") {
            self.expect("|", "")?;
            while !self.at("|", 0) {
                let name = self.ident("el nombre de un parámetro")?;
                // sin uniones acá: `|x: number | string|` sería ambiguo
                let ty = if self.eat(":") { Some(self.parse_postfix_type()?) } else { None };
                params.push(ParamNode { name: name.value, ty, span: name.span });
                if !self.eat(",") {
                    break;
                }
            }
            self.expect("|", "")?;
        }
        let body = self.parse_expr()?;
        let span = Span::join(start, body.span());
        Ok(Expr::Closure { params, body: Box::new(body), span })
    }

    // ---------- patrones ----------

    fn parse_pattern(&mut self) -> PResult<Pattern> {
        let t = self.peek(0).clone();
        if t.kind == TokenKind::Ident {
            self.next();
            let ty = if self.eat(":") { Some(self.parse_type()?) } else { None };
            if t.value == "_" {
                return Ok(Pattern::Wild { id: self.fresh_id(), ty, span: t.span });
            }
            if TYPE_NAMES.contains(&t.value.as_str()) && ty.is_none() {
                return self.fail(
                    format!("`{}` es un tipo, no se puede usar como nombre de variable", t.value),
                    t.span,
                    Some(format!("para filtrar por tipo escribí `x: {}`", t.value)),
                );
            }
            return Ok(Pattern::Bind { id: self.fresh_id(), name: t.value, ty, span: t.span });
        }
        if t.kind == TokenKind::Num || t.kind == TokenKind::Str {
            self.next();
            let value = if t.kind == TokenKind::Num {
                LitVal::Num(t.value.parse().unwrap_or(0.0))
            } else {
                LitVal::Str(t.value)
            };
            return Ok(Pattern::Lit { value, span: t.span });
        }
        if self.at("-", 0) && self.peek(1).kind == TokenKind::Num {
            self.next();
            let n = self.next();
            let value = LitVal::Num(-n.value.parse().unwrap_or(0.0));
            return Ok(Pattern::Lit { value, span: Span::join(t.span, n.span) });
        }
        if self.at("true", 0) || self.at("false", 0) {
            self.next();
            return Ok(Pattern::Lit { value: LitVal::Bool(t.value == "true"), span: t.span });
        }
        if self.at("{", 0) {
            self.next();
            let mut fields: Vec<(String, Pattern)> = Vec::new();
            while !self.at("}", 0) {
                let name = self.ident("el nombre de un campo")?;
                let pattern = if self.eat(":") {
                    self.parse_pattern()?
                } else {
                    Pattern::Bind { id: self.fresh_id(), name: name.value.clone(), ty: None, span: name.span }
                };
                fields.push((name.value, pattern));
                if !self.eat(",") {
                    break;
                }
            }
            let close = self.expect("}", "")?;
            return Ok(Pattern::Object { fields, span: Span::join(t.span, close.span) });
        }
        self.fail(format!("se esperaba un patrón, se encontró {}", describe(&t)), t.span, None)
    }

    // ---------- tipos ----------

    pub fn parse_type(&mut self) -> PResult<TypeNode> {
        let start = self.peek(0).span;
        self.eat("|"); // permite `| A | B` al estilo TS
        let mut members = vec![self.parse_postfix_type()?];
        while self.eat("|") {
            members.push(self.parse_postfix_type()?);
        }
        if members.len() == 1 {
            return Ok(members.pop().unwrap());
        }
        let end = members.last().unwrap().span();
        Ok(TypeNode::Union { members, span: Span::join(start, end) })
    }

    fn parse_postfix_type(&mut self) -> PResult<TypeNode> {
        let mut t = self.parse_primary_type()?;
        while self.at("[", 0) && self.at("]", 1) {
            self.next();
            let close = self.next();
            let span = Span::join(t.span(), close.span);
            t = TypeNode::Array { elem: Box::new(t), span };
        }
        Ok(t)
    }

    fn is_fn_type(&self) -> bool {
        // ¿`(` empieza un tipo función `(a: number) => string`?
        let mut depth = 0;
        for i in self.pos..self.tokens.len() {
            let t = &self.tokens[i];
            if t.kind == TokenKind::Op && t.value == "(" {
                depth += 1;
            } else if t.kind == TokenKind::Op && t.value == ")" {
                depth -= 1;
                if depth == 0 {
                    let n = self.tokens.get(i + 1);
                    return n.is_some_and(|n| n.kind == TokenKind::Op && n.value == "=>");
                }
            } else if t.kind == TokenKind::Eof {
                return false;
            }
        }
        false
    }

    fn parse_primary_type(&mut self) -> PResult<TypeNode> {
        let t = self.peek(0).clone();
        if self.at("(", 0) {
            if self.is_fn_type() {
                self.next();
                let mut params = Vec::new();
                while !self.at(")", 0) {
                    if self.peek(0).kind == TokenKind::Ident && self.at(":", 1) {
                        self.next();
                        self.next();
                    }
                    params.push(self.parse_type()?);
                    if !self.eat(",") {
                        break;
                    }
                }
                self.expect(")", "")?;
                self.expect("=>", "")?;
                let ret = self.parse_type()?;
                let span = Span::join(t.span, ret.span());
                return Ok(TypeNode::Fn { params, ret: Box::new(ret), span });
            }
            self.next();
            let inner = self.parse_type()?;
            self.expect(")", "")?;
            return Ok(inner);
        }
        if self.at("{", 0) {
            self.next();
            let mut fields: Vec<(String, TypeNode)> = Vec::new();
            while !self.at("}", 0) {
                let key = self.peek(0).clone();
                if key.kind != TokenKind::Ident && key.kind != TokenKind::Str {
                    return self.fail(
                        format!("se esperaba el nombre de un campo, se encontró {}", describe(&key)),
                        key.span,
                        None,
                    );
                }
                self.next();
                self.expect(":", &format!("después de `{}`", key.value))?;
                fields.push((key.value, self.parse_type()?));
                if !self.eat(",") && !self.eat(";") {
                    break;
                }
            }
            let close = self.expect("}", "")?;
            return Ok(TypeNode::Object { fields, span: Span::join(t.span, close.span) });
        }
        if t.kind == TokenKind::Str {
            self.next();
            return Ok(TypeNode::Lit { value: LitVal::Str(t.value), span: t.span });
        }
        if t.kind == TokenKind::Num {
            self.next();
            return Ok(TypeNode::Lit { value: LitVal::Num(t.value.parse().unwrap_or(0.0)), span: t.span });
        }
        if self.at("-", 0) && self.peek(1).kind == TokenKind::Num {
            self.next();
            let n = self.next();
            return Ok(TypeNode::Lit {
                value: LitVal::Num(-n.value.parse().unwrap_or(0.0)),
                span: Span::join(t.span, n.span),
            });
        }
        if self.at("true", 0) || self.at("false", 0) {
            self.next();
            return Ok(TypeNode::Lit { value: LitVal::Bool(t.value == "true"), span: t.span });
        }
        if t.kind == TokenKind::Ident {
            self.next();
            let mut args = Vec::new();
            if self.eat("<") {
                loop {
                    args.push(self.parse_type()?);
                    if !self.eat(",") {
                        break;
                    }
                }
                self.expect(">", "")?;
            }
            return Ok(TypeNode::Name { name: t.value, args, span: Span::join(t.span, self.prev()) });
        }
        self.fail(format!("se esperaba un tipo, se encontró {}", describe(&t)), t.span, None)
    }
}

pub fn parse(tokens: Vec<Token>, file: usize) -> Result<Program, UwuError> {
    Parser::new(tokens, file).parse_program()
}
