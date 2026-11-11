//! Expresiones, control de flujo, funciones, métodos, tuplas e interpolación.

use super::*;

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

#[test]
fn literales_hex_octal_binario() {
    assert_eq!(exec("print(0xff, 0o755, 0b1010, 1_000);"), vec!["255 493 10 1000"]);
    assert_eq!(exec("print(0o755.to_base(8), 255.to_base(16), (-5).to_base(2));"), vec!["755 ff -101"]);
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

// ======================= métodos =======================

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

// ======================= tuplas y desestructuración =======================

#[test]
fn tuplas_basicas() {
    let code = r#"
      fn dividir(a: number, b: number): (number, number) { ((a / b).floor(), a % b) }
      let t = dividir(17, 5);
      let anidada = ((1, 2), "x");
      print(t, t.0, t.1, anidada.0.1, (5,), t == (3, 2));"#;
    assert_eq!(exec(code), vec!["(3, 2) 3 2 2 (5,) true"]);
}

#[test]
fn tuplas_mutables_y_tipadas() {
    assert_eq!(exec(r#"let mut t: (number, string) = (1, "a"); t.0 += 1; print(t);"#), vec![r#"(2, "a")"#]);
    assert_eq!(errors("let t = (1, 2); t.0 = 5;"), vec!["no se puede modificar `t` porque es inmutable"]);
    assert_eq!(
        errors(r#"let t: (number, string) = (1, 2);"#),
        vec!["tipos incompatibles: se esperaba `(number, string)`, se encontró `(1, 2)`"]
    );
}

#[test]
fn indice_de_tupla_fuera_de_rango() {
    assert_eq!(errors("let t = (1, 2); print(t.2);"), vec!["la tupla `(number, number)` no tiene el elemento `2`"]);
    assert_eq!(errors("let xs = [1]; print(xs.0);"), vec!["los arreglos no tienen el campo `0`"]);
}

#[test]
fn desestructuracion_en_let() {
    let code = r#"
      let (a, (b, c)) = (1, (2, "tres"));
      let { x, y: otro } = { x: 10, y: 20 };
      let (mut m, _) = (5, true);
      m *= 2;
      print(a, b, c, x, otro, m);"#;
    assert_eq!(exec(code), vec!["1 2 tres 10 20 10"]);
}

#[test]
fn desestructuracion_tiene_que_ser_irrefutable() {
    assert_eq!(
        errors("let (a, b) = (1, 2, 3);")[0],
        "el patrón tiene 2 elemento(s), pero la tupla `(number, number, number)` tiene 3"
    );
    assert_eq!(
        errors("let (0, b) = (1, 2);")[0],
        "el patrón de `let` no cubre todos los valores: falta `(number, number)`"
    );
    assert_eq!(errors("let { b } = { a: 1 };")[0], "el tipo `{ a: number }` no tiene el campo `b`");
    assert_eq!(errors("let (a, b) = (1, 2); a = 3;"), vec!["no se puede modificar `a` porque es inmutable"]);
    assert!(errors("let mut (a, b) = (1, 2);")[0].contains("`mut` va dentro del patrón"));
}

#[test]
fn for_con_patrones_enumerate_y_zip() {
    let code = r#"
      let mut s = "";
      for (i, c) in ["a", "b"].enumerate() { s += "{i}{c} "; }
      for (n, nombre) in [1, 2].zip(["uno", "dos", "tres"]) { s += "{n}={nombre} "; }
      for { x, y } in [{ x: 1, y: 2 }] { s += x + y; }
      print(s);"#;
    assert_eq!(exec(code), vec!["0a 1b 1=uno 2=dos 3"]);
}

#[test]
fn match_con_tuplas_y_exhaustividad() {
    let code = r#"
      fn f(t: (boolean, boolean)): string {
          match t { (true, true) => "ambos", (true, false) => "primero", (false, _) => "otro" }
      }
      print(f((true, false)), f((false, true)));"#;
    assert_eq!(exec(code), vec!["primero otro"]);
    assert!(errors("fn f(t: (boolean, number)): number { match t { (true, n) => n } }")[0]
        .contains("falta cubrir `(false, number)`"));
}

// ======================= interpolación de strings =======================

#[test]
fn interpolacion() {
    let code = r#"
      let nombre = "Juan";
      let xs = [1, 2];
      print("hola {nombre}, {xs.len()} números: {xs}, suma {xs.sum() * 10} {(1, "a")}");
      print("anidado: {"<{nombre.upper()}>"} y {{llaves}} }");"#;
    assert_eq!(exec(code), vec![r#"hola Juan, 2 números: [1, 2], suma 30 (1, "a")"#, "anidado: <JUAN> y {llaves} }"]);
}

#[test]
fn interpolacion_errores() {
    assert_eq!(errors(r#"print("hola {}");"#), vec!["falta una expresión dentro de `{}`"]);
    assert_eq!(errors(r#"print("hola {x");"#), vec!["falta `}` para cerrar la interpolación"]);
    assert_eq!(errors(r#"print("{a b}");"#), vec!["se esperaba `}` para cerrar la interpolación, se encontró `b`"]);
    assert_eq!(errors(r#"fn f() {} print("{f()}");"#), vec!["no se puede interpolar algo de tipo `void`"]);
    assert_eq!(errors(r#"fn f() {} print("{f}");"#), vec!["no se puede interpolar una función (`() => void`)"]);
    assert_eq!(errors(r#"print("{nada}");"#), vec!["no existe `nada` en este ámbito"]);
}
