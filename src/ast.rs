use crate::lexer::Span;

// ---------- Literales ----------

#[derive(Clone, Debug, PartialEq)]
pub enum LitVal {
    Num(f64),
    Str(String),
    Bool(bool),
}

// ---------- Tipos (tal como se escriben en el código) ----------

#[derive(Clone, Debug)]
pub enum TypeNode {
    Name { name: String, args: Vec<TypeNode>, span: Span },
    Lit { value: LitVal, span: Span },
    Array { elem: Box<TypeNode>, span: Span },
    Object { fields: Vec<(String, TypeNode)>, span: Span },
    Union { members: Vec<TypeNode>, span: Span },
    Fn { params: Vec<TypeNode>, ret: Box<TypeNode>, span: Span },
}

impl TypeNode {
    pub fn span(&self) -> Span {
        match self {
            TypeNode::Name { span, .. }
            | TypeNode::Lit { span, .. }
            | TypeNode::Array { span, .. }
            | TypeNode::Object { span, .. }
            | TypeNode::Union { span, .. }
            | TypeNode::Fn { span, .. } => *span,
        }
    }
}

// ---------- Expresiones ----------

#[derive(Clone, Debug)]
pub struct ObjField {
    pub name: String,
    pub value: Expr,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum Expr {
    Num {
        value: f64,
        span: Span,
    },
    Str {
        value: String,
        span: Span,
    },
    Bool {
        value: bool,
        span: Span,
    },
    Ident {
        name: String,
        span: Span,
    },
    /// `fs::leer`: un miembro de una biblioteca estándar importada.
    Path {
        ns: String,
        name: String,
        span: Span,
    },
    Array {
        items: Vec<Expr>,
        span: Span,
    },
    Object {
        fields: Vec<ObjField>,
        span: Span,
    },
    Unary {
        op: String,
        expr: Box<Expr>,
        span: Span,
    },
    Binary {
        op: String,
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        span: Span,
    },
    Assign {
        op: String,
        target: Box<Expr>,
        value: Box<Expr>,
        span: Span,
    },
    Call {
        callee: Box<Expr>,
        type_args: Vec<TypeNode>,
        args: Vec<Expr>,
        span: Span,
    },
    Member {
        object: Box<Expr>,
        name: String,
        span: Span,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    Block {
        block: Block,
        span: Span,
    },
    If {
        cond: Box<Expr>,
        then: Block,
        else_: Option<Box<Expr>>,
        span: Span,
    },
    While {
        cond: Box<Expr>,
        body: Block,
        span: Span,
    },
    Loop {
        body: Block,
        span: Span,
    },
    For {
        name: String,
        iter: Box<Expr>,
        body: Block,
        span: Span,
    },
    Match {
        subject: Box<Expr>,
        arms: Vec<Arm>,
        span: Span,
    },
    Closure {
        params: Vec<ParamNode>,
        body: Box<Expr>,
        span: Span,
    },
    Return {
        value: Option<Box<Expr>>,
        span: Span,
    },
    Break {
        span: Span,
    },
    Continue {
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Num { span, .. }
            | Expr::Str { span, .. }
            | Expr::Bool { span, .. }
            | Expr::Ident { span, .. }
            | Expr::Path { span, .. }
            | Expr::Array { span, .. }
            | Expr::Object { span, .. }
            | Expr::Unary { span, .. }
            | Expr::Binary { span, .. }
            | Expr::Range { span, .. }
            | Expr::Assign { span, .. }
            | Expr::Call { span, .. }
            | Expr::Member { span, .. }
            | Expr::Index { span, .. }
            | Expr::Block { span, .. }
            | Expr::If { span, .. }
            | Expr::While { span, .. }
            | Expr::Loop { span, .. }
            | Expr::For { span, .. }
            | Expr::Match { span, .. }
            | Expr::Closure { span, .. }
            | Expr::Return { span, .. }
            | Expr::Break { span }
            | Expr::Continue { span } => *span,
        }
    }

    /// `if`/`match`/bloques: como sentencia no necesitan `;`.
    pub fn is_block_like(&self) -> bool {
        matches!(
            self,
            Expr::Block { .. }
                | Expr::If { .. }
                | Expr::While { .. }
                | Expr::Loop { .. }
                | Expr::For { .. }
                | Expr::Match { .. }
        )
    }
}

#[derive(Clone, Debug)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    /// Última expresión sin `;` → valor del bloque.
    pub tail: Option<Box<Expr>>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Let {
        mutable: bool,
        exported: bool,
        name: String,
        ty: Option<TypeNode>,
        init: Expr,
        span: Span,
    },
    Expr {
        expr: Expr,
        semi: bool,
        span: Span,
    },
    Fn(FnDecl),
    Type(TypeDecl),
    Import(ImportDecl),
    /// `using namespace fs;` (name = None) o `using fs::leer;` (name = Some).
    Using {
        ns: String,
        name: Option<String>,
        span: Span,
    },
}

impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Stmt::Let { span, .. } | Stmt::Expr { span, .. } | Stmt::Using { span, .. } => *span,
            Stmt::Fn(d) => d.span,
            Stmt::Type(d) => d.span,
            Stmt::Import(d) => d.span,
        }
    }
}

// ---------- Módulos ----------

#[derive(Clone, Debug, PartialEq)]
pub enum ImportTarget {
    /// `import <fs>;`: biblioteca estándar.
    Std(String),
    /// `import "geometria.uwu";`: archivo, relativo al que importa.
    File(String),
    /// `import geometria;`: módulo por nombre.
    Module(String),
}

#[derive(Clone, Debug)]
pub struct ImportDecl {
    pub target: ImportTarget,
    /// `export import ...`: quien importe este archivo también recibe lo importado.
    pub reexport: bool,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct ParamNode {
    pub name: String,
    pub ty: Option<TypeNode>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct FnDecl {
    pub id: usize,
    pub exported: bool,
    pub name: String,
    pub type_params: Vec<String>,
    pub params: Vec<ParamNode>,
    pub ret: Option<TypeNode>,
    pub body: Block,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct TypeDecl {
    pub exported: bool,
    pub name: String,
    pub params: Vec<String>,
    pub ty: TypeNode,
    pub span: Span,
}

// ---------- Patrones (match) ----------

#[derive(Clone, Debug)]
pub enum Pattern {
    Wild { id: usize, ty: Option<TypeNode>, span: Span },
    Bind { id: usize, name: String, ty: Option<TypeNode>, span: Span },
    Lit { value: LitVal, span: Span },
    Object { fields: Vec<(String, Pattern)>, span: Span },
}

impl Pattern {
    pub fn span(&self) -> Span {
        match self {
            Pattern::Wild { span, .. }
            | Pattern::Bind { span, .. }
            | Pattern::Lit { span, .. }
            | Pattern::Object { span, .. } => *span,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Arm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Program {
    pub stmts: Vec<Stmt>,
    /// `export module nombre;` al principio del archivo.
    pub module_name: Option<(String, Span)>,
}
