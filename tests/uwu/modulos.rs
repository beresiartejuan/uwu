//! `import`, `export` y namespaces, con varios archivos.

use super::*;

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

// ======================= namespaces =======================

#[test]
fn namespace_en_el_mismo_archivo() {
    let code = r#"
      namespace geo {
          type Punto = { x: number, y: number };
          let mut llamadas = 0;
          fn cuadrado(n: number): number { n * n }
          fn norma(p: Punto): number { llamadas += 1; (cuadrado(p.x) + cuadrado(p.y)).sqrt() }
      }
      let p: geo::Punto = { x: 0, y: 1 };
      print(geo::norma(p), geo::llamadas);
      geo::llamadas = 10;
      fn main() { print(geo::norma({ x: 3, y: 4 }), geo::llamadas); }"#;
    assert_eq!(exec(code), vec!["1 1", "5 11"]);
    assert_eq!(errors("fn f(): number { a::x } namespace a { let x = 1; }"), vec!["`a::x` se usa antes de declararse"]);
}

#[test]
fn using_con_namespaces() {
    let code = r#"
      namespace texto { type Saludo = string; fn gritar(s: string): string { s.upper() + "!" } }
      using texto::gritar;
      print(gritar("hola"));
      using namespace texto;
      let s: Saludo = "chau";
      print(s);"#;
    assert_eq!(exec(code), vec!["HOLA!", "chau"]);
}

#[test]
fn namespace_errores() {
    assert_eq!(errors("namespace a { fn f(): number { 1 } } a::g();"), vec!["el namespace `a` no tiene `g`"]);
    assert_eq!(errors("namespace a { } print(a);"), vec!["`a` es un namespace, no un valor"]);
    assert_eq!(
        errors("namespace a { let x = 1; } a::x = 2;"),
        vec!["no se puede modificar `a::x` porque es inmutable"]
    );
    assert_eq!(errors("import <math>; namespace math { }"), vec!["el namespace `math` ya existe"]);
    assert!(errors("namespace a { print(1); }")[0].starts_with("dentro de un `namespace` sólo puede haber"));
    assert_eq!(errors("namespace a { export fn f() {} }"), vec!["dentro de un `namespace` no se usa `export`"]);
    assert_eq!(errors("namespace a { namespace b { } }"), vec!["los `namespace` no se pueden anidar"]);
    assert!(errors("fn f() { namespace a { } }")[0].contains("sólo puede usarse en el nivel superior"));
}

#[test]
fn export_namespace_entre_archivos() {
    let t = TempDir::new("export_namespace");
    t.write(
        "geo.uwu",
        r#"export namespace geo {
             type Punto = { x: number, y: number };
             let ORIGEN: Punto = { x: 0, y: 0 };
             fn distancia(a: Punto, b: Punto): number { ((a.x - b.x).pow(2) + (a.y - b.y).pow(2)).sqrt() }
           }
           namespace privado { fn f(): number { 1 } }
           export let (A, B) = (1, 2);"#,
    );
    t.write("todo.uwu", "export import \"geo.uwu\";");
    let main = t.write(
        "main.uwu",
        r#"import "todo.uwu";
           let p: geo::Punto = { x: 3, y: 4 };
           using namespace geo;
           print(geo::distancia(ORIGEN, p), A + B);"#,
    );
    assert_eq!(exec_file(&main), vec!["5 3"]);
    let privado = t.write("privado.uwu", "import \"geo.uwu\"; print(privado::f());");
    assert!(file_errors(&privado)[0].0.contains("no existe el módulo `privado`"));
    let modifica = t.write("modifica.uwu", "import \"geo.uwu\"; geo::ORIGEN.x = 5;");
    assert!(file_errors(&modifica)[0].0.contains("fue importado de otro módulo"));
}
