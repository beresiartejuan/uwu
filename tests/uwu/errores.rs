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
