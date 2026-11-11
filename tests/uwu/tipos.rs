//! El sistema de tipos: uniones, objetos, genéricos y `match`.

use super::*;

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
