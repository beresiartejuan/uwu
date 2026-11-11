# Changelog

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
