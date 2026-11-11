//! De texto a AST: el lexer arma los tokens y el parser el árbol de sintaxis.

pub mod ast;
pub mod lexer;
pub mod parser;

pub use lexer::Span;
