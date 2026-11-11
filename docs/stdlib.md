# Biblioteca estándar

> Por ahora la biblioteca estándar funciona sólo en **Linux**.

Cada biblioteca se importa con `<...>` y sus miembros se usan con `::`:

```rust
import <fs>;

match fs::read("/etc/hostname") {
    { ok: true, content } => print(content.trim()),
    { ok: false, error } => print("no se pudo leer:", error),
}
```

También se puede usar `using namespace fs;` o `using fs::read;` para no
escribir el prefijo (ver [módulos](modulos.md#qué-se-importa)).

Los nombres de las bibliotecas, de sus funciones y de los campos que devuelven
están en inglés, como en la mayoría de los lenguajes. Hasta la 0.2 estaban en
castellano (`fs::leer`, `proc::ejecutar`); si usás un nombre viejo, el
compilador te dice cómo se llama ahora.

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
`{ ok: true, value: T } | { ok: false }`, sin `error`.

Los mensajes de error del sistema están en castellano e incluyen el `errno`:
`no existe el archivo o directorio (errno 2)`, `permiso denegado (errno 13)`.

## `<fs>`: archivos

| Función | Tipo |
|---|---|
| `read(path)` | `(string) => Res<{ content: string }>` |
| `read_lines(path)` | `(string) => Res<{ lines: string[] }>` |
| `write(path, text)` | `(string, string) => Res`: crea o reemplaza |
| `append(path, text)` | `(string, string) => Res`: agrega al final (crea si no existe) |
| `exists(path)` | `(string) => boolean` |
| `is_dir(path)` | `(string) => boolean` |
| `is_file(path)` | `(string) => boolean` |
| `stat(path)` | `(string) => Res<{ size, is_dir, is_symlink, modified, permissions }>` |
| `list_dir(path)` | `(string) => Res<{ entries: string[] }>`: ordenadas alfabéticamente |
| `create_dir(path)` | `(string) => Res`: crea también los directorios intermedios |
| `remove(path)` | `(string) => Res`: archivos o directorios con todo su contenido |
| `copy(from, to)` | `(string, string) => Res` |
| `rename(from, to)` | `(string, string) => Res`: mueve o renombra |
| `set_permissions(path, mode)` | `(string, number) => Res`: como `chmod` |
| `symlink(target, link)` | `(string, string) => Res`: crea un enlace simbólico |

En `stat`: `size` está en bytes, `modified` es la época unix y `permissions`
es el modo (`0o644`). `stat` no sigue enlaces, así que `is_symlink` sirve para
detectarlos. `remove` sobre un enlace borra el enlace, no lo que apunta.

```rust
import <fs>;

fs::write("script.sh", "#!/bin/sh\necho hola\n");
fs::set_permissions("script.sh", 0o755);

match fs::stat("script.sh") {
    { ok: true, size, permissions } => print(size, "bytes, modo", permissions.to_base(8)),
    { ok: false, error } => print(error),
}
```

## `<path>`: rutas

| Miembro | Tipo | Ejemplo con `"/home/juan/foto.jpg"` |
|---|---|---|
| `join(a, b)` | `(string, string) => string` | `path::join("/tmp", "x")` → `"/tmp/x"` |
| `file_name(path)` | `(string) => string` | `"foto.jpg"` |
| `stem(path)` | `(string) => string` | `"foto"` |
| `extension(path)` | `(string) => string` | `"jpg"` (o `""`) |
| `parent(path)` | `(string) => string` | `"/home/juan"` |
| `is_absolute(path)` | `(string) => boolean` | `true` |
| `absolute(path)` | `(string) => Res<{ path: string }>` | resuelve `..` y enlaces; la ruta tiene que existir |
| `SEPARATOR` | `string` | `"/"` |

## `<env>`: entorno

| Función | Tipo |
|---|---|
| `get(name)` | `(string) => { ok: true, value: string } \| { ok: false }` |
| `set(name, value)` | `(string, string) => void` |
| `remove(name)` | `(string) => void` |
| `vars()` | `() => { name: string, value: string }[]`: ordenadas por nombre |
| `current_dir()` | `() => string` |
| `change_dir(path)` | `(string) => Res` |
| `home()` | `() => string` |
| `temp_dir()` | `() => string` |

Los cambios de `set`, `remove` y `change_dir` los heredan los procesos que
se lancen después con `<proc>`.

## `<proc>`: procesos

`Output` abrevia `Res<{ code: number, stdout: string, stderr: string }>`.

| Miembro | Tipo |
|---|---|
| `run(program, args)` | `(string, string[]) => Output`: corre y espera |
| `run_with_input(program, args, input)` | `(string, string[], string) => Output`: manda `input` por stdin |
| `shell(command)` | `(string) => Output`: corre con `/bin/sh -c` |
| `spawn(program, args)` | `(string, string[]) => Res<{ pid: number }>`: no espera |
| `wait(pid)` | `(number) => Res<{ code: number }>`: sólo procesos de `spawn` |
| `kill(pid, signal)` | `(number, number) => Res` |
| `exit(code)` | `(number) => never` |
| `args()` | `() => string[]`: los argumentos después del archivo |
| `pid()` / `ppid()` | `() => number`: pid propio y del proceso padre |
| `SIGINT` `SIGKILL` `SIGTERM` | `number`: 2, 9 y 15 |

- `run` no pasa por una shell: los argumentos llegan tal cual, sin
  interpretar comillas, `*` ni `|`. Para eso está `shell`.
- `ok: false` significa que el programa **no se pudo arrancar** (por ejemplo,
  porque no existe). Si arrancó y falló, es `ok: true` con un `code` distinto de 0.
- Si un proceso muere por una señal, su `code` es `128 + señal`, como en
  bash. Por ejemplo, 143 para `SIGTERM`.
- `spawn` hereda stdin, stdout y stderr: la salida del proceso aparece directo
  en la terminal.
- `kill` no acepta pids `0` ni negativos, que afectan a grupos enteros de procesos.

```rust
import <proc>;

match proc::spawn("sleep", ["60"]) {
    { ok: true, pid } => {
        proc::kill(pid, proc::SIGTERM);
        match proc::wait(pid) {
            { ok: true, code } => print("terminó con", code),   // 143
            { ok: false, error } => print(error),
        }
    }
    { ok: false, error } => print("no se pudo lanzar:", error),
}
```

## `<io>`: entrada y salida

| Función | Tipo |
|---|---|
| `read_line()` | `() => { ok: true, line: string } \| { ok: false }`: `ok: false` al terminar la entrada |
| `read_all()` | `() => string`: todo stdin |
| `write(text)` | `(string) => void`: a stdout, sin salto de línea |
| `write_err(text)` | `(string) => void`: a stderr, sin salto de línea |
| `is_terminal()` | `() => boolean`: si stdout es una terminal |

```rust
import <io>;

// cuenta las líneas de stdin: `cat archivo | uwu contar.uwu`
let mut n = 0;
loop {
    match io::read_line() {
        { ok: true, line } => n += 1,
        { ok: false } => break,
    }
}
print(n, "líneas");
```

## `<time>`: tiempo

`Date` es `{ year, month, day, hour, minute, second, weekday: number }`.
`month` va de 1 a 12 y `weekday` de 0 (domingo) a 6.

| Función | Tipo |
|---|---|
| `now()` | `() => number`: época unix en segundos, con decimales |
| `now_ms()` | `() => number`: época unix en milisegundos |
| `elapsed()` | `() => number`: segundos desde que arrancó el programa (para medir) |
| `sleep(seconds)` | `(number) => void`: acepta decimales |
| `date(ts)` | `(number) => Date`: en UTC |
| `local_date(ts)` | `(number) => Date`: en la zona horaria del sistema |
| `iso(ts)` | `(number) => string`: ISO 8601 en UTC, como `"2026-11-11T15:00:00Z"` |

```rust
import <time>;

let inicio = time::elapsed();
time::sleep(0.5);
print("tardó", time::elapsed() - inicio, "segundos");

let { day, month, year } = time::local_date(time::now());
print("{day}/{month}/{year}");
```

## `<os>`: sistema

| Función | Tipo |
|---|---|
| `name()` | `() => string`: `"linux"` |
| `arch()` | `() => string`: `"x86_64"`, `"aarch64"`, ... |
| `user()` | `() => string` |
| `hostname()` | `() => string` |
| `kernel()` | `() => string`: versión del kernel |
| `uid()` | `() => number` |
| `cpus()` | `() => number`: núcleos disponibles |
| `memory()` | `() => { total: number, available: number }`: en bytes |
| `uptime()` | `() => number`: segundos desde que arrancó el sistema |
| `load_avg()` | `() => number[]`: promedios de carga de 1, 5 y 15 minutos |

## `<math>`: matemática

| Miembro | Tipo |
|---|---|
| `PI` `E` | `number` |
| `sin` `cos` `tan` `asin` `acos` `atan` | `(number) => number`: en radianes |
| `atan2(y, x)` | `(number, number) => number` |
| `exp` `ln` `log10` | `(number) => number` |
| `random()` | `() => number`: en `[0, 1)` |
| `random_range(a, b)` | `(number, number) => number`: entero entre `a` y `b`, inclusive |
| `seed(n)` | `(number) => void`: fija la secuencia, para resultados reproducibles |

Redondeo, raíz, potencias, mínimo y máximo son métodos de `number`:
`x.round()`, `x.sqrt()`, `x.pow(2)`, `x.min(y)` (ver [el lenguaje](lenguaje.md#métodos)).

Los números aleatorios no sirven para criptografía.

## Siempre disponibles

Estas funciones no necesitan `import`:

| Función | Tipo |
|---|---|
| `print(...)` | imprime sus argumentos separados por espacios, con salto de línea |
| `panic(message)` | `(string) => never`: termina el programa con un error |
| `assert(condition)` | `(boolean) => void`: entra en pánico si es `false` |
