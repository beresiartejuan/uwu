//! Los programas de `examples/` corren sin errores.

use super::*;

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
