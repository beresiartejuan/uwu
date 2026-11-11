//! Errores de sintaxis y pánicos en ejecución.

use super::*;

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

#[test]
fn salida_cerrada_no_es_un_panico() {
    // `uwu programa.uwu | head -1`: el programa tiene que terminar en silencio
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};
    let dir = TempDir::new("salida_cerrada");
    let path = dir.write("mucho.uwu", "import <io>;\nfor i in 0..1000000 { print(i); io::write(\"x\\n\"); }\n");
    let mut child = Command::new(env!("CARGO_BIN_EXE_uwu"))
        .arg(&path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut first = String::new();
    BufReader::new(child.stdout.take().unwrap()).read_line(&mut first).unwrap();
    assert_eq!(first, "0\n");
    // al salir de este punto se cerró el extremo de lectura del pipe
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "{:?}", out.status);
    assert_eq!(String::from_utf8_lossy(&out.stderr), "");
}
