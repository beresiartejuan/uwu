//! Biblioteca estándar de uwu (sólo Linux por ahora).
//!
//! Cada biblioteca se importa como en C++ (`import <fs>;`) y sus miembros se
//! usan con `::` (`fs::leer(...)`). Las operaciones que pueden fallar devuelven
//! uniones al estilo del lenguaje:
//! `{ ok: true, ... } | { ok: false, error: string }`.
//!
//! Este archivo define *qué* existe y con qué tipo; el intérprete lo ejecuta.

use crate::ast::LitVal;
use crate::types::{arr, lit, t_fn, t_obj, Type, BOOLEAN, NEVER, NUMBER, STRING, VOID};

/// Identifica cada función nativa. El intérprete hace el despacho real.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuiltinFn {
    // prelude: disponibles sin importar nada
    Print,
    Panic,
    Assert,
    // <fs>
    FsLeer,
    FsLeerLineas,
    FsEscribir,
    FsAgregar,
    FsExiste,
    FsEsDir,
    FsEsArchivo,
    FsInfo,
    FsListar,
    FsCrearDir,
    FsBorrar,
    FsCopiar,
    FsMover,
    FsPermisos,
    FsEnlazar,
    // <path>
    PathUnir,
    PathNombre,
    PathBase,
    PathExtension,
    PathPadre,
    PathAbsoluta,
    PathEsAbsoluta,
    // <env>
    EnvObtener,
    EnvDefinir,
    EnvBorrar,
    EnvTodas,
    EnvActualDir,
    EnvCambiarDir,
    EnvHome,
    EnvTempDir,
    // <proc>
    ProcEjecutar,
    ProcEjecutarEntrada,
    ProcShell,
    ProcLanzar,
    ProcEsperar,
    ProcMatar,
    ProcSalir,
    ProcArgs,
    ProcPid,
    ProcPpid,
    // <io>
    IoLeerLinea,
    IoLeerTodo,
    IoEscribir,
    IoError,
    IoEsTerminal,
    // <time>
    TimeAhora,
    TimeAhoraMs,
    TimeReloj,
    TimeDormir,
    TimeFecha,
    TimeFechaLocal,
    TimeIso,
    // <os>
    OsNombre,
    OsArquitectura,
    OsUsuario,
    OsHostname,
    OsKernel,
    OsUid,
    OsCpus,
    OsMemoria,
    OsUptime,
    OsCarga,
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
    MathAleatorio,
    MathAleatorioEntre,
    MathSemilla,
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
            ("leer", F(FsLeer)),
            ("leer_lineas", F(FsLeerLineas)),
            ("escribir", F(FsEscribir)),
            ("agregar", F(FsAgregar)),
            ("existe", F(FsExiste)),
            ("es_dir", F(FsEsDir)),
            ("es_archivo", F(FsEsArchivo)),
            ("info", F(FsInfo)),
            ("listar", F(FsListar)),
            ("crear_dir", F(FsCrearDir)),
            ("borrar", F(FsBorrar)),
            ("copiar", F(FsCopiar)),
            ("mover", F(FsMover)),
            ("permisos", F(FsPermisos)),
            ("enlazar", F(FsEnlazar)),
        ],
    ),
    (
        "path",
        &[
            ("unir", F(PathUnir)),
            ("nombre", F(PathNombre)),
            ("base", F(PathBase)),
            ("extension", F(PathExtension)),
            ("padre", F(PathPadre)),
            ("absoluta", F(PathAbsoluta)),
            ("es_absoluta", F(PathEsAbsoluta)),
            ("SEPARADOR", Member::Str("/")),
        ],
    ),
    (
        "env",
        &[
            ("obtener", F(EnvObtener)),
            ("definir", F(EnvDefinir)),
            ("borrar", F(EnvBorrar)),
            ("todas", F(EnvTodas)),
            ("actual_dir", F(EnvActualDir)),
            ("cambiar_dir", F(EnvCambiarDir)),
            ("home", F(EnvHome)),
            ("temp_dir", F(EnvTempDir)),
        ],
    ),
    (
        "proc",
        &[
            ("ejecutar", F(ProcEjecutar)),
            ("ejecutar_entrada", F(ProcEjecutarEntrada)),
            ("shell", F(ProcShell)),
            ("lanzar", F(ProcLanzar)),
            ("esperar", F(ProcEsperar)),
            ("matar", F(ProcMatar)),
            ("salir", F(ProcSalir)),
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
            ("leer_linea", F(IoLeerLinea)),
            ("leer_todo", F(IoLeerTodo)),
            ("escribir", F(IoEscribir)),
            ("error", F(IoError)),
            ("es_terminal", F(IoEsTerminal)),
        ],
    ),
    (
        "time",
        &[
            ("ahora", F(TimeAhora)),
            ("ahora_ms", F(TimeAhoraMs)),
            ("reloj", F(TimeReloj)),
            ("dormir", F(TimeDormir)),
            ("fecha", F(TimeFecha)),
            ("fecha_local", F(TimeFechaLocal)),
            ("iso", F(TimeIso)),
        ],
    ),
    (
        "os",
        &[
            ("nombre", F(OsNombre)),
            ("arquitectura", F(OsArquitectura)),
            ("usuario", F(OsUsuario)),
            ("hostname", F(OsHostname)),
            ("kernel", F(OsKernel)),
            ("uid", F(OsUid)),
            ("cpus", F(OsCpus)),
            ("memoria", F(OsMemoria)),
            ("uptime", F(OsUptime)),
            ("carga", F(OsCarga)),
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
            ("aleatorio", F(MathAleatorio)),
            ("aleatorio_entre", F(MathAleatorioEntre)),
            ("semilla", F(MathSemilla)),
        ],
    ),
];

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

/// Nombre con el que se muestra una función nativa: `print`, `fs::leer`.
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
    res(vec![("codigo", NUMBER), ("stdout", STRING), ("stderr", STRING)])
}

fn fecha() -> Type {
    t_obj(vec![
        ("anio", NUMBER),
        ("mes", NUMBER),
        ("dia", NUMBER),
        ("hora", NUMBER),
        ("minuto", NUMBER),
        ("segundo", NUMBER),
        ("dia_semana", NUMBER),
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

        FsLeer => t_fn(vec![STRING], res(vec![("contenido", STRING)])),
        FsLeerLineas => t_fn(vec![STRING], res(vec![("lineas", arr(STRING))])),
        FsEscribir | FsAgregar | FsCopiar | FsMover | FsEnlazar => t_fn(vec![STRING, STRING], res(vec![])),
        FsExiste | FsEsDir | FsEsArchivo => t_fn(vec![STRING], BOOLEAN),
        FsInfo => t_fn(
            vec![STRING],
            res(vec![
                ("tamano", NUMBER),
                ("es_dir", BOOLEAN),
                ("es_enlace", BOOLEAN),
                ("modificado", NUMBER),
                ("permisos", NUMBER),
            ]),
        ),
        FsListar => t_fn(vec![STRING], res(vec![("entradas", arr(STRING))])),
        FsCrearDir | FsBorrar => t_fn(vec![STRING], res(vec![])),
        FsPermisos => t_fn(vec![STRING, NUMBER], res(vec![])),

        PathUnir => t_fn(vec![STRING, STRING], STRING),
        PathNombre | PathBase | PathExtension | PathPadre => t_fn(vec![STRING], STRING),
        PathAbsoluta => t_fn(vec![STRING], res(vec![("ruta", STRING)])),
        PathEsAbsoluta => t_fn(vec![STRING], BOOLEAN),

        EnvObtener => t_fn(vec![STRING], opt("valor", STRING)),
        EnvDefinir => t_fn(vec![STRING, STRING], VOID),
        EnvBorrar => t_fn(vec![STRING], VOID),
        EnvTodas => t_fn(vec![], arr(t_obj(vec![("nombre", STRING), ("valor", STRING)]))),
        EnvActualDir | EnvHome | EnvTempDir => t_fn(vec![], STRING),
        EnvCambiarDir => t_fn(vec![STRING], res(vec![])),

        ProcEjecutar => t_fn(vec![STRING, arr(STRING)], salida()),
        ProcEjecutarEntrada => t_fn(vec![STRING, arr(STRING), STRING], salida()),
        ProcShell => t_fn(vec![STRING], salida()),
        ProcLanzar => t_fn(vec![STRING, arr(STRING)], res(vec![("pid", NUMBER)])),
        ProcEsperar => t_fn(vec![NUMBER], res(vec![("codigo", NUMBER)])),
        ProcMatar => t_fn(vec![NUMBER, NUMBER], res(vec![])),
        ProcSalir => t_fn(vec![NUMBER], NEVER),
        ProcArgs => t_fn(vec![], arr(STRING)),
        ProcPid | ProcPpid => t_fn(vec![], NUMBER),

        IoLeerLinea => t_fn(vec![], opt("linea", STRING)),
        IoLeerTodo => t_fn(vec![], STRING),
        IoEscribir | IoError => t_fn(vec![STRING], VOID),
        IoEsTerminal => t_fn(vec![], BOOLEAN),

        TimeAhora | TimeAhoraMs | TimeReloj => t_fn(vec![], NUMBER),
        TimeDormir => t_fn(vec![NUMBER], VOID),
        TimeFecha | TimeFechaLocal => t_fn(vec![NUMBER], fecha()),
        TimeIso => t_fn(vec![NUMBER], STRING),

        OsNombre | OsArquitectura | OsUsuario | OsHostname | OsKernel => t_fn(vec![], STRING),
        OsUid | OsCpus | OsUptime => t_fn(vec![], NUMBER),
        OsMemoria => t_fn(vec![], t_obj(vec![("total", NUMBER), ("disponible", NUMBER)])),
        OsCarga => t_fn(vec![], arr(NUMBER)),

        MathSin | MathCos | MathTan | MathAsin | MathAcos | MathAtan | MathExp | MathLn | MathLog10 => num_to_num(),
        MathAtan2 | MathAleatorioEntre => t_fn(vec![NUMBER, NUMBER], NUMBER),
        MathAleatorio => t_fn(vec![], NUMBER),
        MathSemilla => t_fn(vec![NUMBER], VOID),
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
