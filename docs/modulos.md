# Módulos

uwu usa la sintaxis de los módulos de C++20: cada archivo es un módulo,
`import` trae lo que otro exporta y sólo lo marcado con `export` se ve desde
afuera.

```rust
// geometria.uwu
export module geometria;          // opcional: declara el nombre del módulo

import <math>;

export type Punto = { x: number, y: number };
export let ORIGEN: Punto = { x: 0, y: 0 };

fn cuadrado(n: number): number { n * n }   // privada

export fn distancia(a: Punto, b: Punto): number {
    (cuadrado(a.x - b.x) + cuadrado(a.y - b.y)).sqrt()
}
```

```rust
// main.uwu
import geometria;

let p: Punto = { x: 3, y: 4 };
print(distancia(ORIGEN, p));     // 5
print(cuadrado(2));              // error: no existe `cuadrado` en este ámbito
```

## Sintaxis

| Sintaxis | Qué hace |
|---|---|
| `import <fs>;` | importa una [biblioteca estándar](stdlib.md) |
| `import "ruta/archivo.uwu";` | importa un archivo, relativo al que importa |
| `import nombre;` | importa el módulo `nombre` (ver [cómo se busca](#cómo-se-buscan-los-archivos)) |
| `export fn ...` | exporta una función |
| `export let ...` | exporta una constante (también con `let mut`) |
| `export type ...` | exporta un tipo |
| `export module nombre;` | declara el nombre del módulo |
| `export import ...;` | re-exporta: quien te importe también recibe eso |
| `using namespace fs;` | trae todos los miembros de una biblioteca al ámbito |
| `using fs::leer;` | trae un solo miembro |

`import` y `export` sólo pueden ir en el nivel superior del archivo, no dentro
de funciones ni bloques. `export module` tiene que ser la primera línea.

## Qué se importa

**De un archivo**, los nombres exportados entran directo al ámbito, como en
C++. No hace falta un prefijo.

**De una biblioteca estándar**, los miembros quedan bajo su nombre y se usan con
`::`, como `std::` en C++. `using` permite traerlos sin prefijo.

```rust
import <math>;
import <path>;

print(math::PI);

using namespace math;      // ahora `PI`, `sin`, ... sin prefijo
using path::extension;     // sólo `extension`
print(PI, extension("foto.png"));
```

## Cómo se buscan los archivos

- `import "util/texto.uwu";` es relativo a la carpeta **del archivo que importa**,
  no a la carpeta desde donde se ejecuta `uwu`. La extensión `.uwu` es opcional.
- `import geometria;` busca `geometria.uwu` en la carpeta del archivo que importa
  y después en cada carpeta de la variable de entorno `UWU_PATH` (separadas por `:`).
  Si el archivo declara `export module X;`, `X` tiene que coincidir con el nombre importado.

```bash
UWU_PATH=~/uwu/lib:/opt/uwu uwu main.uwu
```

## Reglas

- **Una sola vez**: cada módulo se carga y se ejecuta una sola vez, aunque lo
  importen varios archivos.
- **Orden**: los módulos se ejecutan antes que quien los importa.
- **Sin ciclos**: `a.uwu → b.uwu → a.uwu` es un error que muestra la cadena completa.
- **Sin choques**: si dos módulos exportan el mismo nombre y un archivo importa
  los dos, es un error.
- **De sólo lectura**: lo importado no se puede reasignar. Para cambiar el
  estado de otro módulo, exportá una función que lo haga.
- **Tipos privados**: dos módulos pueden tener tipos privados con el mismo
  nombre sin chocar.
- **`main`**: `fn main()` sólo se ejecuta en el archivo de entrada.
- **Errores**: los errores de cada módulo se reportan en su propio archivo.

## Re-exportar

`export import` arma un módulo que agrupa a otros:

```rust
// todo.uwu
export import "geometria.uwu";
export import "texto.uwu";
export import <math>;
```

```rust
// main.uwu
import "todo.uwu";    // trae lo de geometria, texto y también math::
```

Hay un ejemplo completo en [`examples/modulos/`](../examples/modulos/).
