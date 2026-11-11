//! El REPL.

use uwu::repl::Repl;

fn repl_session(inputs: &[&str]) -> Vec<String> {
    let mut printed: Vec<String> = Vec::new();
    let mut shown: Vec<String> = Vec::new();
    {
        let mut out = |l: &str| printed.push(l.to_string());
        let mut repl = Repl::new(&mut out);
        for input in inputs {
            let r = repl.eval(input);
            let mut lines: Vec<String> = r.diagnostics.iter().map(|d| format!("error: {}", d.message)).collect();
            lines.extend(r.runtime_error.map(|d| format!("pánico: {}", d.message)));
            lines.extend(r.value);
            shown.push(lines.join(" | "));
        }
    }
    shown.extend(printed.into_iter().map(|l| format!("print: {}", l)));
    shown
}

#[test]
fn repl_conserva_el_estado() {
    let out = repl_session(&[
        "let x = 5",
        "fn doble(n: number): number { n * 2 }",
        "doble(x)",
        "\"x vale {x}\"",
        "fn doble(n: number): number { n * 3 }",
        "doble(x)",
        "import <math>",
        "math::PI > 3",
        "print(1)",
    ]);
    assert_eq!(out, vec!["", "", "10", "\"x vale 5\"", "", "15", "", "true", "", "print: 1"]);
}

#[test]
fn repl_descarta_las_entradas_con_errores() {
    let out = repl_session(&["let y: string = 1;", "y", "let z = 1; panic(\"ups\")", "z"]);
    assert_eq!(
        out,
        vec![
            "error: tipos incompatibles: se esperaba `string`, se encontró `1`",
            "error: no existe `y` en este ámbito",
            "pánico: pánico: ups",
            "error: no existe `z` en este ámbito",
        ]
    );
}

#[test]
fn repl_tipos_y_entradas_incompletas() {
    let mut out = |_: &str| {};
    let mut repl = Repl::new(&mut out);
    repl.eval("fn f(n: number): string { n.to_string() }");
    assert_eq!(repl.type_of("f").unwrap(), "(number) => string");
    assert_eq!(repl.type_of("(f(1), [true])").unwrap(), "(string, boolean[])");
    assert!(Repl::is_incomplete("fn f() {\n"));
    assert!(Repl::is_incomplete("let x = (1,\n"));
    assert!(!Repl::is_incomplete("let x = \"{\";\n"));
}
