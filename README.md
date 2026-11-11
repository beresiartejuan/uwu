# uwu 🦀✨

[![CI](https://github.com/beresiartejuan/uwu/actions/workflows/ci.yml/badge.svg)](https://github.com/beresiartejuan/uwu/actions/workflows/ci.yml)

**Un lenguaje de programación con la sintaxis de Rust y el sistema de tipos de TypeScript.**
El intérprete está escrito en Rust, sin dependencias externas.

```rust
import <fs>;

type Figura =
    | { tipo: "circulo", radio: number }
    | { tipo: "rectangulo", ancho: number, alto: number };

fn area(f: Figura): number {
    match f {
        { tipo: "circulo", radio } => 3.14159 * radio * radio,
        { tipo: "rectangulo", ancho, alto } => ancho * alto,
    }
}

match fs::read("figuras.txt") {
    { ok: true, content } => print(content),
    { ok: false, error } => print("no se pudo leer:", error),
}
```

## ¿Por qué?

uwu toma lo mejor de los dos lenguajes:

| De **Rust** | De **TypeScript** |
|---|---|
| Variables inmutables por defecto (`let` / `let mut`) | Uniones: `string \| number` |
| Todo es una expresión (`if`, `match`, bloques) | Tipos literales: `"norte" \| "sur"` |
| `match` con chequeo de exhaustividad | Tipos estructurales: `{ x: number, y: number }` |
| Closures `\|x\| x + 1` y turbofish `f::<T>()` | Alias genéricos: `type Option<T> = ...` |
| Errores del compilador claros y con ayudas | Inferencia de tipos |
| Tuplas y desestructuración: `let (a, b) = ...` | Strings con interpolación: `"hola {nombre}"` |

Y a diferencia de TypeScript, el sistema de tipos es **estricto**: no hay `any`,
no hay conversiones implícitas (`if 1 {}` es un error) y los tipos se chequean
antes de ejecutar.

Además trae **módulos y namespaces al estilo C++20** (`import <fs>;`,
`export fn ...`, `export namespace geo { ... }`), una **biblioteca estándar**
para trabajar con el sistema operativo y un **REPL** para probar código al toque.

## Instalación

Necesitás Linux y [Rust](https://rustup.rs/) 1.83 o más nuevo.

```bash
git clone https://github.com/beresiartejuan/uwu.git
cd uwu
cargo install --path .
```

Eso deja el comando `uwu` disponible. Si sólo querés probarlo sin instalarlo,
usá `cargo run --release -- examples/hola.uwu`.

## Uso

```bash
uwu programa.uwu               # chequea los tipos y ejecuta
uwu check programa.uwu         # sólo chequea los tipos
uwu programa.uwu uno dos       # los argumentos extra llegan por proc::args()
uwu                            # abre el REPL
```

### REPL

`uwu` sin argumentos (o `uwu repl`) abre una consola interactiva. Cada línea se
chequea y se ejecuta enseguida, y se muestra el valor de la última expresión:

```
uwu> let xs = [3, 1, 2]
uwu> fn doble(n: number): number { n * 2 }
uwu> xs.map(doble).sort()
[2, 4, 6]
uwu> :tipo xs.enumerate()
(number, number)[]
uwu> let s: string = xs
error: tipos incompatibles: se esperaba `string`, se encontró `number[]`
 --> <repl>:1:17
  |
1 | let s: string = xs;
  |                 ^^
```

- Lo definido se conserva entre líneas, y se puede redefinir.
- Una entrada con errores no cambia nada.
- El `;` final es opcional.
- Si falta cerrar un `{`, el REPL sigue leyendo.

`:ayuda` lista los comandos. No tiene historial propio: para usar las flechas,
abrilo con `rlwrap uwu`.

| Variable de entorno | Para qué sirve |
|---|---|
| `UWU_PATH` | carpetas extra (separadas por `:`) donde buscar `import modulo;` |
| `NO_COLOR` | desactiva los colores en los errores |

El programa termina con código `0` si todo salió bien, `1` si hubo errores de
compilación y `101` si hubo un pánico en ejecución.

## Tour rápido

### Variables y expresiones

```rust
let nombre = "Juan";           // inmutable, tipo inferido: string
let mut contador = 0;          // mutable
contador += 1;

// if, match y los bloques devuelven valores
let saludo = if contador > 0 { "hola" } else { "chau" };
let doble = { let x = 21; x * 2 };
```

### Funciones y closures

```rust
// La última expresión sin `;` es el valor de retorno
fn factorial(n: number): number {
    if n <= 1 { 1 } else { n * factorial(n - 1) }
}

let pares = [1, 2, 3, 4].filter(|n| n % 2 == 0);
let suma = [1, 2, 3].fold(0, |acc, n| acc + n);
```

### Tuplas, desestructuración e interpolación

```rust
fn min_max(xs: number[]): (number, number) {
    let ordenados = xs.sort();
    (ordenados[0], ordenados[ordenados.len() - 1])
}

let (min, max) = min_max([4, 8, 1]);
print("entre {min} y {max}");            // entre 1 y 8

let { x, y } = { x: 1, y: 2 };
for (i, nombre) in ["ana", "beto"].enumerate() {
    print("{i}: {nombre.upper()}");
}
```

### Uniones, genéricos y `match`

`Option` y `Result` no son parte del lenguaje: se construyen con uniones.

```rust
type Option<T> = { some: true, value: T } | { some: false };

fn buscar<T>(items: T[], pred: (item: T) => boolean): Option<T> {
    for item in items {
        if pred(item) { return { some: true, value: item }; }
    }
    { some: false }
}

match buscar([12, 17, 25], |e| e >= 18) {
    { some: true, value } => print("encontré", value),
    { some: false } => print("nada"),
}
```

Dentro de cada brazo del `match` el tipo se estrecha, igual que en TypeScript:

```rust
fn describir(x: number | string): string {
    match x {
        n: number => "número " + (n * 2),
        s: string => "texto de largo " + s.len(),
    }
}
```

### Módulos

Se importan y exportan como en C++20. Lo que no tiene `export` es privado del archivo.

```rust
// geometria.uwu
export module geometria;

export type Punto = { x: number, y: number };

fn cuadrado(n: number): number { n * n }   // privada

export fn distancia(a: Punto, b: Punto): number {
    (cuadrado(a.x - b.x) + cuadrado(a.y - b.y)).sqrt()
}
```

```rust
// main.uwu
import geometria;           // busca geometria.uwu en la misma carpeta
import "util/texto.uwu";    // o por ruta relativa

print(distancia({ x: 0, y: 0 }, { x: 3, y: 4 }));   // 5
```

Un `namespace` agrupa declaraciones bajo un prefijo, igual que en C++:

```rust
// formas.uwu
export namespace circulo {
    fn area(r: number): number { 3.14159 * r * r }
}
```

```rust
// main.uwu
import formas;

print(circulo::area(2));        // con el prefijo
using circulo::area;
print(area(2));                 // o sin él, después de `using`
```

### Biblioteca estándar

Cada biblioteca se importa con `<...>` y sus miembros se usan con `::`, como `std::` en C++.
Lo que puede fallar devuelve `{ ok: true, ... } | { ok: false, error: string }`,
así que el compilador te obliga a manejar el error.

```rust
import <proc>;
import <time>;

match proc::shell("ls /tmp | wc -l") {
    { ok: true, stdout } => print("archivos en /tmp:", stdout.trim()),
    { ok: false, error } => print("falló:", error),
}
print("hoy:", time::iso(time::now()));
```

| Biblioteca | Para qué |
|---|---|
| `<fs>` | leer, escribir, listar, copiar, borrar archivos; permisos y enlaces |
| `<path>` | unir y descomponer rutas |
| `<env>` | variables de entorno y directorio actual |
| `<proc>` | ejecutar programas, shell, procesos en segundo plano, señales |
| `<io>` | leer de stdin, escribir en stdout y stderr |
| `<time>` | hora actual, fechas, medir y esperar |
| `<os>` | información del sistema: usuario, kernel, memoria, CPUs, carga |
| `<math>` | trigonometría, logaritmos, números aleatorios |

`print`, `panic` y `assert` están siempre disponibles sin importar nada.

## Errores del compilador

uwu encuentra los errores antes de ejecutar y explica cómo arreglarlos:

```
error: `match` no exhaustivo: falta cubrir `{ tipo: "cuadrado", lado: number }`
  --> examples/errores.uwu:14:11
   |
14 |     match f {                    // falta el caso "cuadrado"
   |           ^
   = ayuda: agregá un brazo para esos casos, o uno comodín: `_ => ...`

error: la función `doble` debe devolver `number`, pero su cuerpo no produce un valor
  --> examples/errores.uwu:19:1
   |
19 | fn doble(n: number): number {
   | ^^^^^^^^
   = ayuda: quitá el `;` de la última expresión: en uwu, la última expresión de un bloque es su valor
```

## Documentación

- [**El lenguaje**](docs/lenguaje.md): tipos, control de flujo, funciones, `match` y métodos.
- [**Módulos**](docs/modulos.md): `import`, `export`, `using` y cómo se resuelven los archivos.
- [**Biblioteca estándar**](docs/stdlib.md): todas las funciones con sus tipos.
- [**Ejemplos**](examples/):
  [`hola.uwu`](examples/hola.uwu) (lo básico),
  [`figuras.uwu`](examples/figuras.uwu) (uniones y `match`),
  [`genericos.uwu`](examples/genericos.uwu) (`Option`, `Result`, árbol binario),
  [`sistema.uwu`](examples/sistema.uwu) (biblioteca estándar),
  [`inventario.uwu`](examples/inventario.uwu) (tuplas, desestructuración, interpolación y namespaces),
  [`tareas.uwu`](examples/tareas.uwu) (una lista de tareas en la terminal, guardada en un archivo),
  [`modulos/`](examples/modulos/) (un proyecto con varios archivos) y
  [`errores.uwu`](examples/errores.uwu) (los mensajes del compilador).

## Desarrollo

```bash
cargo test                          # todos los tests
cargo test tipos::                  # sólo los de tests/uwu/tipos.rs
cargo clippy --all-targets          # lint (la CI no acepta warnings)
cargo fmt                           # formato
```

Las decisiones de diseño, los detalles de la implementación y las convenciones
del proyecto están en la [ficha del proyecto](FICHA.md).

El intérprete es un pipeline clásico:

```
texto ─► lexer ─► parser ─► modules ─► checker ─► interpreter
                   (AST)    (imports)   (tipos)    (tree-walking)
```

```
src/
├── main.rs           CLI
├── lib.rs            API pública: compile(), run(), run_file() y repl::Repl
├── syntax/
│   ├── lexer.rs      texto → tokens
│   ├── parser.rs     tokens → AST (descenso recursivo + Pratt para operadores)
│   └── ast.rs        definición del árbol de sintaxis
├── modules.rs        resuelve los `import`, detecta ciclos, ordena los módulos
├── types.rs          representación de tipos, asignabilidad, uniones
├── checker/          chequeo e inferencia de tipos
│   ├── mod.rs        estado del checker, módulos, exports y namespaces
│   ├── stmt.rs       sentencias y funciones
│   ├── expr.rs       expresiones
│   ├── call.rs       llamadas, genéricos, métodos y closures
│   └── pattern.rs    `match` y exhaustividad
├── interpreter/      ejecuta el AST
│   ├── mod.rs        sentencias y expresiones
│   ├── value.rs      valores y entornos
│   ├── method.rs     métodos de arreglos, strings y números
│   └── native.rs     la implementación de la biblioteca estándar (Linux)
├── stdlib.rs         qué hay en cada biblioteca estándar y con qué tipo
├── repl.rs           el REPL: cada entrada continúa a la anterior
└── diagnostics.rs    formato de errores estilo rustc
tests/uwu/            tests de integración, un archivo por tema
```

Los cambios de cada versión están en el [CHANGELOG](CHANGELOG.md).

## Próximos pasos

- [x] Tuplas y desestructuración en `let`
- [x] Interpolación de strings: `"hola {nombre}"`
- [x] `export namespace` para agrupar exports bajo un prefijo
- [x] REPL
- [ ] Narrowing en `if` (`if f.tipo == "circulo" { f.radio }`)
- [ ] `impl` para asociar métodos a tipos
- [ ] Patrones con `|` en `match` (`(0, _) | (_, 0) => ...`)
- [ ] Historial y autocompletado en el REPL
- [ ] Soporte para macOS y Windows en la biblioteca estándar
- [ ] Extensión de VS Code con resaltado de sintaxis

## Licencia

[ISC](LICENSE) © Juan Beresiarte
