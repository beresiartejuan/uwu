//! La biblioteca estándar: cómo se importa y qué hace cada función.

use super::*;

// ======================= biblioteca estándar: importación =======================

#[test]
fn stdlib_requiere_import() {
    let r = uwu::run("print(fs::exists(\"/\"));", &mut |_| {}, true, &[]);
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
    let r = uwu::run("import <fs>; fs::raed(\"x\");", &mut |_| {}, true, &[]);
    assert_eq!(r.diagnostics[0].hint.as_deref(), Some("¿quisiste decir `fs::read`?"));
}

#[test]
fn stdlib_nombres_viejos_en_castellano_dan_pista() {
    let r = uwu::run("import <proc>; proc::ejecutar(\"ls\", []);", &mut |_| {}, true, &[]);
    assert_eq!(r.diagnostics[0].message, "la biblioteca `proc` no tiene `ejecutar`");
    assert_eq!(r.diagnostics[0].hint.as_deref(), Some("ahora se llama `proc::run`"));
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
        exec("import <path>; let f = path::file_name; print(f(\"/a/b.txt\"), f);"),
        vec!["b.txt <fn path::file_name>"]
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
        let dir = path::join("{dir}", "sub");
        let ruta = path::join(dir, "nota.txt");
        fs::create_dir(dir);
        match fs::write(ruta, "hola\n") {{
            {{ ok: true }} => print("escrito"),
            {{ ok: false, error }} => print("error:", error),
        }}
        fs::append(ruta, "chau\n");
        match fs::read_lines(ruta) {{
            {{ ok: true, lines }} => print(lines),
            {{ ok: false, error }} => print("error:", error),
        }}
        print(fs::exists(ruta), fs::is_file(ruta), fs::is_dir(dir));
        fs::remove(dir);
        print(fs::exists(ruta));
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
        fs::write(ruta, "12345");
        fs::set_permissions(ruta, 0o750);
        match fs::stat(ruta) {{
            {{ ok: true, size, permissions, is_symlink }} => print(size, permissions == 0o750, is_symlink),
            {{ ok: false, error }} => print(error),
        }}
        fs::symlink(ruta, enlace);
        match fs::stat(enlace) {{
            {{ ok: true, is_symlink }} => print("enlace:", is_symlink),
            {{ ok: false, error }} => print(error),
        }}
        match fs::list_dir("{dir}") {{
            {{ ok: true, entries }} => print(entries),
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
        match fs::read("/ruta/que/no/existe/para/uwu") {
            { ok: true, content } => print(content),
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
        print(path::file_name(r), path::stem(r), path::extension(r), path::parent(r));
        print(path::join("/tmp", "x.txt"), path::is_absolute(r), path::is_absolute("rel/a"), path::SEPARATOR);
        match path::absolute("/") {
            { ok: true, path: abs } => print(abs),
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
        env::set("UWU_TEST_VAR", "42");
        match env::get("UWU_TEST_VAR") {
            { ok: true, value } => print("valor:", value),
            { ok: false } => print("no está"),
        }
        print(env::vars().any(|v| v.name == "UWU_TEST_VAR"));
        env::remove("UWU_TEST_VAR");
        match env::get("UWU_TEST_VAR") {
            { ok: true, value } => print(value),
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
        match proc::run("echo", ["hola", "uwu"]) {
            { ok: true, code, stdout } => print(stdout.trim(), code),
            { ok: false, error } => print("error:", error),
        }
        match proc::run("/programa/inexistente/uwu", []) {
            { ok: true, code } => print(code),
            { ok: false, error } => print("no se pudo ejecutar"),
        }
        match proc::shell("echo $((6 * 7)); exit 3") {
            { ok: true, code, stdout } => print(stdout.trim(), code),
            { ok: false, error } => print(error),
        }
        match proc::run_with_input("tr", ["a-z", "A-Z"], "gritar") {
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
        match proc::spawn("sleep", ["30"]) {
            { ok: true, pid } => {
                proc::kill(pid, proc::SIGTERM);
                match proc::wait(pid) {
                    { ok: true, code } => print("código:", code),
                    { ok: false, error } => print(error),
                }
            }
            { ok: false, error } => print(error),
        }
        match proc::wait(1) {
            { ok: true, code } => print(code),
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
        let f = time::date(1700000000);
        print(f.year, f.month, f.day, f.hour, f.minute, f.second, f.weekday);
        print(time::iso(0), time::iso(951782400));
        let antes = time::elapsed();
        time::sleep(0.01);
        print(time::elapsed() - antes >= 0.01, time::now() > 1700000000, time::now_ms() > 1700000000000);
        let l = time::local_date(time::now());
        print(l.year >= 2024);"#,
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
        print(os::name(), os::arch().len() > 0, os::kernel().len() > 0);
        print(os::cpus() >= 1, os::uid() >= 0, os::uptime() > 0, os::load_avg().len());
        let m = os::memory();
        print(m.total > 0, m.available <= m.total);"#,
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
        math::seed(7);
        let a = [math::random(), math::random()];
        math::seed(7);
        let b = [math::random(), math::random()];
        print(a == b, a[0] >= 0 && a[0] < 1);
        let mut ok = true;
        for i in 0..200 {
            let d = math::random_range(1, 6);
            if d < 1 || d > 6 || d != d.floor() { ok = false; }
        }
        print(ok);"#,
    );
    assert_eq!(out, vec!["0 1 1 1 3", "true", "true true", "true"]);
}
