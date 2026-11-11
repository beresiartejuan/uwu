//! La biblioteca estándar: cómo se importa y qué hace cada función.

use super::*;

// ======================= biblioteca estándar: importación =======================

#[test]
fn stdlib_requiere_import() {
    let r = uwu::run("print(fs::existe(\"/\"));", &mut |_| {}, true, &[]);
    let d = &r.diagnostics[0];
    assert_eq!(d.message, "la biblioteca `fs` no está importada");
    assert_eq!(d.hint.as_deref(), Some("agregá `import <fs>;` al principio del archivo"));
}

#[test]
fn stdlib_sintaxis_vieja_da_pista() {
    let r = uwu::run("print(fs.existe(\"/\"));", &mut |_| {}, true, &[]);
    assert!(r.diagnostics[0].hint.as_deref().unwrap().contains("import <fs>;"));
    let r = uwu::run("import <fs>; print(fs.existe(\"/\"));", &mut |_| {}, true, &[]);
    assert_eq!(r.diagnostics[0].message, "`fs` es una biblioteca, no un valor");
}

#[test]
fn stdlib_inexistente() {
    let r = uwu::run("import <tiempo>;", &mut |_| {}, true, &[]);
    assert_eq!(r.diagnostics[0].message, "no existe la biblioteca estándar `<tiempo>`");
    assert_eq!(r.diagnostics[0].hint.as_deref(), Some("¿quisiste decir `import <time>;`?"));
    let r = uwu::run("print(tiempo::ahora());", &mut |_| {}, true, &[]);
    assert!(r.diagnostics[0].hint.as_deref().unwrap().contains("`time::`"));
}

#[test]
fn stdlib_miembro_inexistente_sugiere() {
    let r = uwu::run("import <fs>; fs::leeer(\"x\");", &mut |_| {}, true, &[]);
    assert_eq!(r.diagnostics[0].hint.as_deref(), Some("¿quisiste decir `fs::leer`?"));
}

#[test]
fn using_namespace_y_using_declaracion() {
    let out = exec(
        r#"
        import <math>;
        import <path>;
        using namespace math;
        using path::extension;
        print(PI > 3, extension("foto.png"));"#,
    );
    assert_eq!(out, vec!["true png"]);
}

#[test]
fn stdlib_funciones_como_valores() {
    assert_eq!(
        exec("import <path>; let f = path::nombre; print(f(\"/a/b.txt\"), f);"),
        vec!["b.txt <fn path::nombre>"]
    );
}

// ======================= biblioteca estándar: funciones =======================

#[test]
fn stdlib_fs_escribir_leer_borrar() {
    let t = TempDir::new("fs_basico");
    let code = format!(
        r#"
        import <fs>;
        import <path>;
        let dir = path::unir("{dir}", "sub");
        let ruta = path::unir(dir, "nota.txt");
        fs::crear_dir(dir);
        match fs::escribir(ruta, "hola\n") {{
            {{ ok: true }} => print("escrito"),
            {{ ok: false, error }} => print("error:", error),
        }}
        fs::agregar(ruta, "chau\n");
        match fs::leer_lineas(ruta) {{
            {{ ok: true, lineas }} => print(lineas),
            {{ ok: false, error }} => print("error:", error),
        }}
        print(fs::existe(ruta), fs::es_archivo(ruta), fs::es_dir(dir));
        fs::borrar(dir);
        print(fs::existe(ruta));
        "#,
        dir = t.path(),
    );
    assert_eq!(exec(&code), vec!["escrito", r#"["hola", "chau"]"#, "true true true", "false"]);
}

#[test]
fn stdlib_fs_info_permisos_y_enlaces() {
    let t = TempDir::new("fs_info");
    let code = format!(
        r#"
        import <fs>;
        let ruta = "{dir}/x.sh";
        let enlace = "{dir}/enlace";
        fs::escribir(ruta, "12345");
        fs::permisos(ruta, 0o750);
        match fs::info(ruta) {{
            {{ ok: true, tamano, permisos, es_enlace }} => print(tamano, permisos == 0o750, es_enlace),
            {{ ok: false, error }} => print(error),
        }}
        fs::enlazar(ruta, enlace);
        match fs::info(enlace) {{
            {{ ok: true, es_enlace }} => print("enlace:", es_enlace),
            {{ ok: false, error }} => print(error),
        }}
        match fs::listar("{dir}") {{
            {{ ok: true, entradas }} => print(entradas),
            {{ ok: false, error }} => print(error),
        }}
        "#,
        dir = t.path(),
    );
    assert_eq!(exec(&code), vec!["5 true false", "enlace: true", r#"["enlace", "x.sh"]"#]);
}

#[test]
fn stdlib_fs_errores_en_castellano() {
    let out = exec(
        r#"
        import <fs>;
        match fs::leer("/ruta/que/no/existe/para/uwu") {
            { ok: true, contenido } => print(contenido),
            { ok: false, error } => print(error),
        }"#,
    );
    assert_eq!(out, vec!["no existe el archivo o directorio (errno 2)"]);
}

#[test]
fn stdlib_path() {
    let out = exec(
        r#"
        import <path>;
        let r = "/home/juan/fotos/playa.jpg";
        print(path::nombre(r), path::base(r), path::extension(r), path::padre(r));
        print(path::unir("/tmp", "x.txt"), path::es_absoluta(r), path::es_absoluta("rel/a"), path::SEPARADOR);
        match path::absoluta("/") {
            { ok: true, ruta } => print(ruta),
            { ok: false, error } => print(error),
        }"#,
    );
    assert_eq!(out, vec!["playa.jpg playa jpg /home/juan/fotos", "/tmp/x.txt true false /", "/"]);
}

#[test]
fn stdlib_env() {
    let out = exec(
        r#"
        import <env>;
        env::definir("UWU_TEST_VAR", "42");
        match env::obtener("UWU_TEST_VAR") {
            { ok: true, valor } => print("valor:", valor),
            { ok: false } => print("no está"),
        }
        print(env::todas().any(|v| v.nombre == "UWU_TEST_VAR"));
        env::borrar("UWU_TEST_VAR");
        match env::obtener("UWU_TEST_VAR") {
            { ok: true, valor } => print(valor),
            { ok: false } => print("no está"),
        }
        print(env::temp_dir().len() > 0);"#,
    );
    assert_eq!(out, vec!["valor: 42", "true", "no está", "true"]);
}

#[test]
fn stdlib_proc() {
    let out = exec(
        r#"
        import <proc>;
        match proc::ejecutar("echo", ["hola", "uwu"]) {
            { ok: true, codigo, stdout } => print(stdout.trim(), codigo),
            { ok: false, error } => print("error:", error),
        }
        match proc::ejecutar("/programa/inexistente/uwu", []) {
            { ok: true, codigo } => print(codigo),
            { ok: false, error } => print("no se pudo ejecutar"),
        }
        match proc::shell("echo $((6 * 7)); exit 3") {
            { ok: true, codigo, stdout } => print(stdout.trim(), codigo),
            { ok: false, error } => print(error),
        }
        match proc::ejecutar_entrada("tr", ["a-z", "A-Z"], "gritar") {
            { ok: true, stdout } => print(stdout),
            { ok: false, error } => print(error),
        }
        print(proc::pid() > 0, proc::ppid() > 0);"#,
    );
    assert_eq!(out, vec!["hola uwu 0", "no se pudo ejecutar", "42 3", "GRITAR", "true true"]);
}

#[test]
fn stdlib_proc_lanzar_matar_esperar() {
    let out = exec(
        r#"
        import <proc>;
        match proc::lanzar("sleep", ["30"]) {
            { ok: true, pid } => {
                proc::matar(pid, proc::SIGTERM);
                match proc::esperar(pid) {
                    { ok: true, codigo } => print("código:", codigo),
                    { ok: false, error } => print(error),
                }
            }
            { ok: false, error } => print(error),
        }
        match proc::esperar(1) {
            { ok: true, codigo } => print(codigo),
            { ok: false, error } => print(error),
        }"#,
    );
    assert_eq!(out, vec!["código: 143", "no hay ningún proceso lanzado con pid 1"]);
}

#[test]
fn stdlib_time() {
    let out = exec(
        r#"
        import <time>;
        let f = time::fecha(1700000000);
        print(f.anio, f.mes, f.dia, f.hora, f.minuto, f.segundo, f.dia_semana);
        print(time::iso(0), time::iso(951782400));
        let antes = time::reloj();
        time::dormir(0.01);
        print(time::reloj() - antes >= 0.01, time::ahora() > 1700000000, time::ahora_ms() > 1700000000000);
        let l = time::fecha_local(time::ahora());
        print(l.anio >= 2024);"#,
    );
    assert_eq!(
        out,
        vec!["2023 11 14 22 13 20 2", "1970-01-01T00:00:00Z 2000-02-29T00:00:00Z", "true true true", "true"]
    );
}

#[test]
fn stdlib_os() {
    let out = exec(
        r#"
        import <os>;
        print(os::nombre(), os::arquitectura().len() > 0, os::kernel().len() > 0);
        print(os::cpus() >= 1, os::uid() >= 0, os::uptime() > 0, os::carga().len());
        let m = os::memoria();
        print(m.total > 0, m.disponible <= m.total);"#,
    );
    assert_eq!(out, vec!["linux true true", "true true true 3", "true true"]);
}

#[test]
fn stdlib_math() {
    let out = exec(
        r#"
        import <math>;
        print(math::sin(0), math::cos(0), math::exp(0), math::ln(math::E), math::log10(1000));
        print((math::atan2(1, 1) * 4 - math::PI).abs() < 0.0000001);
        math::semilla(7);
        let a = [math::aleatorio(), math::aleatorio()];
        math::semilla(7);
        let b = [math::aleatorio(), math::aleatorio()];
        print(a == b, a[0] >= 0 && a[0] < 1);
        let mut ok = true;
        for i in 0..200 {
            let d = math::aleatorio_entre(1, 6);
            if d < 1 || d > 6 || d != d.floor() { ok = false; }
        }
        print(ok);"#,
    );
    assert_eq!(out, vec!["0 1 1 1 3", "true", "true true", "true"]);
}
