# Changelog

## Sin publicar

### Lenguaje

- **Tuplas** al estilo Rust: `(1, "a")`, con tipo `(number, string)`, acceso
  `t.0` y comparación estructural. Una tupla de un elemento se escribe `(x,)`.
- **Desestructuración** en `let` y `for`, de tuplas y de objetos:
  `let (a, b) = t;`, `let { x, y: alto } = p;`, `for (i, x) in xs.enumerate()`.
  El patrón tiene que coincidir siempre, y `mut` se pone en cada variable:
  `let (mut a, b) = ...`.
- Patrones de tuplas en `match`, con chequeo de exhaustividad.
- **Interpolación de strings**: `"hola {nombre}, tenés {xs.len()} mensajes"`.
  `{{` y `}}` escriben llaves literales.
- Métodos nuevos de arreglos: `enumerate()` y `zip(otro)`.
- Los closures desestructuran sus parámetros: `xs.enumerate().map(|(i, x)| ...)`,
  `|{ x, y }| x + y`, `|(a, b): (number, number)| a * b`.
- `for let x in xs` hace mutable la variable del `for` (o todas las de un patrón:
  `for let (i, t) in ...`). `for mut x` da un error que sugiere `for let`.
- `slice(desde, hasta?)`: el final es opcional, `"hola mundo".slice(5)` → `"mundo"`.
- `let` conserva los campos literales de los objetos: `let r = { ok: true }` es
  `{ ok: true }`, no `{ ok: boolean }`. Arregla `match` que daban «no exhaustivo»
  con uniones discriminadas sin anotar. `let mut` sobre un objeto nuevo sigue
  ensanchando los campos para poder modificarlos.

> **Cambio incompatible:** las llaves dentro de los strings ahora interpolan.
> Un string que tenía `{` o `}` literales tiene que escribirlos como `{{` y `}}`.
> `namespace` pasa a ser palabra reservada.

### Biblioteca estándar

> **Cambio incompatible:** las funciones y los campos que devuelven pasan al
> inglés: `fs::leer` → `fs::read`, `proc::ejecutar` → `proc::run`,
> `{ ok: true, contenido }` → `{ ok: true, content }`, `time::fecha` → `time::date`
> con `{ year, month, day, ... }`, etc. Usar un nombre viejo da un error que
> dice cómo se llama ahora. La lista completa está en [docs/stdlib.md](docs/stdlib.md).

### Módulos

- **`namespace` y `export namespace`**: agrupan funciones, constantes y tipos
  bajo un prefijo (`geo::area`, `geo::Punto`), como en C++.
- `using namespace geo;` y `using geo::area;` también funcionan con ellos.
- `export let` acepta desestructuración: `export let (A, B) = (1, 2);`.

### REPL (nuevo)

- `uwu` sin argumentos, o `uwu repl`, abre una consola interactiva.
- Muestra el valor de la última expresión y deja redefinir funciones y tipos.
- Una entrada con errores no cambia el estado.
- Comandos `:tipo`, `:limpiar`, `:ayuda` y `:salir`.
- Lee varias líneas si falta cerrar un `{`, `(` o `[`.
- La API está en `uwu::repl::Repl`.

### Arreglos

- `uwu programa.uwu | head` ya no entra en pánico cuando se cierra la salida:
  el programa termina en silencio, con código 0.

### Código

- `src/` está organizado por etapa: `syntax/` (lexer, parser y AST), `checker/`
  e `interpreter/`, con el checker y el intérprete repartidos en varios archivos.
- Los tests de integración pasaron de un solo archivo a `tests/uwu/`, uno por tema.
- Para quien use `uwu` como biblioteca: `uwu::lexer`, `uwu::parser` y `uwu::ast`
  ahora son `uwu::syntax::lexer`, `uwu::syntax::parser` y `uwu::syntax::ast`.

## 0.2.0 (2026-11-11)

Reescritura completa en Rust, con módulos y biblioteca estándar.

### Intérprete

- El intérprete pasa de TypeScript (Node.js) a **Rust**, sin dependencias externas.
  Ya no hace falta `npm`: se compila con `cargo build` o se instala con `cargo install --path .`.
- Se mantiene todo el lenguaje de la versión 0.1: los mismos tipos, la misma
  semántica y los mismos mensajes de error.
- Literales numéricos hexadecimales (`0xff`), octales (`0o755`) y binarios (`0b1010`).
- Métodos nuevos:
  - de `string`: `to_number`, `replace`, `slice`, `lines`, `index_of`, `is_empty`;
  - de arreglos: `sort`, `slice`, `index_of`, `is_empty`;
  - de `number`: `to_base`.

### Módulos (nuevo)

- `import` y `export` con la sintaxis de los módulos de C++20:
  `import <fs>;`, `import "archivo.uwu";`, `import modulo;`,
  `export fn` / `let` / `type`, `export module nombre;`, `export import ...;`.
- `using namespace fs;` y `using fs::leer;`.
- `$UWU_PATH` para buscar módulos en otras carpetas.
- Detección de importaciones circulares. Cada módulo se ejecuta una sola vez.
  Los errores se reportan en el archivo donde ocurren.

### Biblioteca estándar (nuevo, sólo Linux)

- Ocho bibliotecas: `<fs>`, `<path>`, `<env>`, `<proc>`, `<io>`, `<time>`, `<os>` y `<math>`.
- Se importan explícitamente y se usan con `::` (`fs::leer(...)`).
- Lo que puede fallar devuelve `{ ok: true, ... } | { ok: false, error: string }`,
  con los mensajes de error del sistema en castellano.

### Proyecto

- Documentación separada en [`docs/`](docs/): el lenguaje, los módulos y la biblioteca estándar.
- Integración continua en GitHub Actions: formato, clippy y tests.
- Licencia ISC en [`LICENSE`](LICENSE).

## 0.1.0 (2026-11-10)

Segunda versión del lenguaje, escrita en TypeScript: lexer, parser, type checker
e intérprete nuevos. Sintaxis de Rust con el sistema de tipos de TypeScript
(uniones, tipos literales, objetos estructurales, genéricos) y `match` con
chequeo de exhaustividad.

## v3.0.x (2024)

Prototipo original en TypeScript, con otra sintaxis (`ctx`, `use`). Quedó en el
historial de git hasta el commit `b0e881e`.
