# Biblioteca estándar

> Por ahora la biblioteca estándar funciona sólo en **Linux**.

Cada biblioteca se importa con `<...>` y sus miembros se usan con `::`:

```rust
import <fs>;

match fs::leer("/etc/hostname") {
    { ok: true, contenido } => print(contenido.trim()),
    { ok: false, error } => print("no se pudo leer:", error),
}
```

También se puede usar `using namespace fs;` o `using fs::leer;` para no
escribir el prefijo (ver [módulos](modulos.md#qué-se-importa)).

- [Resultados y errores](#resultados-y-errores)
- [`<fs>`](#fs-archivos): archivos
- [`<path>`](#path-rutas): rutas
- [`<env>`](#env-entorno): entorno
- [`<proc>`](#proc-procesos): procesos
- [`<io>`](#io-entrada-y-salida): entrada y salida
- [`<time>`](#time-tiempo): tiempo
- [`<os>`](#os-sistema): sistema
- [`<math>`](#math-matemática): matemática
- [Siempre disponibles](#siempre-disponibles)

## Resultados y errores

Lo que puede fallar devuelve una unión. Como el `match` tiene que ser
exhaustivo, el compilador no te deja olvidarte del caso de error:

```rust
type Resultado = { ok: true, ... } | { ok: false, error: string };
```

En las tablas de abajo, **`Res<{ ... }>`** abrevia
`{ ok: true, ... } | { ok: false, error: string }`, y **`Res`** sola es
`{ ok: true } | { ok: false, error: string }`.

Algunas funciones devuelven algo que puede *no estar* sin que sea un error, como
una variable de entorno que no existe. Esas usan
`{ ok: true, valor: T } | { ok: false }`, sin `error`.

Los mensajes de error del sistema están en castellano e incluyen el `errno`:
`no existe el archivo o directorio (errno 2)`, `permiso denegado (errno 13)`.

## `<fs>`: archivos

| Función | Tipo |
|---|---|
| `leer(ruta)` | `(string) => Res<{ contenido: string }>` |
| `leer_lineas(ruta)` | `(string) => Res<{ lineas: string[] }>` |
| `escribir(ruta, texto)` | `(string, string) => Res`: crea o reemplaza |
| `agregar(ruta, texto)` | `(string, string) => Res`: agrega al final (crea si no existe) |
| `existe(ruta)` | `(string) => boolean` |
| `es_dir(ruta)` | `(string) => boolean` |
| `es_archivo(ruta)` | `(string) => boolean` |
| `info(ruta)` | `(string) => Res<{ tamano, es_dir, es_enlace, modificado, permisos }>` |
| `listar(ruta)` | `(string) => Res<{ entradas: string[] }>`: ordenadas alfabéticamente |
| `crear_dir(ruta)` | `(string) => Res`: crea también los directorios intermedios |
| `borrar(ruta)` | `(string) => Res`: archivos o directorios con todo su contenido |
| `copiar(desde, hasta)` | `(string, string) => Res` |
| `mover(desde, hasta)` | `(string, string) => Res` |
| `permisos(ruta, modo)` | `(string, number) => Res`: como `chmod` |
| `enlazar(destino, enlace)` | `(string, string) => Res`: crea un enlace simbólico |

En `info`: `tamano` está en bytes, `modificado` es la época unix y `permisos`
es el modo (`0o644`). `info` no sigue enlaces, así que `es_enlace` sirve para
detectarlos. `borrar` sobre un enlace borra el enlace, no lo que apunta.

```rust
import <fs>;

fs::escribir("script.sh", "#!/bin/sh\necho hola\n");
fs::permisos("script.sh", 0o755);

match fs::info("script.sh") {
    { ok: true, tamano, permisos } => print(tamano, "bytes, modo", permisos.to_base(8)),
    { ok: false, error } => print(error),
}
```

## `<path>`: rutas

| Miembro | Tipo | Ejemplo con `"/home/juan/foto.jpg"` |
|---|---|---|
| `unir(a, b)` | `(string, string) => string` | `path::unir("/tmp", "x")` → `"/tmp/x"` |
| `nombre(ruta)` | `(string) => string` | `"foto.jpg"` |
| `base(ruta)` | `(string) => string` | `"foto"` |
| `extension(ruta)` | `(string) => string` | `"jpg"` (o `""`) |
| `padre(ruta)` | `(string) => string` | `"/home/juan"` |
| `es_absoluta(ruta)` | `(string) => boolean` | `true` |
| `absoluta(ruta)` | `(string) => Res<{ ruta: string }>` | resuelve `..` y enlaces; la ruta tiene que existir |
| `SEPARADOR` | `string` | `"/"` |

## `<env>`: entorno

| Función | Tipo |
|---|---|
| `obtener(nombre)` | `(string) => { ok: true, valor: string } \| { ok: false }` |
| `definir(nombre, valor)` | `(string, string) => void` |
| `borrar(nombre)` | `(string) => void` |
| `todas()` | `() => { nombre: string, valor: string }[]`: ordenadas por nombre |
| `actual_dir()` | `() => string` |
| `cambiar_dir(ruta)` | `(string) => Res` |
| `home()` | `() => string` |
| `temp_dir()` | `() => string` |

Los cambios de `definir`, `borrar` y `cambiar_dir` los heredan los procesos que
se lancen después con `<proc>`.

## `<proc>`: procesos

`Salida` abrevia `Res<{ codigo: number, stdout: string, stderr: string }>`.

| Miembro | Tipo |
|---|---|
| `ejecutar(programa, args)` | `(string, string[]) => Salida`: corre y espera |
| `ejecutar_entrada(programa, args, entrada)` | `(string, string[], string) => Salida`: manda `entrada` por stdin |
| `shell(comando)` | `(string) => Salida`: corre con `/bin/sh -c` |
| `lanzar(programa, args)` | `(string, string[]) => Res<{ pid: number }>`: no espera |
| `esperar(pid)` | `(number) => Res<{ codigo: number }>`: sólo procesos de `lanzar` |
| `matar(pid, señal)` | `(number, number) => Res` |
| `salir(codigo)` | `(number) => never` |
| `args()` | `() => string[]`: los argumentos después del archivo |
| `pid()` / `ppid()` | `() => number`: pid propio y del proceso padre |
| `SIGINT` `SIGKILL` `SIGTERM` | `number`: 2, 9 y 15 |

- `ejecutar` no pasa por una shell: los argumentos llegan tal cual, sin
  interpretar comillas, `*` ni `|`. Para eso está `shell`.
- `ok: false` significa que el programa **no se pudo arrancar** (por ejemplo,
  porque no existe). Si arrancó y falló, es `ok: true` con un `codigo` distinto de 0.
- Si un proceso muere por una señal, su `codigo` es `128 + señal`, como en
  bash. Por ejemplo, 143 para `SIGTERM`.
- `lanzar` hereda stdin, stdout y stderr: la salida del proceso aparece directo
  en la terminal.
- `matar` no acepta pids `0` ni negativos, que afectan a grupos enteros de procesos.

```rust
import <proc>;

match proc::lanzar("sleep", ["60"]) {
    { ok: true, pid } => {
        proc::matar(pid, proc::SIGTERM);
        match proc::esperar(pid) {
            { ok: true, codigo } => print("terminó con", codigo),   // 143
            { ok: false, error } => print(error),
        }
    }
    { ok: false, error } => print("no se pudo lanzar:", error),
}
```

## `<io>`: entrada y salida

| Función | Tipo |
|---|---|
| `leer_linea()` | `() => { ok: true, linea: string } \| { ok: false }`: `ok: false` al terminar la entrada |
| `leer_todo()` | `() => string`: todo stdin |
| `escribir(texto)` | `(string) => void`: a stdout, sin salto de línea |
| `error(texto)` | `(string) => void`: a stderr, sin salto de línea |
| `es_terminal()` | `() => boolean`: si stdout es una terminal |

```rust
import <io>;

// cuenta las líneas de stdin: `cat archivo | uwu contar.uwu`
let mut n = 0;
loop {
    match io::leer_linea() {
        { ok: true, linea } => n += 1,
        { ok: false } => break,
    }
}
print(n, "líneas");
```

## `<time>`: tiempo

`Fecha` es `{ anio, mes, dia, hora, minuto, segundo, dia_semana: number }`.
`mes` va de 1 a 12 y `dia_semana` de 0 (domingo) a 6.

| Función | Tipo |
|---|---|
| `ahora()` | `() => number`: época unix en segundos, con decimales |
| `ahora_ms()` | `() => number`: época unix en milisegundos |
| `reloj()` | `() => number`: segundos desde que arrancó el programa (para medir) |
| `dormir(segundos)` | `(number) => void`: acepta decimales |
| `fecha(ts)` | `(number) => Fecha`: en UTC |
| `fecha_local(ts)` | `(number) => Fecha`: en la zona horaria del sistema |
| `iso(ts)` | `(number) => string`: ISO 8601 en UTC, como `"2026-11-11T15:00:00Z"` |

```rust
import <time>;

let inicio = time::reloj();
time::dormir(0.5);
print("tardó", time::reloj() - inicio, "segundos");

let hoy = time::fecha_local(time::ahora());
print(hoy.dia.to_string() + "/" + hoy.mes + "/" + hoy.anio);
```

## `<os>`: sistema

| Función | Tipo |
|---|---|
| `nombre()` | `() => string`: `"linux"` |
| `arquitectura()` | `() => string`: `"x86_64"`, `"aarch64"`, ... |
| `usuario()` | `() => string` |
| `hostname()` | `() => string` |
| `kernel()` | `() => string`: versión del kernel |
| `uid()` | `() => number` |
| `cpus()` | `() => number`: núcleos disponibles |
| `memoria()` | `() => { total: number, disponible: number }`: en bytes |
| `uptime()` | `() => number`: segundos desde que arrancó el sistema |
| `carga()` | `() => number[]`: promedios de carga de 1, 5 y 15 minutos |

## `<math>`: matemática

| Miembro | Tipo |
|---|---|
| `PI` `E` | `number` |
| `sin` `cos` `tan` `asin` `acos` `atan` | `(number) => number`: en radianes |
| `atan2(y, x)` | `(number, number) => number` |
| `exp` `ln` `log10` | `(number) => number` |
| `aleatorio()` | `() => number`: en `[0, 1)` |
| `aleatorio_entre(a, b)` | `(number, number) => number`: entero entre `a` y `b`, inclusive |
| `semilla(n)` | `(number) => void`: fija la secuencia, para resultados reproducibles |

Redondeo, raíz, potencias, mínimo y máximo son métodos de `number`:
`x.round()`, `x.sqrt()`, `x.pow(2)`, `x.min(y)` (ver [el lenguaje](lenguaje.md#métodos)).

Los números aleatorios no sirven para criptografía.

## Siempre disponibles

Estas funciones no necesitan `import`:

| Función | Tipo |
|---|---|
| `print(...)` | imprime sus argumentos separados por espacios, con salto de línea |
| `panic(mensaje)` | `(string) => never`: termina el programa con un error |
| `assert(condicion)` | `(boolean) => void`: entra en pánico si es `false` |
