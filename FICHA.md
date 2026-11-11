# Ficha del proyecto: uwu

> Documento de contexto para colaboradores y asistentes: qué es el lenguaje,
> cómo está armado el intérprete, qué decisiones están tomadas y qué sigue.
> Para *usar* el lenguaje, mirá el [README](README.md) y [`docs/`](docs/).

## 1. Qué es

**uwu** es un lenguaje interpretado con la **sintaxis de Rust** y el **sistema
de tipos de TypeScript**, pero estricto: sin `any` ni conversiones implícitas,
y con los tipos chequeados antes de ejecutar.

```rust
type Resultado = { ok: true, valor: number } | { ok: false, error: string };

fn dividir(a: number, b: number): Resultado {
    if b == 0 { return { ok: false, error: "división por cero" }; }
    { ok: true, valor: a / b }
}

match dividir(10, 2) {
    { ok: true, valor } => print(valor),
    { ok: false, error } => print(error),
}
```

## 2. Stack y comandos

| | |
|---|---|
| Lenguaje del intérprete | Rust, edición 2021, **versión mínima 1.83** |
| Dependencias | ninguna: sólo `std`, y la libc del sistema vía `extern "C"` |
| Plataforma | **sólo Linux**; `lib.rs` tiene un `compile_error!` para el resto |
| CI | GitHub Actions: `fmt --check`, `clippy -D warnings`, tests, ejemplos y MSRV |

```bash
cargo build --release               # binario en target/release/uwu
cargo test                          # tests de integración (tests/uwu.rs)
cargo clippy --all-targets          # tiene que quedar sin warnings
cargo fmt                           # max_width = 120 (rustfmt.toml)
cargo run -- examples/hola.uwu      # ejecutar
cargo run -- check archivo.uwu      # sólo chequear tipos
```

## 3. Arquitectura

```
texto ─► lexer ─► parser ─► modules ─► checker ─► interpreter
```

| Archivo | Responsabilidad |
|---|---|
| `lexer.rs` | Texto → tokens. Cada `Span` lleva el índice del archivo (`span.file`). |
| `parser.rs` | Tokens → AST. Descenso recursivo + Pratt para operadores. Los ids de funciones y patrones son `file << 32 \| n`, únicos en todo el programa. |
| `ast.rs` | El árbol de sintaxis. |
| `modules.rs` | Sigue los `import`, resuelve rutas, detecta ciclos y devuelve los módulos **en orden topológico** (la entrada queda última). |
| `types.rs` | Representación de tipos, `mk_union` (normaliza), `is_assignable`, `resolve` de alias. |
| `checker.rs` | Chequeo e inferencia. Se llama una vez por módulo (`check_module`) en orden; guarda lo que exporta cada uno. |
| `interpreter.rs` | Tree-walking. Ejecuta los módulos en el mismo orden y copia los exports al entorno de quien importa. |
| `stdlib.rs` | **Qué** hay en la biblioteca estándar: nombres, miembros y tipos. |
| `native.rs` | **Cómo** se ejecuta la biblioteca estándar (Linux). |
| `diagnostics.rs` | Formato de errores estilo rustc. |
| `lib.rs` | API pública: `compile`, `run` (código suelto) y `run_file`. |
| `main.rs` | CLI. |

### Detalles importantes

- **Alias de tipo por módulo**: se guardan como `Nombre#módulo` en `TypeCtx`, y
  cada módulo tiene su `type_scope` (nombre visible → clave). `print()` oculta el
  `#...`. Por eso dos módulos pueden tener tipos privados homónimos.
- **Parámetros de tipo frescos**: al aplicar una función genérica se renombran
  `T` → `T'3` para no chocar con los del llamador. `clean_name` los muestra como `T`.
- **Valores**: números `f64` formateados como JS (`5`, no `5.0`). Arreglos y
  objetos son `Rc<RefCell<...>>`: se comparten por referencia, como en JS.
- **Pila**: el intérprete corre en un hilo con 256 MB de pila y un límite de
  profundidad de 10 000 llamadas, para que la recursión infinita sea un pánico
  y no un segfault.
- **Agregar una función a la stdlib**: (1) una variante en `BuiltinFn`, (2) una
  entrada en `MODULES`, (3) su tipo en `builtin_type`, (4) la implementación en
  `native.rs` y (5) un test y su línea en `docs/stdlib.md`.

## 4. Decisiones tomadas

| Tema | Decisión |
|---|---|
| Sintaxis | La de Rust: `fn`, `let`/`let mut`, `match`, closures `\|x\|`, todo es expresión. |
| Tipos | Los de TypeScript, pero sólidos: uniones, literales, estructurales, genéricos con inferencia. |
| `Option` / `Result` | No son parte del lenguaje: se arman con uniones discriminadas. |
| Errores recuperables | Uniones `{ ok: true, ... } \| { ok: false, error: string }`, nunca excepciones. |
| Módulos | Sintaxis de C++20 (`import <x>;`, `export fn`, `export module`), con nombres exportados directo en el ámbito. |
| Biblioteca estándar | Con prefijo `::` como `std::` en C++, e importación explícita. |
| Prelude | `print`, `panic` y `assert` no necesitan `import`. |
| Idioma | Palabras clave en inglés, igual que Rust; biblioteca estándar, mensajes y docs en castellano rioplatense. |
| Plataforma | Linux primero. |

## 5. Historia

1. **v3.0.x (2024)**: prototipo en TypeScript con otra sintaxis (`ctx`, `use`),
   lexer con `moo` y sin parser completo. Quedó en el historial hasta `b0e881e`.
2. **0.1.0**: reescritura en TypeScript con la sintaxis y los tipos actuales.
3. **0.2.0**: reescritura en Rust, módulos estilo C++ y biblioteca estándar.

Ver [CHANGELOG.md](CHANGELOG.md).

## 6. Qué sigue

- Narrowing en `if` (`if f.tipo == "circulo" { f.radio }`).
- `impl` para asociar métodos a tipos.
- Tuplas y desestructuración en `let`.
- Interpolación de strings (`"hola {nombre}"`).
- `export namespace` para agrupar exports bajo un prefijo.
- Biblioteca estándar en macOS y Windows (lo específico de Linux está en `native.rs`:
  `/proc`, `kill`, `getuid`, `localtime_r`, permisos Unix).
- REPL y extensión de VS Code.

## 7. Preguntas abiertas

- ¿La biblioteca estándar debería poder escribirse en parte en uwu, con `export`?
- El prototipo tenía mensajes de error con estética "uwu/owo" (*"OnO Wow! Your
  code has an error, programmer-kun..."*). Desde la 0.1 los mensajes son
  sobrios, al estilo rustc. ¿Vale la pena recuperar algo de esa identidad?
- ¿`print` debería pasar a `<io>` y exigir `import`, para ser 100% como C++?

## 8. Convenciones

- Código formateado con `cargo fmt` y sin warnings de clippy (lo exige la CI).
- Los comentarios del código y los mensajes al usuario están en castellano.
- Todo cambio visible lleva un test en `tests/uwu.rs` y su documentación en `docs/`.
- Los ejemplos de `examples/` se ejecutan en la CI; `errores.uwu` tiene exactamente 7 errores a propósito.
