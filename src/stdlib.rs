//! Biblioteca estándar de uwu (sólo Linux por ahora).
//!
//! Cada biblioteca se importa como en C++ (`import <fs>;`) y sus miembros se
//! usan con `::` (`fs::read(...)`). Las operaciones que pueden fallar devuelven
//! uniones al estilo del lenguaje:
//! `{ ok: true, ... } | { ok: false, error: string }`.
//!
//! Este archivo define *qué* existe y con qué tipo; el intérprete lo ejecuta.

use crate::syntax::ast::LitVal;
use crate::types::{arr, lit, t_fn, t_obj, Type, BOOLEAN, NEVER, NUMBER, STRING, VOID};

/// Identifica cada función nativa. El intérprete hace el despacho real.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuiltinFn {
    // prelude: disponibles sin importar nada
    Print,
    Panic,
    Assert,
    // <fs>
    FsRead,
    FsReadLines,
    FsWrite,
    FsAppend,
    FsExists,
    FsIsDir,
    FsIsFile,
    FsStat,
    FsListDir,
    FsCreateDir,
    FsRemove,
    FsCopy,
    FsRename,
    FsSetPermissions,
    FsSymlink,
    // <path>
    PathJoin,
    PathFileName,
    PathStem,
    PathExtension,
    PathParent,
    PathAbsolute,
    PathIsAbsolute,
    // <env>
    EnvGet,
    EnvSet,
    EnvRemove,
    EnvVars,
    EnvCurrentDir,
    EnvChangeDir,
    EnvHome,
    EnvTempDir,
    // <proc>
    ProcRun,
    ProcRunWithInput,
    ProcShell,
    ProcSpawn,
    ProcWait,
    ProcKill,
    ProcExit,
    ProcArgs,
    ProcPid,
    ProcPpid,
    // <io>
    IoReadLine,
    IoReadAll,
    IoWrite,
    IoWriteErr,
    IoIsTerminal,
    // <time>
    TimeNow,
    TimeNowMs,
    TimeElapsed,
    TimeSleep,
    TimeDate,
    TimeLocalDate,
    TimeIso,
    // <os>
    OsName,
    OsArch,
    OsUser,
    OsHostname,
    OsKernel,
    OsUid,
    OsCpus,
    OsMemory,
    OsUptime,
    OsLoadAvg,
    // <math>
    MathSin,
    MathCos,
    MathTan,
    MathAsin,
    MathAcos,
    MathAtan,
    MathAtan2,
    MathExp,
    MathLn,
    MathLog10,
    MathRandom,
    MathRandomRange,
    MathSeed,
}

/// Un miembro de una biblioteca: una función nativa o una constante.
#[derive(Clone, Copy, Debug)]
pub enum Member {
    Fn(BuiltinFn),
    Num(f64),
    Str(&'static str),
}

use BuiltinFn::*;
use Member::Fn as F;

/// Funciones globales sueltas (el "prelude").
pub const GLOBALS: &[(&str, BuiltinFn)] = &[("print", Print), ("panic", Panic), ("assert", Assert)];

/// Bibliotecas estándar: (nombre, miembros).
pub const MODULES: &[(&str, &[(&str, Member)])] = &[
    (
        "fs",
        &[
            ("read", F(FsRead)),
            ("read_lines", F(FsReadLines)),
            ("write", F(FsWrite)),
            ("append", F(FsAppend)),
            ("exists", F(FsExists)),
            ("is_dir", F(FsIsDir)),
            ("is_file", F(FsIsFile)),
            ("stat", F(FsStat)),
            ("list_dir", F(FsListDir)),
            ("create_dir", F(FsCreateDir)),
            ("remove", F(FsRemove)),
            ("copy", F(FsCopy)),
            ("rename", F(FsRename)),
            ("set_permissions", F(FsSetPermissions)),
            ("symlink", F(FsSymlink)),
        ],
    ),
    (
        "path",
        &[
            ("join", F(PathJoin)),
            ("file_name", F(PathFileName)),
            ("stem", F(PathStem)),
            ("extension", F(PathExtension)),
            ("parent", F(PathParent)),
            ("absolute", F(PathAbsolute)),
            ("is_absolute", F(PathIsAbsolute)),
            ("SEPARATOR", Member::Str("/")),
        ],
    ),
    (
        "env",
        &[
            ("get", F(EnvGet)),
            ("set", F(EnvSet)),
            ("remove", F(EnvRemove)),
            ("vars", F(EnvVars)),
            ("current_dir", F(EnvCurrentDir)),
            ("change_dir", F(EnvChangeDir)),
            ("home", F(EnvHome)),
            ("temp_dir", F(EnvTempDir)),
        ],
    ),
    (
        "proc",
        &[
            ("run", F(ProcRun)),
            ("run_with_input", F(ProcRunWithInput)),
            ("shell", F(ProcShell)),
            ("spawn", F(ProcSpawn)),
            ("wait", F(ProcWait)),
            ("kill", F(ProcKill)),
            ("exit", F(ProcExit)),
            ("args", F(ProcArgs)),
            ("pid", F(ProcPid)),
            ("ppid", F(ProcPpid)),
            ("SIGINT", Member::Num(2.0)),
            ("SIGKILL", Member::Num(9.0)),
            ("SIGTERM", Member::Num(15.0)),
        ],
    ),
    (
        "io",
        &[
            ("read_line", F(IoReadLine)),
            ("read_all", F(IoReadAll)),
            ("write", F(IoWrite)),
            ("write_err", F(IoWriteErr)),
            ("is_terminal", F(IoIsTerminal)),
        ],
    ),
    (
        "time",
        &[
            ("now", F(TimeNow)),
            ("now_ms", F(TimeNowMs)),
            ("elapsed", F(TimeElapsed)),
            ("sleep", F(TimeSleep)),
            ("date", F(TimeDate)),
            ("local_date", F(TimeLocalDate)),
            ("iso", F(TimeIso)),
        ],
    ),
    (
        "os",
        &[
            ("name", F(OsName)),
            ("arch", F(OsArch)),
            ("user", F(OsUser)),
            ("hostname", F(OsHostname)),
            ("kernel", F(OsKernel)),
            ("uid", F(OsUid)),
            ("cpus", F(OsCpus)),
            ("memory", F(OsMemory)),
            ("uptime", F(OsUptime)),
            ("load_avg", F(OsLoadAvg)),
        ],
    ),
    (
        "math",
        &[
            ("PI", Member::Num(std::f64::consts::PI)),
            ("E", Member::Num(std::f64::consts::E)),
            ("sin", F(MathSin)),
            ("cos", F(MathCos)),
            ("tan", F(MathTan)),
            ("asin", F(MathAsin)),
            ("acos", F(MathAcos)),
            ("atan", F(MathAtan)),
            ("atan2", F(MathAtan2)),
            ("exp", F(MathExp)),
            ("ln", F(MathLn)),
            ("log10", F(MathLog10)),
            ("random", F(MathRandom)),
            ("random_range", F(MathRandomRange)),
            ("seed", F(MathSeed)),
        ],
    ),
];

/// Los nombres en castellano que tenía la biblioteca hasta la 0.2: (biblioteca, antes, ahora).
/// Sólo se usan para avisar cómo se llama ahora cada función.
const RENAMED: &[(&str, &str, &str)] = &[
    ("fs", "leer", "read"),
    ("fs", "leer_lineas", "read_lines"),
    ("fs", "escribir", "write"),
    ("fs", "agregar", "append"),
    ("fs", "existe", "exists"),
    ("fs", "es_dir", "is_dir"),
    ("fs", "es_archivo", "is_file"),
    ("fs", "info", "stat"),
    ("fs", "listar", "list_dir"),
    ("fs", "crear_dir", "create_dir"),
    ("fs", "borrar", "remove"),
    ("fs", "copiar", "copy"),
    ("fs", "mover", "rename"),
    ("fs", "permisos", "set_permissions"),
    ("fs", "enlazar", "symlink"),
    ("path", "unir", "join"),
    ("path", "nombre", "file_name"),
    ("path", "base", "stem"),
    ("path", "padre", "parent"),
    ("path", "absoluta", "absolute"),
    ("path", "es_absoluta", "is_absolute"),
    ("path", "SEPARADOR", "SEPARATOR"),
    ("env", "obtener", "get"),
    ("env", "definir", "set"),
    ("env", "borrar", "remove"),
    ("env", "todas", "vars"),
    ("env", "actual_dir", "current_dir"),
    ("env", "cambiar_dir", "change_dir"),
    ("proc", "ejecutar", "run"),
    ("proc", "ejecutar_entrada", "run_with_input"),
    ("proc", "lanzar", "spawn"),
    ("proc", "esperar", "wait"),
    ("proc", "matar", "kill"),
    ("proc", "salir", "exit"),
    ("io", "leer_linea", "read_line"),
    ("io", "leer_todo", "read_all"),
    ("io", "escribir", "write"),
    ("io", "error", "write_err"),
    ("io", "es_terminal", "is_terminal"),
    ("time", "ahora", "now"),
    ("time", "ahora_ms", "now_ms"),
    ("time", "reloj", "elapsed"),
    ("time", "dormir", "sleep"),
    ("time", "fecha", "date"),
    ("time", "fecha_local", "local_date"),
    ("os", "nombre", "name"),
    ("os", "arquitectura", "arch"),
    ("os", "usuario", "user"),
    ("os", "memoria", "memory"),
    ("os", "carga", "load_avg"),
    ("math", "aleatorio", "random"),
    ("math", "aleatorio_entre", "random_range"),
    ("math", "semilla", "seed"),
];

/// `renamed("fs", "leer")` → `Some("read")`.
pub fn renamed(ns: &str, old: &str) -> Option<&'static str> {
    RENAMED.iter().find(|(n, o, _)| *n == ns && *o == old).map(|(_, _, new)| *new)
}

pub fn module(name: &str) -> Option<&'static [(&'static str, Member)]> {
    MODULES.iter().find(|(n, _)| *n == name).map(|(_, m)| *m)
}

pub fn lookup(ns: &str, name: &str) -> Option<Member> {
    module(ns)?.iter().find(|(n, _)| *n == name).map(|(_, m)| *m)
}

/// Para nombres parecidos o en castellano: `tiempo` → `time`.
pub fn suggest(name: &str) -> Option<&'static str> {
    let alias = match name {
        "tiempo" | "fecha" | "chrono" => "time",
        "archivos" | "archivo" | "filesystem" => "fs",
        "ruta" | "rutas" => "path",
        "entorno" => "env",
        "proceso" | "procesos" | "process" => "proc",
        "sistema" | "system" => "os",
        "mate" | "matematica" | "matemáticas" | "cmath" => "math",
        "iostream" | "stdio" => "io",
        _ => "",
    };
    if !alias.is_empty() {
        return Some(alias);
    }
    MODULES.iter().map(|(n, _)| *n).find(|n| {
        let (a, b) = (n.len() as isize, name.len() as isize);
        (a - b).abs() <= 1 && (n.starts_with(name) || name.starts_with(n))
    })
}

pub fn module_names() -> String {
    MODULES.iter().map(|(n, _)| format!("<{}>", n)).collect::<Vec<_>>().join(" ")
}

/// Nombre con el que se muestra una función nativa: `print`, `fs::read`.
pub fn builtin_name(b: BuiltinFn) -> String {
    if let Some((n, _)) = GLOBALS.iter().find(|(_, g)| *g == b) {
        return n.to_string();
    }
    for (ns, members) in MODULES {
        for (n, m) in members.iter() {
            if matches!(m, Member::Fn(f) if *f == b) {
                return format!("{}::{}", ns, n);
            }
        }
    }
    "nativa".into()
}

// ---------- tipos ----------

fn ok_obj(extra: Vec<(&str, Type)>) -> Type {
    let mut fields = vec![("ok", lit(LitVal::Bool(true)))];
    fields.extend(extra);
    t_obj(fields)
}

fn err_obj() -> Type {
    t_obj(vec![("ok", lit(LitVal::Bool(false))), ("error", STRING)])
}

/// `{ ok: true, ...extra } | { ok: false, error: string }`
fn res(extra: Vec<(&str, Type)>) -> Type {
    Type::Union(vec![ok_obj(extra), err_obj()])
}

/// `{ ok: true, campo: T } | { ok: false }`: algo que puede no estar, sin ser un error.
fn opt(field: &str, t: Type) -> Type {
    Type::Union(vec![ok_obj(vec![(field, t)]), t_obj(vec![("ok", lit(LitVal::Bool(false)))])])
}

fn salida() -> Type {
    res(vec![("code", NUMBER), ("stdout", STRING), ("stderr", STRING)])
}

fn fecha() -> Type {
    t_obj(vec![
        ("year", NUMBER),
        ("month", NUMBER),
        ("day", NUMBER),
        ("hour", NUMBER),
        ("minute", NUMBER),
        ("second", NUMBER),
        ("weekday", NUMBER),
    ])
}

fn num_to_num() -> Type {
    t_fn(vec![NUMBER], NUMBER)
}

pub fn builtin_type(b: BuiltinFn) -> Type {
    match b {
        Print => Type::Fn { params: vec![Type::Err], ret: Box::new(VOID), type_params: vec![], variadic: true },
        Panic => t_fn(vec![STRING], NEVER),
        Assert => t_fn(vec![BOOLEAN], VOID),

        FsRead => t_fn(vec![STRING], res(vec![("content", STRING)])),
        FsReadLines => t_fn(vec![STRING], res(vec![("lines", arr(STRING))])),
        FsWrite | FsAppend | FsCopy | FsRename | FsSymlink => t_fn(vec![STRING, STRING], res(vec![])),
        FsExists | FsIsDir | FsIsFile => t_fn(vec![STRING], BOOLEAN),
        FsStat => t_fn(
            vec![STRING],
            res(vec![
                ("size", NUMBER),
                ("is_dir", BOOLEAN),
                ("is_symlink", BOOLEAN),
                ("modified", NUMBER),
                ("permissions", NUMBER),
            ]),
        ),
        FsListDir => t_fn(vec![STRING], res(vec![("entries", arr(STRING))])),
        FsCreateDir | FsRemove => t_fn(vec![STRING], res(vec![])),
        FsSetPermissions => t_fn(vec![STRING, NUMBER], res(vec![])),

        PathJoin => t_fn(vec![STRING, STRING], STRING),
        PathFileName | PathStem | PathExtension | PathParent => t_fn(vec![STRING], STRING),
        PathAbsolute => t_fn(vec![STRING], res(vec![("path", STRING)])),
        PathIsAbsolute => t_fn(vec![STRING], BOOLEAN),

        EnvGet => t_fn(vec![STRING], opt("value", STRING)),
        EnvSet => t_fn(vec![STRING, STRING], VOID),
        EnvRemove => t_fn(vec![STRING], VOID),
        EnvVars => t_fn(vec![], arr(t_obj(vec![("name", STRING), ("value", STRING)]))),
        EnvCurrentDir | EnvHome | EnvTempDir => t_fn(vec![], STRING),
        EnvChangeDir => t_fn(vec![STRING], res(vec![])),

        ProcRun => t_fn(vec![STRING, arr(STRING)], salida()),
        ProcRunWithInput => t_fn(vec![STRING, arr(STRING), STRING], salida()),
        ProcShell => t_fn(vec![STRING], salida()),
        ProcSpawn => t_fn(vec![STRING, arr(STRING)], res(vec![("pid", NUMBER)])),
        ProcWait => t_fn(vec![NUMBER], res(vec![("code", NUMBER)])),
        ProcKill => t_fn(vec![NUMBER, NUMBER], res(vec![])),
        ProcExit => t_fn(vec![NUMBER], NEVER),
        ProcArgs => t_fn(vec![], arr(STRING)),
        ProcPid | ProcPpid => t_fn(vec![], NUMBER),

        IoReadLine => t_fn(vec![], opt("line", STRING)),
        IoReadAll => t_fn(vec![], STRING),
        IoWrite | IoWriteErr => t_fn(vec![STRING], VOID),
        IoIsTerminal => t_fn(vec![], BOOLEAN),

        TimeNow | TimeNowMs | TimeElapsed => t_fn(vec![], NUMBER),
        TimeSleep => t_fn(vec![NUMBER], VOID),
        TimeDate | TimeLocalDate => t_fn(vec![NUMBER], fecha()),
        TimeIso => t_fn(vec![NUMBER], STRING),

        OsName | OsArch | OsUser | OsHostname | OsKernel => t_fn(vec![], STRING),
        OsUid | OsCpus | OsUptime => t_fn(vec![], NUMBER),
        OsMemory => t_fn(vec![], t_obj(vec![("total", NUMBER), ("available", NUMBER)])),
        OsLoadAvg => t_fn(vec![], arr(NUMBER)),

        MathSin | MathCos | MathTan | MathAsin | MathAcos | MathAtan | MathExp | MathLn | MathLog10 => num_to_num(),
        MathAtan2 | MathRandomRange => t_fn(vec![NUMBER, NUMBER], NUMBER),
        MathRandom => t_fn(vec![], NUMBER),
        MathSeed => t_fn(vec![NUMBER], VOID),
    }
}

pub fn member_type(m: &Member) -> Type {
    match m {
        Member::Fn(b) => builtin_type(*b),
        Member::Num(_) => NUMBER,
        Member::Str(_) => STRING,
    }
}

/// Tipos de las funciones del prelude, para el checker.
pub fn global_types() -> Vec<(&'static str, Type)> {
    GLOBALS.iter().map(|(n, b)| (*n, builtin_type(*b))).collect()
}
