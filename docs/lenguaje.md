# El lenguaje uwu

Referencia del lenguaje. Para módulos mirá [modulos.md](modulos.md) y para la
biblioteca estándar [stdlib.md](stdlib.md).

- [Programas](#programas)
- [Variables](#variables)
- [Tipos](#tipos)
- [Strings e interpolación](#strings-e-interpolación)
- [Tuplas](#tuplas)
- [Desestructuración](#desestructuración)
- [Expresiones y operadores](#expresiones-y-operadores)
- [Control de flujo](#control-de-flujo)
- [Funciones y closures](#funciones-y-closures)
- [Genéricos](#genéricos)
- [`match`](#match)
- [Métodos](#métodos)
- [Errores](#errores)

## Programas

Un programa es una lista de sentencias que se ejecutan de arriba hacia abajo.
Si el archivo de entrada define `fn main()`, se llama automáticamente al final.

```rust
fn main() {
    print("hola");
}
```

Las funciones y los tipos se pueden usar antes de declararlos, así que dos
funciones pueden llamarse entre sí.

Los comentarios son `// de línea` y `/* de bloque */`.

## Variables

```rust
let x = 5;              // inmutable
let mut y = 0;          // mutable
y += x;
let z: number = 10;     // con anotación de tipo
let x = "ahora texto";  // shadowing: una nueva variable con el mismo nombre
```

Modificar una variable inmutable es un error de compilación, también a través
de sus campos (`o.a = 1`) o métodos que la cambian (`v.push(1)`).

Sin anotación, `let x = 5` tiene tipo `number`, no `5`. En los objetos depende
de si se pueden modificar:

```rust
let r = { ok: true };          // { ok: true }: los campos no cambian, el literal se conserva
let mut m = { ok: true };      // { ok: boolean }: se puede hacer `m.ok = false`
```

Así una unión discriminada como `{ ok: true } | { ok: false, error: string }`
sigue siéndolo aunque no se anote el tipo. `let mut q = p;` no ensancha el tipo
de `p`, porque `q` es otra referencia al mismo objeto.

## Tipos

| Tipo | Ejemplos |
|---|---|
| `number` | `42`, `3.14`, `1_000`, `0xff`, `0o755`, `0b1010` |
| `string` | `"hola"`, `"hola {nombre}"` (ver [interpolación](#strings-e-interpolación)) |
| `boolean` | `true`, `false` |
| `void` | lo que devuelve una función sin valor |
| `never` | lo que nunca termina: `panic(...)`, `return`, `loop` sin `break` |
| literales | `"norte"`, `1`, `true` |
| arreglos | `number[]`, `(string \| number)[]` |
| tuplas | `(number, string)`, `(boolean,)` |
| objetos | `{ x: number, y: number }` |
| uniones | `"norte" \| "sur"`, `string \| number` |
| funciones | `(x: number) => string` |

Los alias de tipo pueden ser genéricos y recursivos:

```rust
type Punto = { x: number, y: number };
type Lista<T> = { tipo: "vacia" } | { tipo: "nodo", valor: T, resto: Lista<T> };
```

**Tipado estructural**: un objeto con campos de más sirve donde se espera uno
con menos, como en TypeScript.

```rust
type Punto = { x: number, y: number };

fn norma(p: Punto): number { (p.x * p.x + p.y * p.y).sqrt() }
norma({ x: 3, y: 4, z: 99 });   // 5: `z` sobra, pero no molesta
```

**Inferencia**: `let x = 5` tiene tipo `number`, no el literal `5`. Un arreglo
vacío necesita anotación: `let v: number[] = [];`.

**Sin conversiones implícitas**: `if 1 { }` es un error. Sí se puede concatenar
un `string` con un `number` o un `boolean` usando `+`.

## Strings e interpolación

Lo que va entre llaves dentro de un string se evalúa y se inserta en el texto:

```rust
let nombre = "Ana";
let xs = [1, 2, 3];
print("hola {nombre}, tenés {xs.len()} números: {xs}");   // hola Ana, tenés 3 números: [1, 2, 3]
print("el doble: {xs.map(|x| x * 2)}, {"anidado {nombre.upper()}"}");
print("llaves literales: {{ y }}");                        // llaves literales: { y }
```

- Entre las llaves va cualquier expresión, incluso otros strings.
- Se puede interpolar cualquier valor salvo `void` y las funciones (si te
  olvidaste los `()`, el compilador te avisa). Los strings se insertan tal cual
  y el resto se muestra igual que con `print`.
- `{{` y `}}` escriben una llave literal.
- Los escapes son `\n` `\t` `\r` `\"` `\\` `\0`. Un string no puede ocupar
  varias líneas.

## Tuplas

Como en Rust: un grupo de valores de tipos distintos, de largo fijo.

```rust
fn dividir(a: number, b: number): (number, number) {
    ((a / b).floor(), a % b)
}

let t = dividir(17, 5);
print(t, t.0, t.1);           // (3, 2) 3 2

let mut par: (string, number) = ("edad", 30);
par.1 += 1;
let anidada = ((1, 2), "x");
print(anidada.0.1);           // 2
let uno = (5,);               // con un solo elemento lleva coma
```

- Los elementos se leen con `.0`, `.1`, ... Un índice que no existe es un error
  de compilación.
- `(number, string)` sólo acepta tuplas de exactamente ese largo y esos tipos.
- Igual que los arreglos y los objetos, se comparan por estructura con `==` y se
  comparten por referencia.

## Desestructuración

`let` y `for` aceptan patrones de tuplas y de objetos:

```rust
let (cociente, resto) = (3, 2);
let ((a, b), c) = ((1, 2), "tres");
let { x, y: alto } = { x: 10, y: 20 };   // `alto` vale 20
let (mut total, _) = (0, "ignorado");    // `mut` en cada variable, `_` descarta
total += 1;

for (i, letra) in ["a", "b"].enumerate() {
    print(i, letra);
}

[(1, "a"), (2, "b")].map(|(n, s)| s.repeat(n));     // también en closures
let area = |{ ancho, alto }: { ancho: number, alto: number }| ancho * alto;
```

El patrón tiene que coincidir **siempre**: `let (0, x) = t;` es un error,
porque `t.0` podría no ser `0`. Para eso está `match`.

## Expresiones y operadores

| Precedencia | Operadores |
|---|---|
| 1 (menor) | `..` (rango) |
| 2 | `\|\|` |
| 3 | `&&` |
| 4 | `==` `!=` `<` `>` `<=` `>=` |
| 5 | `+` `-` |
| 6 | `*` `/` `%` |
| 7 (mayor) | `-x` `!x`, llamadas, `.campo`, `[índice]` |

Asignaciones: `=` `+=` `-=` `*=` `/=`.

`==` compara por estructura: `[1, 2] == [1, 2]` es `true`. Si los tipos no
tienen valores en común (`"a" | "b"` contra `"c"`), el compilador avisa que la
comparación siempre da `false`.

Los bloques, `if` y `match` son expresiones: valen lo que vale su última
expresión sin `;`.

```rust
let a = { let b = 2; b * 10 };                   // 20
let c = if a > 5 { "grande" } else { "chico" };
```

## Control de flujo

```rust
if x > 0 { ... } else if x < 0 { ... } else { ... }

while i < 10 { i += 1; }

loop {
    if listo { break; }
    continue;
}

for i in 0..10 { }          // rango, sin incluir el 10
for x in [1, 2, 3] { }      // arreglo
for c in "hola" { }         // caracteres de un string
for (i, x) in xs.enumerate() { }   // con desestructuración
for let t in tareas { t.hecha = true; }   // `let`: la variable es mutable
for let (i, t) in tareas.enumerate() { }  // todas las variables del patrón son mutables
```

La variable del `for` es inmutable. Con `for let x in ...` se puede modificar,
como cualquier `let mut`. Los objetos se comparten por referencia, así que
`t.hecha = true` cambia el elemento del arreglo.

`return` sale de la función actual. `break` y `continue` fuera de un bucle son
errores de compilación.

## Funciones y closures

```rust
fn sumar(a: number, b: number): number {
    a + b          // sin `;`: es el valor de retorno
}

fn saludar(nombre: string) {      // sin tipo de retorno: void
    print("hola", nombre);
}
```

Los parámetros siempre llevan tipo. Si la función dice que devuelve algo, el
cuerpo tiene que producirlo; olvidarse el valor (o dejar un `;` de más) es un
error con ayuda.

Las closures usan la sintaxis de Rust y capturan su entorno:

```rust
fn sumador(n: number): (x: number) => number { |x| x + n }
let mas5 = sumador(5);
mas5(10);                           // 15

let doble = |x: number| x * 2;      // tipo explícito
[1, 2].map(|x| x * 2);              // tipo inferido por el contexto
xs.enumerate().map(|(i, x)| i * x); // desestructurando el argumento
```

## Genéricos

```rust
fn primero<T>(xs: T[]): T { xs[0] }
let s: string = primero(["a", "b"]);   // T = string, inferido

fn vacio<T>(): T[] { [] }
let v = vacio::<number>();             // turbofish cuando no se puede inferir
```

## `match`

```rust
match valor {
    1 => "uno",                          // literal
    n: number if n < 0 => "negativo",    // filtro por tipo + guarda
    { tipo: "circulo", radio } => ...,   // objeto: liga `radio`
    { tipo: "punto", x: 0 } => ...,      // patrones anidados
    (0, y) => ...,                       // tupla
    _ => "otro",                         // comodín
}
```

- **Exhaustividad**: si falta un caso, el compilador dice cuál
  (`falta cubrir { a: 2, b: false }`). Las guardas `if` no cuentan, y para
  `number` o `string` hace falta un comodín.
- **Narrowing**: dentro de cada brazo, las variables tienen el tipo más preciso.
  En `{ tipo: "circulo", radio }`, `radio` existe seguro.
- **Brazos imposibles**: un patrón que nunca coincide es un error, y uno que
  nunca se ejecuta (porque los anteriores ya cubren todo) da un warning.

Acceder a un campo que no está en todas las variantes de una unión es un error:
primero hay que distinguirlas con `match`.

## Métodos

| Tipo | Métodos |
|---|---|
| `T[]` | `len()` `is_empty()` `push(x)` `pop()` `contains(x)` `index_of(x)` `reverse()` `slice(desde, hasta?)` `sort()` `join(sep)` `map(f)` `filter(f)` `any(f)` `all(f)` `fold(inicial, f)` `sum()` `enumerate()` `zip(otro)` |
| `string` | `len()` `is_empty()` `upper()` `lower()` `trim()` `contains(s)` `starts_with(s)` `ends_with(s)` `index_of(s)` `split(sep)` `lines()` `chars()` `slice(desde, hasta?)` `replace(de, a)` `repeat(n)` `to_number()` |
| `number` | `to_string()` `to_base(b)` `abs()` `floor()` `ceil()` `round()` `sqrt()` `pow(n)` `min(n)` `max(n)` |
| `boolean` | `to_string()` |
| rango | `to_array()` |

Detalles:

- `push` y `pop` modifican el arreglo, así que piden `let mut`. `reverse`,
  `sort` y `slice` devuelven uno nuevo.
- `sort` sólo existe para arreglos de `number` o de `string`. `sum` sólo para `number`.
- `slice` acepta índices negativos, que cuentan desde el final: `"hola".slice(-2, 4)` → `"la"`.
  Si se omite `hasta`, llega hasta el final: `"hola mundo".slice(5)` → `"mundo"`.
- `len`, `index_of` y `slice` en strings cuentan caracteres, no bytes.
- `"42".to_number()` devuelve `{ ok: true, valor: number } | { ok: false }`.
- `255.to_base(16)` → `"ff"`; la base va de 2 a 36.
- `enumerate()` devuelve `(number, T)[]` con cada elemento y su índice, y
  `a.zip(b)` devuelve `(A, B)[]` con los pares, tan largo como el más corto.

## Errores

**De compilación**: se detectan antes de ejecutar nada, con el formato de rustc
y casi siempre con una ayuda. Si hay alguno, el programa no corre.

**En ejecución** (pánicos): índice fuera de rango, división por cero,
`panic(msg)`, `assert(cond)` que falla, recursión infinita. El programa termina
con código 101 y muestra dónde ocurrió.

Las funciones globales `print(...)`, `panic(msg)` y `assert(cond)` están
siempre disponibles.
