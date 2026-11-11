use uwu::diagnostics::Severity;
use uwu::run;

/// Ejecuta código uwu y devuelve lo que imprimió. Falla si hay errores de compilación.
fn exec(code: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let r = run(code, &mut |l| out.push(l.to_string()), false, &[]);
    let errors: Vec<String> =
        r.diagnostics.iter().filter(|d| d.severity != Severity::Warning).map(|d| d.message.clone()).collect();
    assert!(errors.is_empty(), "errores inesperados:\n{}", errors.join("\n"));
    assert!(r.runtime_error.is_none(), "pánico: {}", r.runtime_error.unwrap().message);
    out
}

/// Devuelve los mensajes de error de compilación.
fn errors(code: &str) -> Vec<String> {
    let r = run(code, &mut |_| {}, true, &[]);
    r.diagnostics.iter().filter(|d| d.severity != Severity::Warning).map(|d| d.message.clone()).collect()
}

fn panic_msg(code: &str) -> String {
    let r = run(code, &mut |_| {}, false, &[]);
    r.runtime_error.map(|d| d.message).unwrap_or_default()
}

// ======================= expresiones y variables =======================

#[test]
fn aritmetica_y_precedencia() {
    assert_eq!(exec("print(1 + 2 * 3, (1 + 2) * 3, 10 % 3, -2 * 3);"), vec!["7 9 1 -6"]);
}

#[test]
fn concatenacion_de_strings_con_numeros() {
    assert_eq!(exec(r#"print("x = " + 5 + "!");"#), vec!["x = 5!"]);
}

#[test]
fn bloques_e_if_como_expresiones() {
    assert_eq!(
        exec("let a = { let b = 2; b * 10 }; let c = if a > 5 { \"grande\" } else { \"chico\" }; print(a, c);"),
        vec!["20 grande"]
    );
}

#[test]
fn mutabilidad() {
    assert_eq!(exec("let mut x = 1; x += 4; x *= 2; print(x);"), vec!["10"]);
    assert_eq!(errors("let x = 1; x = 2;"), vec!["no se puede modificar `x` porque es inmutable"]);
    assert_eq!(errors("let v = [1]; v.push(2);"), vec!["no se puede modificar `v` porque es inmutable"]);
    assert_eq!(errors("let o = { a: 1 }; o.a = 2;"), vec!["no se puede modificar `o` porque es inmutable"]);
}

#[test]
fn shadowing_como_en_rust() {
    assert_eq!(exec(r#"let x = 5; let x = x.to_string() + "!"; print(x);"#), vec!["5!"]);
}

#[test]
fn igualdad_estructural() {
    assert_eq!(exec("print([1, 2] == [1, 2], { a: 1 } == { a: 2 });"), vec!["true false"]);
}

// ======================= control de flujo =======================

#[test]
fn while_loop_break_y_continue() {
    let code = "
      let mut i = 0;
      let mut pares: number[] = [];
      loop {
        i += 1;
        if i > 10 { break; }
        if i % 2 == 1 { continue; }
        pares.push(i);
      }
      print(pares);";
    assert_eq!(exec(code), vec!["[2, 4, 6, 8, 10]"]);
}

#[test]
fn for_sobre_rangos_arreglos_y_strings() {
    assert_eq!(
        exec(r#"let mut s = ""; for c in "abc" { s = c + s; } for i in 0..3 { s += i; } print(s);"#),
        vec!["cba012"]
    );
}

#[test]
fn return_anticipado() {
    assert_eq!(
        exec("fn f(x: number): string { if x < 0 { return \"neg\"; } \"pos\" } print(f(-1), f(1));"),
        vec!["neg pos"]
    );
}

#[test]
fn break_fuera_de_un_bucle() {
    assert_eq!(errors("break;"), vec!["`break` fuera de un bucle"]);
}

// ======================= funciones y closures =======================

#[test]
fn recursion_mutua() {
    let code = "
      fn es_par(n: number): boolean { if n == 0 { true } else { es_impar(n - 1) } }
      fn es_impar(n: number): boolean { if n == 0 { false } else { es_par(n - 1) } }
      print(es_par(10), es_impar(7));";
    assert_eq!(exec(code), vec!["true true"]);
}

#[test]
fn closures_capturan_su_entorno() {
    let code = "
      fn sumador(n: number): (x: number) => number { |x| x + n }
      let mas5 = sumador(5);
      print(mas5(10));";
    assert_eq!(exec(code), vec!["15"]);
}

#[test]
fn main_se_ejecuta_automaticamente() {
    assert_eq!(exec(r#"fn main() { print("desde main"); }"#), vec!["desde main"]);
}

#[test]
fn cantidad_de_argumentos() {
    assert_eq!(errors("fn f(a: number): number { a } f(1, 2);"), vec!["`f` espera 1 argumento(s), pero recibió 2"]);
}

#[test]
fn falta_valor_de_retorno() {
    assert!(errors("fn f(): number { 1; }")[0].contains("debe devolver `number`"));
}

#[test]
fn closure_sin_tipo_inferible() {
    assert!(errors("let f = |x| x;")[0].contains("no se puede inferir el tipo del parámetro `x`"));
}

// ======================= sistema de tipos =======================

#[test]
fn tipos_literales_y_uniones() {
    assert!(errors(r#"let d: "norte" | "sur" = "este";"#)[0].contains(r#"se esperaba `"norte" | "sur"`"#));
    assert_eq!(exec(r#"let d: "norte" | "sur" = "sur"; print(d);"#), vec!["sur"]);
}

#[test]
fn objetos_estructurales_como_ts() {
    let code = "
      type Punto = { x: number, y: number };
      fn norma(p: Punto): number { (p.x * p.x + p.y * p.y).sqrt() }
      let p3 = { x: 3, y: 4, z: 99 };
      print(norma(p3));";
    assert_eq!(exec(code), vec!["5"]);
}

#[test]
fn campo_faltante() {
    assert!(errors("type P = { x: number }; let p: P = { y: 1 };")[0].contains("tipos incompatibles"));
}

#[test]
fn no_hay_conversiones_implicitas_a_boolean() {
    assert!(errors("if 1 { }")[0].contains("se esperaba `boolean`"));
}

#[test]
fn comparacion_imposible() {
    assert!(errors(r#"let s: "a" | "b" = "a"; print(s == "c");"#)[0].contains("siempre da false"));
}

#[test]
fn arreglo_vacio_necesita_anotacion() {
    assert!(errors("let mut v = [];")[0].contains("no se puede inferir el tipo de `v`"));
}

#[test]
fn tipo_desconocido_con_sugerencia() {
    let (_, diags) = uwu::compile("let x: int = 1;");
    assert_eq!(diags[0].hint.as_deref(), Some("en uwu se escribe `number`"));
}

// ======================= genéricos =======================

#[test]
fn inferencia_de_parametros_de_tipo() {
    let code = r#"
      fn primero<T>(xs: T[]): T { xs[0] }
      let s: string = primero(["a", "b"]);
      let n: number = primero([1, 2]);
      print(s, n);"#;
    assert_eq!(exec(code), vec!["a 1"]);
}

#[test]
fn el_tipo_inferido_se_respeta() {
    assert!(errors("fn id<T>(x: T): T { x } let s: string = id(5);")[0]
        .contains("se esperaba `string`, se encontró `number`"));
}

#[test]
fn turbofish() {
    assert_eq!(exec("fn vacio<T>(): T[] { [] } let v = vacio::<number>(); print(v.len());"), vec!["0"]);
}

#[test]
fn genericos_con_closures() {
    let code = r#"
      fn aplicar<A, B>(x: A, f: (a: A) => B): B { f(x) }
      let largo: number = aplicar("hola", |s| s.len());
      print(largo);"#;
    assert_eq!(exec(code), vec!["4"]);
}

#[test]
fn map_filter_fold() {
    assert_eq!(
        exec(
            r#"print([1, 2, 3].map(|x| x.to_string() + "!").join(" "), [1,2,3,4].filter(|x| x > 2), [1,2,3].fold("", |a, x| a + x));"#
        ),
        vec!["1! 2! 3! [3, 4] 123"]
    );
}

// ======================= match =======================

#[test]
fn literales_y_comodin() {
    let code = r#"
      fn nombre(n: number): string { match n { 1 => "uno", 2 => "dos", _ => "muchos" } }
      print(nombre(1), nombre(2), nombre(7));"#;
    assert_eq!(exec(code), vec!["uno dos muchos"]);
}

#[test]
fn narrowing_por_tipo_en_uniones() {
    let code = r#"
      fn describir(x: number | string | boolean): string {
        match x {
          n: number => "número " + (n * 2),
          s: string => "texto de " + s.len(),
          b: boolean => if b { "sí" } else { "no" },
        }
      }
      print(describir(21), describir("hola"), describir(false));"#;
    assert_eq!(exec(code), vec!["número 42 texto de 4 no"]);
}

#[test]
fn exhaustividad_con_booleanos_anidados() {
    let code = r#"
      type R = { ok: true, v: number } | { ok: false, e: string };
      fn f(r: R): string { match r { { ok: true, v } => "ok " + v, { ok: false, e } => e } }
      print(f({ ok: true, v: 1 }), f({ ok: false, e: "mal" }));"#;
    assert_eq!(exec(code), vec!["ok 1 mal"]);
}

#[test]
fn exhaustividad_partiendo_uniones_de_literales() {
    let errs = errors(
        "
      type C = { a: 1 | 2, b: boolean };
      fn f(c: C): number { match c { { a: 1 } => 1, { a: 2, b: true } => 2 } }",
    );
    assert_eq!(errs[0], "`match` no exhaustivo: falta cubrir `{ a: 2, b: false }`");
}

#[test]
fn los_guardas_no_cuentan_para_la_exhaustividad() {
    assert!(errors("fn f(b: boolean): number { match b { true if 1 > 0 => 1, false => 0 } }")[0]
        .contains("falta cubrir `true`"));
}

#[test]
fn numeros_necesitan_comodin() {
    assert!(errors("fn f(n: number): number { match n { 0 => 1 } }")[0].contains("falta cubrir `number`"));
}

#[test]
fn patron_imposible() {
    assert!(errors(r#"let s: "a" | "b" = "a"; match s { "z" => 1, _ => 2 };"#)[0].contains("nunca coincide"));
}

#[test]
fn acceder_a_campo_de_variante_requiere_match() {
    assert!(errors(r#"type F = { t: "a", x: number } | { t: "b" }; let f: F = { t: "b" }; print(f.x);"#)[0]
        .contains("no todas las variantes"));
}

// ======================= errores en ejecución =======================

#[test]
fn indice_fuera_de_rango() {
    assert!(panic_msg("let v = [1]; print(v[3]);").contains("índice fuera de rango"));
}

#[test]
fn division_por_cero() {
    assert_eq!(panic_msg("let a = 0; print(1 / a);"), "división por cero");
}

#[test]
fn panic_y_assert() {
    assert_eq!(panic_msg(r#"panic("algo salió mal");"#), "pánico: algo salió mal");
    assert_eq!(panic_msg("let a = 1; assert(a == 2);"), "la aserción falló");
}

#[test]
fn recursion_infinita() {
    assert!(panic_msg("fn f(n: number): number { f(n + 1) } f(0);").contains("desbordamiento de pila"));
}

// ======================= errores de sintaxis =======================

#[test]
fn falta_punto_y_coma() {
    assert!(errors("let a = 1\nlet b = 2;")[0].contains("se esperaba `;`"));
}

#[test]
fn string_sin_cerrar() {
    assert_eq!(errors(r#"let s = "hola;"#)[0], "string sin cerrar");
}

#[test]
fn parametro_sin_tipo() {
    assert_eq!(errors("fn f(x) { }")[0], "falta el tipo del parámetro `x`");
}

// ======================= helpers para archivos =======================

/// Carpeta temporal propia de cada test, que se borra al terminar.
struct TempDir(std::path::PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("uwu_{}_{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }

    fn write(&self, rel: &str, code: &str) -> std::path::PathBuf {
        let p = self.0.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, code).unwrap();
        p
    }

    fn path(&self) -> String {
        self.0.display().to_string()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Ejecuta un archivo; devuelve el resultado y lo que imprimió.
fn run_path(path: &std::path::Path) -> (uwu::RunResult, Vec<String>) {
    let mut out: Vec<String> = Vec::new();
    let r = uwu::run_file(path, &mut |l| out.push(l.to_string()), false, &[]).unwrap();
    (r, out)
}

/// Ejecuta un archivo y devuelve lo que imprimió. Falla si hay errores.
fn exec_file(path: &std::path::Path) -> Vec<String> {
    let (r, out) = run_path(path);
    let errors: Vec<String> = r
        .diagnostics
        .iter()
        .filter(|d| d.severity != Severity::Warning)
        .map(|d| format!("{} ({})", d.message, r.files[d.span.file].name))
        .collect();
    assert!(errors.is_empty(), "errores inesperados:\n{}", errors.join("\n"));
    assert!(r.runtime_error.is_none(), "pánico: {}", r.runtime_error.unwrap().message);
    out
}

/// Errores de compilación de un archivo, con el nombre del archivo donde ocurrieron.
fn file_errors(path: &std::path::Path) -> Vec<(String, String)> {
    let (r, _) = run_path(path);
    r.diagnostics
        .iter()
        .filter(|d| d.severity != Severity::Warning)
        .map(|d| {
            let file =
                std::path::Path::new(&r.files[d.span.file].name).file_name().unwrap().to_string_lossy().into_owned();
            (d.message.clone(), file)
        })
        .collect()
}

// ======================= módulos =======================

#[test]
fn import_de_archivo_con_export() {
    let t = TempDir::new("import_archivo");
    t.write(
        "geometria.uwu",
        r#"
        export type Punto = { x: number, y: number };
        export let ORIGEN: Punto = { x: 0, y: 0 };
        fn cuadrado(n: number): number { n * n }   // privada
        export fn distancia(a: Punto, b: Punto): number {
            (cuadrado(a.x - b.x) + cuadrado(a.y - b.y)).sqrt()
        }"#,
    );
    let main = t.write(
        "main.uwu",
        r#"
        import "geometria.uwu";
        let p: Punto = { x: 3, y: 4 };
        print(distancia(ORIGEN, p));"#,
    );
    assert_eq!(exec_file(&main), vec!["5"]);
}

#[test]
fn lo_no_exportado_es_privado() {
    let t = TempDir::new("privado");
    t.write("util.uwu", "fn secreto(): number { 42 } export fn publico(): number { secreto() }");
    let main = t.write("main.uwu", r#"import "util"; print(publico()); print(secreto());"#);
    let errs = file_errors(&main);
    assert_eq!(errs.len(), 1);
    assert_eq!(errs[0].0, "no existe `secreto` en este ámbito");
}

#[test]
fn import_por_nombre_de_modulo() {
    let t = TempDir::new("por_nombre");
    t.write("mates.uwu", "export module mates;\nexport fn doble(n: number): number { n * 2 }");
    let main = t.write("main.uwu", "import mates; print(doble(21));");
    assert_eq!(exec_file(&main), vec!["42"]);
}

#[test]
fn nombre_de_modulo_tiene_que_coincidir() {
    let t = TempDir::new("nombre_mal");
    t.write("mates.uwu", "export module matematica;\nexport fn uno(): number { 1 }");
    let main = t.write("main.uwu", "import mates; print(uno());");
    assert!(file_errors(&main)[0].0.contains("declara `export module matematica;`"));
}

#[test]
fn import_relativo_desde_subcarpetas() {
    let t = TempDir::new("subcarpetas");
    t.write("lib/a.uwu", r#"import "b.uwu"; export fn a(): string { "a" + b() }"#);
    t.write("lib/b.uwu", r#"export fn b(): string { "b" }"#);
    let main = t.write("main.uwu", r#"import "lib/a.uwu"; print(a());"#);
    assert_eq!(exec_file(&main), vec!["ab"]);
}

#[test]
fn reexport_con_export_import() {
    let t = TempDir::new("reexport");
    t.write("base.uwu", "export fn base(): number { 1 }");
    t.write("todo.uwu", "export import \"base.uwu\";\nexport import <math>;\nexport fn mas(): number { base() + 1 }");
    let main = t.write("main.uwu", r#"import "todo.uwu"; print(base(), mas(), math::PI > 3);"#);
    assert_eq!(exec_file(&main), vec!["1 2 true"]);
}

#[test]
fn modulo_importado_dos_veces_se_ejecuta_una_vez() {
    let t = TempDir::new("diamante");
    t.write("contador.uwu", r#"print("cargando contador"); export fn uno(): number { 1 }"#);
    t.write("a.uwu", r#"import "contador.uwu"; export fn a(): number { uno() }"#);
    t.write("b.uwu", r#"import "contador.uwu"; export fn b(): number { uno() + 1 }"#);
    let main = t.write("main.uwu", r#"import "a.uwu"; import "b.uwu"; print(a() + b());"#);
    assert_eq!(exec_file(&main), vec!["cargando contador", "3"]);
}

#[test]
fn importacion_circular() {
    let t = TempDir::new("circular");
    t.write("a.uwu", r#"import "b.uwu"; export fn a(): number { 1 }"#);
    t.write("b.uwu", r#"import "a.uwu"; export fn b(): number { 2 }"#);
    let main = t.write("main.uwu", r#"import "a.uwu"; print(a());"#);
    let errs = file_errors(&main);
    assert!(errs[0].0.starts_with("importación circular:"), "{:?}", errs);
}

#[test]
fn archivo_inexistente() {
    let t = TempDir::new("inexistente");
    let main = t.write("main.uwu", r#"import "no_existe.uwu";"#);
    assert_eq!(file_errors(&main)[0].0, "no se encontró el archivo `no_existe.uwu`");
}

#[test]
fn errores_se_reportan_en_el_archivo_correcto() {
    let t = TempDir::new("error_en_modulo");
    t.write("roto.uwu", r#"export fn f(): number { "no soy un número" }"#);
    let main = t.write("main.uwu", r#"import "roto.uwu"; print(f());"#);
    let errs = file_errors(&main);
    assert_eq!(errs.len(), 1);
    assert_eq!(errs[0].1, "roto.uwu");
}

#[test]
fn tipos_privados_homonimos_no_chocan() {
    let t = TempDir::new("homonimos");
    t.write("a.uwu", r#"type T = { a: number }; export fn hacer_a(): number { let x: T = { a: 1 }; x.a }"#);
    t.write("b.uwu", r#"type T = { b: string }; export fn hacer_b(): string { let x: T = { b: "b" }; x.b }"#);
    let main = t.write("main.uwu", r#"import "a.uwu"; import "b.uwu"; print(hacer_a(), hacer_b());"#);
    assert_eq!(exec_file(&main), vec!["1 b"]);
}

#[test]
fn no_se_puede_modificar_lo_importado() {
    let t = TempDir::new("mutar_importado");
    t.write("conf.uwu", "export let mut NIVEL = 1;");
    let main = t.write("main.uwu", r#"import "conf.uwu"; NIVEL = 2;"#);
    assert!(file_errors(&main)[0].0.contains("fue importado de otro módulo"));
}

#[test]
fn import_solo_en_el_nivel_superior() {
    assert!(errors("fn f() { import <fs>; }")[0].contains("sólo puede usarse en el nivel superior"));
}

#[test]
fn export_module_tiene_que_ir_primero() {
    assert!(errors("let a = 1; export module x;")[0].contains("primera declaración"));
}

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

#[test]
fn literales_hex_octal_binario() {
    assert_eq!(exec("print(0xff, 0o755, 0b1010, 1_000);"), vec!["255 493 10 1000"]);
    assert_eq!(exec("print(0o755.to_base(8), 255.to_base(16), (-5).to_base(2));"), vec!["755 ff -101"]);
}

// ======================= métodos nuevos =======================

#[test]
fn metodos_de_string() {
    let out = exec(
        r#"
        print("a-b-c".replace("-", "+"), "hola mundo".slice(0, 4), "hola".slice(-2, 4), "hola".index_of("la"));
        print("uno\ndos".lines(), "".is_empty(), "x".is_empty());
        match " 3.5 ".to_number() {
            { ok: true, valor } => print(valor * 2),
            { ok: false } => print("no es un número"),
        }
        match "abc".to_number() {
            { ok: true, valor } => print(valor),
            { ok: false } => print("no es un número"),
        }"#,
    );
    assert_eq!(out, vec!["a+b+c hola la 2", r#"["uno", "dos"] true false"#, "7", "no es un número"]);
}

#[test]
fn metodos_de_arreglo() {
    let out = exec(
        r#"
        let v = [3, 1, 2];
        print(v.sort(), v, ["b", "a"].sort(), v.slice(1, 3), v.index_of(2), v.index_of(9), v.is_empty());"#,
    );
    assert_eq!(out, vec![r#"[1, 2, 3] [3, 1, 2] ["a", "b"] [1, 2] 2 -1 false"#]);
}

#[test]
fn sort_solo_para_numeros_o_strings() {
    assert!(errors("let v = [{ a: 1 }]; v.sort();")[0].contains("no tiene el método `sort`"));
}

// ======================= ejemplos =======================

#[test]
fn ejemplos() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if !name.ends_with(".uwu") || name == "errores.uwu" {
            continue;
        }
        exec_file(&path); // falla si hay errores
    }
}

#[test]
fn ejemplo_de_modulos() {
    let main = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/modulos/main.uwu");
    let out = exec_file(&main);
    assert!(out.iter().any(|l| l.contains("perímetro")), "{:?}", out);
}

#[test]
fn errores_uwu_tiene_exactamente_7_errores() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/errores.uwu");
    let code = std::fs::read_to_string(path).unwrap();
    assert_eq!(errors(&code).len(), 7);
}
