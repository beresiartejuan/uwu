# uwu 🦀✨

**Un lenguaje de programación con la sintaxis de Rust y el sistema de tipos de TypeScript.**

```rust
type Figura =
    | { tipo: "circulo", radio: number }
    | { tipo: "rectangulo", ancho: number, alto: number };

fn area(f: Figura): number {
    match f {
        { tipo: "circulo", radio } => 3.14159 * radio * radio,
        { tipo: "rectangulo", ancho, alto } => ancho * alto,
    }
}
```

La idea: tomar lo mejor de los dos mundos.

| De **Rust** | De **TypeScript** |
|---|---|
| Variables inmutables por defecto (`let` / `let mut`) | Uniones: `string \| number` |
| Todo es una expresión (`if`, `match`, bloques) | Tipos literales: `"norte" \| "sur"` |
| `match` con chequeo de exhaustividad | Tipos estructurales: `{ x: number, y: number }` |
| Closures `\|x\| x + 1` y turbofish `f::<T>()` | Alias genéricos: `type Option<T> = ...` |
| Errores del compilador claros y con ayudas | Inferencia de tipos |

Y a diferencia de TypeScript, el sistema de tipos es **estricto**: no hay `any`, no hay conversiones implícitas (`if 1 {}` es un error) y los tipos se chequean antes de ejecutar.

## Instalación

```bash
git clone https://github.com/beresiartejuan/uwu.git
cd uwu
npm install
npm run build
```

## Uso

```bash
node dist/main.js examples/hola.uwu        # chequea tipos y ejecuta
node dist/main.js check examples/hola.uwu  # sólo chequea tipos
npm test                                   # corre los tests
```

Con `npm link` queda disponible el comando `uwu`.

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

### Genéricos: `Option` y `Result` hechos con uniones

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

### `match` con narrowing

Dentro de cada brazo el tipo se estrecha, igual que en TypeScript:

```rust
fn describir(x: number | string): string {
    match x {
        n: number => "número " + (n * 2),
        s: string => "texto de largo " + s.len(),
    }
}
```

### Errores

El compilador encuentra los errores antes de ejecutar y explica cómo arreglarlos:

```
error: `match` no exhaustivo: falta cubrir `{ tipo: "cuadrado", lado: number }`
  --> examples/errores.uwu:14:11
   |
14 |     match f {
   |           ^
   = ayuda: agregá un brazo para esos casos, o uno comodín: `_ => ...`

error: no se puede modificar `x` porque es inmutable
 --> examples/errores.uwu:9:1
  |
9 | x = 10;
  | ^
  = ayuda: declarala con `let mut x`
```

Mirá [`examples/`](examples/) para más ejemplos (árboles binarios genéricos, `Result`, etc).

## Referencia

**Tipos:** `number`, `string`, `boolean`, `void`, `never`, literales (`"a"`, `1`, `true`), arreglos (`T[]`), objetos (`{ a: T }`), uniones (`A | B`), funciones (`(x: A) => B`), alias genéricos y recursivos (`type Lista<T> = ...`).

**Control de flujo:** `if`/`else`, `match` (con guardas `if`), `while`, `loop`, `for x in ...` (arreglos, strings, rangos `0..10`), `break`, `continue`, `return`.

**Funciones globales:** `print(...)`, `panic(msg)`, `assert(cond)`.

**Métodos:**

| Tipo | Métodos |
|---|---|
| `T[]` | `len` `push` `pop` `contains` `reverse` `join` `map` `filter` `any` `all` `fold` `sum` |
| `string` | `len` `upper` `lower` `trim` `contains` `starts_with` `ends_with` `split` `chars` `repeat` |
| `number` | `to_string` `abs` `floor` `ceil` `round` `sqrt` `pow` `min` `max` |
| rango | `to_array` |

Si el archivo define `fn main()`, se ejecuta automáticamente al final.

## Arquitectura

```
src/
├── lexer.ts        texto → tokens
├── parser.ts       tokens → AST (descenso recursivo + Pratt para operadores)
├── ast.ts          definición del árbol de sintaxis
├── types.ts        representación de tipos, asignabilidad, uniones
├── checker.ts      chequeo e inferencia de tipos, genéricos, exhaustividad del match
├── interpreter.ts  ejecuta el AST (tree-walking)
├── diagnostics.ts  formato de errores estilo rustc
├── index.ts        API: compile() y run()
└── main.ts         CLI
```

## Próximos pasos

- [ ] Módulos: `use "otro_archivo";`
- [ ] Narrowing en `if` (`if f.tipo == "circulo" { f.radio }`)
- [ ] `impl` para asociar métodos a tipos
- [ ] Tuplas y desestructuración en `let`
- [ ] Interpolación de strings: `"hola {nombre}"`
- [ ] Compilar a JavaScript en vez de interpretar
- [ ] REPL
- [ ] Extensión de VS Code con resaltado de sintaxis
