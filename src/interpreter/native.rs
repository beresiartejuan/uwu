//! Implementación de la biblioteca estándar (Linux).
//!
//! Lo que expone `std` se usa directamente; lo que no (señales, uid, hora
//! local) se pide a la libc, que en Linux ya está enlazada con todo binario.

use super::{display, rt, varr, vobj, vstr, Control, EResult, Interpreter, Value};
use crate::stdlib::BuiltinFn;
use crate::syntax::Span;
use std::io::{IsTerminal, Read, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{Command, Output, Stdio};

mod sys {
    /// `struct tm` de glibc/musl.
    #[repr(C)]
    pub struct Tm {
        pub tm_sec: i32,
        pub tm_min: i32,
        pub tm_hour: i32,
        pub tm_mday: i32,
        pub tm_mon: i32,
        pub tm_year: i32,
        pub tm_wday: i32,
        pub tm_yday: i32,
        pub tm_isdst: i32,
        pub tm_gmtoff: i64,
        pub tm_zone: *const u8,
    }

    extern "C" {
        pub fn kill(pid: i32, sig: i32) -> i32;
        pub fn getuid() -> u32;
        pub fn tzset();
        pub fn localtime_r(t: *const i64, out: *mut Tm) -> *mut Tm;
    }
}

// ---------- utilidades ----------

fn ok_val<'p>(extra: Vec<(&str, Value<'p>)>) -> Value<'p> {
    let mut fields = vec![("ok", Value::Bool(true))];
    fields.extend(extra);
    vobj(fields)
}

fn err_val<'p>(msg: String) -> Value<'p> {
    vobj(vec![("ok", Value::Bool(false)), ("error", vstr(msg))])
}

fn none_val<'p>() -> Value<'p> {
    vobj(vec![("ok", Value::Bool(false))])
}

/// Mensajes de error del sistema en castellano.
fn io_err(e: &std::io::Error) -> String {
    use std::io::ErrorKind::*;
    let msg = match e.kind() {
        NotFound => "no existe el archivo o directorio",
        PermissionDenied => "permiso denegado",
        AlreadyExists => "ya existe",
        IsADirectory => "es un directorio",
        NotADirectory => "no es un directorio",
        DirectoryNotEmpty => "el directorio no está vacío",
        InvalidData => "el contenido no es texto UTF-8 válido",
        ReadOnlyFilesystem => "el sistema de archivos es de sólo lectura",
        StorageFull => "no queda espacio en el disco",
        _ => return e.to_string(),
    };
    match e.raw_os_error() {
        Some(code) => format!("{} (errno {})", msg, code),
        None => msg.to_string(),
    }
}

fn unit_res<'p>(r: std::io::Result<()>) -> Value<'p> {
    match r {
        Ok(()) => ok_val(vec![]),
        Err(e) => err_val(io_err(&e)),
    }
}

fn exit_code(status: std::process::ExitStatus) -> f64 {
    // como en bash: muerto por la señal N → 128 + N
    status.code().unwrap_or_else(|| 128 + status.signal().unwrap_or(0)) as f64
}

fn salida<'p>(r: std::io::Result<Output>) -> Value<'p> {
    match r {
        Ok(o) => ok_val(vec![
            ("codigo", Value::Num(exit_code(o.status))),
            ("stdout", vstr(String::from_utf8_lossy(&o.stdout).into_owned())),
            ("stderr", vstr(String::from_utf8_lossy(&o.stderr).into_owned())),
        ]),
        Err(e) => err_val(io_err(&e)),
    }
}

fn path_str(p: Option<&std::ffi::OsStr>) -> String {
    p.map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Días desde 1970-01-01 → (año, mes, día). Algoritmo de Howard Hinnant.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    (y, m, d)
}

struct Fecha {
    anio: i64,
    mes: i64,
    dia: i64,
    hora: i64,
    minuto: i64,
    segundo: i64,
    dia_semana: i64,
}

fn fecha_utc(ts: f64) -> Fecha {
    let secs = ts.floor() as i64;
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (anio, mes, dia) = civil_from_days(days);
    Fecha {
        anio,
        mes,
        dia,
        hora: rem / 3600,
        minuto: rem % 3600 / 60,
        segundo: rem % 60,
        // 1970-01-01 fue jueves; 0 = domingo
        dia_semana: (days + 4).rem_euclid(7),
    }
}

fn fecha_local(ts: f64) -> Fecha {
    let t = ts.floor() as i64;
    let mut tm = std::mem::MaybeUninit::<sys::Tm>::zeroed();
    // SAFETY: localtime_r sólo escribe en `tm`, que es nuestro y tiene el layout de `struct tm`.
    let ok = unsafe {
        sys::tzset();
        !sys::localtime_r(&t, tm.as_mut_ptr()).is_null()
    };
    if !ok {
        return fecha_utc(ts);
    }
    // SAFETY: localtime_r devolvió éxito, así que inicializó la estructura.
    let tm = unsafe { tm.assume_init() };
    Fecha {
        anio: tm.tm_year as i64 + 1900,
        mes: tm.tm_mon as i64 + 1,
        dia: tm.tm_mday as i64,
        hora: tm.tm_hour as i64,
        minuto: tm.tm_min as i64,
        segundo: tm.tm_sec as i64,
        dia_semana: tm.tm_wday as i64,
    }
}

fn fecha_val<'p>(f: Fecha) -> Value<'p> {
    let n = |x: i64| Value::Num(x as f64);
    vobj(vec![
        ("anio", n(f.anio)),
        ("mes", n(f.mes)),
        ("dia", n(f.dia)),
        ("hora", n(f.hora)),
        ("minuto", n(f.minuto)),
        ("segundo", n(f.segundo)),
        ("dia_semana", n(f.dia_semana)),
    ])
}

/// Semilla inicial de `math::aleatorio`, tomada de /dev/urandom.
pub(crate) fn initial_seed() -> u64 {
    let mut buf = [0u8; 8];
    let from_urandom = std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut buf)).is_ok();
    let seed = if from_urandom {
        u64::from_le_bytes(buf)
    } else {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15)
    };
    seed.max(1)
}

fn proc_file(path: &str) -> String {
    std::fs::read_to_string(path).map(|s| s.trim().to_string()).unwrap_or_default()
}

// ---------- despacho ----------

impl<'p, 'o> Interpreter<'p, 'o> {
    fn arg_str(&self, args: &[Value<'p>], i: usize, span: Span) -> Result<String, Control<'p>> {
        match args.get(i) {
            Some(Value::Str(s)) => Ok(s.to_string()),
            _ => Err(rt("se esperaba un string", span)),
        }
    }

    fn arg_num(&self, args: &[Value<'p>], i: usize, span: Span) -> Result<f64, Control<'p>> {
        match args.get(i) {
            Some(v) => self.num(v, span),
            None => Err(rt("falta un argumento numérico", span)),
        }
    }

    fn arg_strs(&self, args: &[Value<'p>], i: usize) -> Vec<String> {
        match args.get(i) {
            Some(Value::Array(items)) => items
                .borrow()
                .iter()
                .map(|v| match v {
                    Value::Str(s) => s.to_string(),
                    v => display(v, true),
                })
                .collect(),
            _ => vec![],
        }
    }

    fn next_random(&mut self) -> f64 {
        // xorshift64*
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        let r = x.wrapping_mul(0x2545_F491_4F6C_DD1D);
        (r >> 11) as f64 / (1u64 << 53) as f64
    }

    pub(crate) fn call_builtin(&mut self, b: BuiltinFn, args: Vec<Value<'p>>, span: Span) -> EResult<'p> {
        use BuiltinFn::*;
        let num = |x: f64| Ok(Value::Num(x));
        match b {
            // ---------- prelude ----------
            Print => {
                let line: Vec<String> = args.iter().map(|a| display(a, true)).collect();
                (self.out)(&line.join(" "));
                Ok(Value::Unit)
            }
            Panic => {
                let msg = args.first().map(|a| display(a, true)).unwrap_or_default();
                Err(rt(format!("pánico: {}", msg), span))
            }
            Assert => {
                if matches!(args.first(), Some(Value::Bool(true))) {
                    Ok(Value::Unit)
                } else {
                    Err(rt("la aserción falló", span))
                }
            }

            // ---------- <fs> ----------
            FsLeer => {
                let ruta = self.arg_str(&args, 0, span)?;
                Ok(match std::fs::read_to_string(&ruta) {
                    Ok(contenido) => ok_val(vec![("contenido", vstr(contenido))]),
                    Err(e) => err_val(io_err(&e)),
                })
            }
            FsLeerLineas => {
                let ruta = self.arg_str(&args, 0, span)?;
                Ok(match std::fs::read_to_string(&ruta) {
                    Ok(contenido) => ok_val(vec![("lineas", varr(contenido.lines().map(vstr).collect()))]),
                    Err(e) => err_val(io_err(&e)),
                })
            }
            FsEscribir => {
                let ruta = self.arg_str(&args, 0, span)?;
                let texto = self.arg_str(&args, 1, span)?;
                Ok(unit_res(std::fs::write(&ruta, texto)))
            }
            FsAgregar => {
                let ruta = self.arg_str(&args, 0, span)?;
                let texto = self.arg_str(&args, 1, span)?;
                let r = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&ruta)
                    .and_then(|mut f| f.write_all(texto.as_bytes()));
                Ok(unit_res(r))
            }
            FsExiste => Ok(Value::Bool(Path::new(&self.arg_str(&args, 0, span)?).exists())),
            FsEsDir => Ok(Value::Bool(Path::new(&self.arg_str(&args, 0, span)?).is_dir())),
            FsEsArchivo => Ok(Value::Bool(Path::new(&self.arg_str(&args, 0, span)?).is_file())),
            FsInfo => {
                let ruta = self.arg_str(&args, 0, span)?;
                Ok(match std::fs::symlink_metadata(&ruta) {
                    Ok(m) => ok_val(vec![
                        ("tamano", Value::Num(m.len() as f64)),
                        ("es_dir", Value::Bool(m.is_dir())),
                        ("es_enlace", Value::Bool(m.file_type().is_symlink())),
                        ("modificado", Value::Num(m.mtime() as f64)),
                        ("permisos", Value::Num((m.mode() & 0o7777) as f64)),
                    ]),
                    Err(e) => err_val(io_err(&e)),
                })
            }
            FsListar => {
                let ruta = self.arg_str(&args, 0, span)?;
                Ok(match std::fs::read_dir(&ruta) {
                    Ok(entries) => {
                        let mut nombres: Vec<String> = entries
                            .filter_map(|e| e.ok())
                            .map(|e| e.file_name().to_string_lossy().into_owned())
                            .collect();
                        nombres.sort();
                        ok_val(vec![("entradas", varr(nombres.into_iter().map(vstr).collect()))])
                    }
                    Err(e) => err_val(io_err(&e)),
                })
            }
            FsCrearDir => Ok(unit_res(std::fs::create_dir_all(self.arg_str(&args, 0, span)?))),
            FsBorrar => {
                let ruta = self.arg_str(&args, 0, span)?;
                let path = Path::new(&ruta);
                // un enlace a un directorio se borra como enlace, sin tocar el directorio
                let is_real_dir = std::fs::symlink_metadata(path).is_ok_and(|m| m.is_dir());
                let r = if is_real_dir { std::fs::remove_dir_all(path) } else { std::fs::remove_file(path) };
                Ok(unit_res(r))
            }
            FsCopiar => {
                let desde = self.arg_str(&args, 0, span)?;
                let hasta = self.arg_str(&args, 1, span)?;
                Ok(unit_res(std::fs::copy(&desde, &hasta).map(|_| ())))
            }
            FsMover => {
                let desde = self.arg_str(&args, 0, span)?;
                let hasta = self.arg_str(&args, 1, span)?;
                Ok(unit_res(std::fs::rename(&desde, &hasta)))
            }
            FsPermisos => {
                let ruta = self.arg_str(&args, 0, span)?;
                let modo = self.arg_num(&args, 1, span)?;
                if !(0.0..=4095.0).contains(&modo) || modo.fract() != 0.0 {
                    return Ok(err_val(format!("modo inválido {}: usá un número como 0o644", modo)));
                }
                Ok(unit_res(std::fs::set_permissions(&ruta, std::fs::Permissions::from_mode(modo as u32))))
            }
            FsEnlazar => {
                let destino = self.arg_str(&args, 0, span)?;
                let enlace = self.arg_str(&args, 1, span)?;
                Ok(unit_res(std::os::unix::fs::symlink(&destino, &enlace)))
            }

            // ---------- <path> ----------
            PathUnir => {
                let a = self.arg_str(&args, 0, span)?;
                let b2 = self.arg_str(&args, 1, span)?;
                Ok(vstr(Path::new(&a).join(b2).to_string_lossy().into_owned()))
            }
            PathNombre => Ok(vstr(path_str(Path::new(&self.arg_str(&args, 0, span)?).file_name()))),
            PathBase => Ok(vstr(path_str(Path::new(&self.arg_str(&args, 0, span)?).file_stem()))),
            PathExtension => Ok(vstr(path_str(Path::new(&self.arg_str(&args, 0, span)?).extension()))),
            PathPadre => {
                let ruta = self.arg_str(&args, 0, span)?;
                let padre = Path::new(&ruta).parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
                Ok(vstr(padre))
            }
            PathAbsoluta => {
                let ruta = self.arg_str(&args, 0, span)?;
                Ok(match std::fs::canonicalize(&ruta) {
                    Ok(p) => ok_val(vec![("ruta", vstr(p.to_string_lossy().into_owned()))]),
                    Err(e) => err_val(io_err(&e)),
                })
            }
            PathEsAbsoluta => Ok(Value::Bool(Path::new(&self.arg_str(&args, 0, span)?).is_absolute())),

            // ---------- <env> ----------
            EnvObtener => {
                let nombre = self.arg_str(&args, 0, span)?;
                Ok(match std::env::var(&nombre) {
                    Ok(valor) => ok_val(vec![("valor", vstr(valor))]),
                    Err(_) => none_val(),
                })
            }
            EnvDefinir => {
                let nombre = self.arg_str(&args, 0, span)?;
                let valor = self.arg_str(&args, 1, span)?;
                if nombre.is_empty() || nombre.contains('=') || nombre.contains('\0') || valor.contains('\0') {
                    return Err(rt(format!("nombre de variable de entorno inválido: {:?}", nombre), span));
                }
                std::env::set_var(nombre, valor);
                Ok(Value::Unit)
            }
            EnvBorrar => {
                let nombre = self.arg_str(&args, 0, span)?;
                if !nombre.is_empty() && !nombre.contains('=') && !nombre.contains('\0') {
                    std::env::remove_var(nombre);
                }
                Ok(Value::Unit)
            }
            EnvTodas => {
                let mut vars: Vec<(String, String)> = std::env::vars_os()
                    .map(|(k, v)| (k.to_string_lossy().into_owned(), v.to_string_lossy().into_owned()))
                    .collect();
                vars.sort();
                Ok(varr(vars.into_iter().map(|(k, v)| vobj(vec![("nombre", vstr(k)), ("valor", vstr(v))])).collect()))
            }
            EnvActualDir => {
                Ok(vstr(std::env::current_dir().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default()))
            }
            EnvCambiarDir => Ok(unit_res(std::env::set_current_dir(self.arg_str(&args, 0, span)?))),
            EnvHome => Ok(vstr(std::env::var("HOME").unwrap_or_default())),
            EnvTempDir => Ok(vstr(std::env::temp_dir().to_string_lossy().into_owned())),

            // ---------- <proc> ----------
            ProcEjecutar => {
                let programa = self.arg_str(&args, 0, span)?;
                let cmd_args = self.arg_strs(&args, 1);
                Ok(salida(Command::new(&programa).args(&cmd_args).stdin(Stdio::null()).output()))
            }
            ProcEjecutarEntrada => {
                let programa = self.arg_str(&args, 0, span)?;
                let cmd_args = self.arg_strs(&args, 1);
                let entrada = self.arg_str(&args, 2, span)?;
                let r = Command::new(&programa)
                    .args(&cmd_args)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .and_then(|mut child| {
                        // escribimos en otro hilo para no trabarnos si el hijo llena su stdout
                        let stdin = child.stdin.take();
                        let writer = std::thread::spawn(move || {
                            if let Some(mut s) = stdin {
                                let _ = s.write_all(entrada.as_bytes());
                            }
                        });
                        let out = child.wait_with_output();
                        let _ = writer.join();
                        out
                    });
                Ok(salida(r))
            }
            ProcShell => {
                let comando = self.arg_str(&args, 0, span)?;
                Ok(salida(Command::new("/bin/sh").arg("-c").arg(&comando).stdin(Stdio::null()).output()))
            }
            ProcLanzar => {
                let programa = self.arg_str(&args, 0, span)?;
                let cmd_args = self.arg_strs(&args, 1);
                Ok(match Command::new(&programa).args(&cmd_args).spawn() {
                    Ok(child) => {
                        let pid = child.id();
                        self.children.insert(pid, child);
                        ok_val(vec![("pid", Value::Num(pid as f64))])
                    }
                    Err(e) => err_val(io_err(&e)),
                })
            }
            ProcEsperar => {
                let pid = self.arg_num(&args, 0, span)? as u32;
                let Some(mut child) = self.children.remove(&pid) else {
                    return Ok(err_val(format!("no hay ningún proceso lanzado con pid {}", pid)));
                };
                Ok(match child.wait() {
                    Ok(status) => ok_val(vec![("codigo", Value::Num(exit_code(status)))]),
                    Err(e) => err_val(io_err(&e)),
                })
            }
            ProcMatar => {
                let pid = self.arg_num(&args, 0, span)?;
                let senal = self.arg_num(&args, 1, span)?;
                if pid <= 0.0 || pid.fract() != 0.0 {
                    // kill(0) y kill(-1) afectan a grupos enteros de procesos: no lo permitimos
                    return Ok(err_val(format!("pid inválido {}", pid)));
                }
                // SAFETY: kill(2) no toca memoria nuestra.
                let r = unsafe { sys::kill(pid as i32, senal as i32) };
                Ok(if r == 0 { ok_val(vec![]) } else { err_val(io_err(&std::io::Error::last_os_error())) })
            }
            ProcSalir => {
                let code = match args.first() {
                    Some(Value::Num(n)) => *n as i32,
                    _ => 0,
                };
                let _ = std::io::stdout().flush();
                std::process::exit(code);
            }
            ProcArgs => Ok(varr(self.script_args.iter().map(|s| vstr(s.clone())).collect())),
            ProcPid => num(std::process::id() as f64),
            ProcPpid => num(std::os::unix::process::parent_id() as f64),

            // ---------- <io> ----------
            IoLeerLinea => {
                let mut linea = String::new();
                Ok(match std::io::stdin().read_line(&mut linea) {
                    Ok(0) | Err(_) => none_val(),
                    Ok(_) => {
                        let linea = linea.strip_suffix('\n').unwrap_or(&linea);
                        let linea = linea.strip_suffix('\r').unwrap_or(linea);
                        ok_val(vec![("linea", vstr(linea.to_string()))])
                    }
                })
            }
            IoLeerTodo => {
                let mut todo = String::new();
                let _ = std::io::stdin().read_to_string(&mut todo);
                Ok(vstr(todo))
            }
            IoEscribir => {
                let texto = self.arg_str(&args, 0, span)?;
                let mut stdout = std::io::stdout();
                let _ = stdout.write_all(texto.as_bytes());
                let _ = stdout.flush();
                Ok(Value::Unit)
            }
            IoError => {
                let texto = self.arg_str(&args, 0, span)?;
                let mut stderr = std::io::stderr();
                let _ = stderr.write_all(texto.as_bytes());
                let _ = stderr.flush();
                Ok(Value::Unit)
            }
            IoEsTerminal => Ok(Value::Bool(std::io::stdout().is_terminal())),

            // ---------- <time> ----------
            TimeAhora | TimeAhoraMs => {
                let d = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
                num(if b == TimeAhora { d.as_secs_f64() } else { d.as_millis() as f64 })
            }
            TimeReloj => num(self.start.elapsed().as_secs_f64()),
            TimeDormir => {
                let secs = self.arg_num(&args, 0, span)?;
                if secs.is_finite() && secs > 0.0 {
                    std::thread::sleep(std::time::Duration::from_secs_f64(secs));
                }
                Ok(Value::Unit)
            }
            TimeFecha => Ok(fecha_val(fecha_utc(self.arg_num(&args, 0, span)?))),
            TimeFechaLocal => Ok(fecha_val(fecha_local(self.arg_num(&args, 0, span)?))),
            TimeIso => {
                let f = fecha_utc(self.arg_num(&args, 0, span)?);
                Ok(vstr(format!(
                    "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
                    f.anio, f.mes, f.dia, f.hora, f.minuto, f.segundo
                )))
            }

            // ---------- <os> ----------
            OsNombre => Ok(vstr(std::env::consts::OS)),
            OsArquitectura => Ok(vstr(std::env::consts::ARCH)),
            OsUsuario => Ok(vstr(std::env::var("USER").or_else(|_| std::env::var("LOGNAME")).unwrap_or_default())),
            OsHostname => {
                let h = proc_file("/proc/sys/kernel/hostname");
                Ok(vstr(if h.is_empty() { proc_file("/etc/hostname") } else { h }))
            }
            OsKernel => Ok(vstr(proc_file("/proc/sys/kernel/osrelease"))),
            // SAFETY: getuid(2) no puede fallar ni toca memoria.
            OsUid => num(unsafe { sys::getuid() } as f64),
            OsCpus => num(std::thread::available_parallelism().map_or(1, |n| n.get()) as f64),
            OsMemoria => {
                let meminfo = proc_file("/proc/meminfo");
                let campo = |nombre: &str| -> f64 {
                    meminfo
                        .lines()
                        .find(|l| l.starts_with(nombre))
                        .and_then(|l| l.split_whitespace().nth(1))
                        .and_then(|kb| kb.parse::<f64>().ok())
                        .map_or(0.0, |kb| kb * 1024.0)
                };
                Ok(vobj(vec![
                    ("total", Value::Num(campo("MemTotal:"))),
                    ("disponible", Value::Num(campo("MemAvailable:"))),
                ]))
            }
            OsUptime => {
                let up = proc_file("/proc/uptime");
                num(up.split_whitespace().next().and_then(|s| s.parse().ok()).unwrap_or(0.0))
            }
            OsCarga => {
                let load = proc_file("/proc/loadavg");
                let vals: Vec<Value<'p>> =
                    load.split_whitespace().take(3).map(|s| Value::Num(s.parse().unwrap_or(0.0))).collect();
                Ok(varr(vals))
            }

            // ---------- <math> ----------
            MathSin => num(self.arg_num(&args, 0, span)?.sin()),
            MathCos => num(self.arg_num(&args, 0, span)?.cos()),
            MathTan => num(self.arg_num(&args, 0, span)?.tan()),
            MathAsin => num(self.arg_num(&args, 0, span)?.asin()),
            MathAcos => num(self.arg_num(&args, 0, span)?.acos()),
            MathAtan => num(self.arg_num(&args, 0, span)?.atan()),
            MathAtan2 => {
                let y = self.arg_num(&args, 0, span)?;
                let x = self.arg_num(&args, 1, span)?;
                num(y.atan2(x))
            }
            MathExp => num(self.arg_num(&args, 0, span)?.exp()),
            MathLn => num(self.arg_num(&args, 0, span)?.ln()),
            MathLog10 => num(self.arg_num(&args, 0, span)?.log10()),
            MathAleatorio => num(self.next_random()),
            MathAleatorioEntre => {
                let a = self.arg_num(&args, 0, span)?.ceil();
                let z = self.arg_num(&args, 1, span)?.floor();
                if z < a {
                    return Err(rt(format!("math::aleatorio_entre: el rango {}..={} está vacío", a, z), span));
                }
                num(a + (self.next_random() * (z - a + 1.0)).floor())
            }
            MathSemilla => {
                let s = self.arg_num(&args, 0, span)?;
                self.rng = (s as i64 as u64) ^ 0x9E37_79B9_7F4A_7C15;
                if self.rng == 0 {
                    self.rng = 1;
                }
                Ok(Value::Unit)
            }
        }
    }
}
