import { readdirSync, readFileSync } from "fs";
import path from "path";
import { run } from "../index";

/** Ejecuta código uwu y devuelve lo que imprimió. Falla si hay errores de compilación. */
function exec(code: string): string[] {
  const out: string[] = [];
  const r = run(code, (l) => out.push(l));
  const errors = r.diagnostics.filter((d) => d.severity !== "warning");
  if (errors.length) throw new Error("errores inesperados:\n" + errors.map((d) => d.message).join("\n"));
  if (r.runtimeError) throw new Error("pánico: " + r.runtimeError.message);
  return out;
}

/** Devuelve los mensajes de error de compilación. */
function errors(code: string): string[] {
  return run(code, () => {}, true).diagnostics.filter((d) => d.severity !== "warning").map((d) => d.message);
}

function panic(code: string): string {
  const r = run(code, () => {});
  return r.runtimeError?.message ?? "";
}

describe("expresiones y variables", () => {
  test("aritmética y precedencia", () => {
    expect(exec("print(1 + 2 * 3, (1 + 2) * 3, 10 % 3, -2 * 3);")).toEqual(["7 9 1 -6"]);
  });
  test("concatenación de strings con números", () => {
    expect(exec(`print("x = " + 5 + "!");`)).toEqual(["x = 5!"]);
  });
  test("bloques e if como expresiones", () => {
    expect(exec("let a = { let b = 2; b * 10 }; let c = if a > 5 { \"grande\" } else { \"chico\" }; print(a, c);"))
      .toEqual(["20 grande"]);
  });
  test("mutabilidad", () => {
    expect(exec("let mut x = 1; x += 4; x *= 2; print(x);")).toEqual(["10"]);
    expect(errors("let x = 1; x = 2;")).toEqual(["no se puede modificar `x` porque es inmutable"]);
    expect(errors("let v = [1]; v.push(2);")).toEqual(["no se puede modificar `v` porque es inmutable"]);
    expect(errors("let o = { a: 1 }; o.a = 2;")).toEqual(["no se puede modificar `o` porque es inmutable"]);
  });
  test("shadowing como en Rust", () => {
    expect(exec(`let x = 5; let x = x.to_string() + "!"; print(x);`)).toEqual(["5!"]);
  });
  test("igualdad estructural", () => {
    expect(exec("print([1, 2] == [1, 2], { a: 1 } == { a: 2 });")).toEqual(["true false"]);
  });
});

describe("control de flujo", () => {
  test("while, loop, break y continue", () => {
    const code = `
      let mut i = 0;
      let mut pares: number[] = [];
      loop {
        i += 1;
        if i > 10 { break; }
        if i % 2 == 1 { continue; }
        pares.push(i);
      }
      print(pares);`;
    expect(exec(code)).toEqual(["[2, 4, 6, 8, 10]"]);
  });
  test("for sobre rangos, arreglos y strings", () => {
    expect(exec(`let mut s = ""; for c in "abc" { s = c + s; } for i in 0..3 { s += i; } print(s);`)).toEqual(["cba012"]);
  });
  test("return anticipado", () => {
    expect(exec("fn f(x: number): string { if x < 0 { return \"neg\"; } \"pos\" } print(f(-1), f(1));")).toEqual(["neg pos"]);
  });
  test("break fuera de un bucle", () => {
    expect(errors("break;")).toEqual(["`break` fuera de un bucle"]);
  });
});

describe("funciones y closures", () => {
  test("recursión mutua (las funciones se declaran antes de usarse)", () => {
    const code = `
      fn es_par(n: number): boolean { if n == 0 { true } else { es_impar(n - 1) } }
      fn es_impar(n: number): boolean { if n == 0 { false } else { es_par(n - 1) } }
      print(es_par(10), es_impar(7));`;
    expect(exec(code)).toEqual(["true true"]);
  });
  test("closures capturan su entorno", () => {
    const code = `
      fn sumador(n: number): (x: number) => number { |x| x + n }
      let mas5 = sumador(5);
      print(mas5(10));`;
    expect(exec(code)).toEqual(["15"]);
  });
  test("main se ejecuta automáticamente", () => {
    expect(exec(`fn main() { print("desde main"); }`)).toEqual(["desde main"]);
  });
  test("cantidad de argumentos", () => {
    expect(errors("fn f(a: number): number { a } f(1, 2);")).toEqual(["`f` espera 1 argumento(s), pero recibió 2"]);
  });
  test("falta valor de retorno", () => {
    expect(errors("fn f(): number { 1; }")[0]).toMatch(/debe devolver `number`/);
  });
  test("closure sin tipo inferible", () => {
    expect(errors("let f = |x| x;")[0]).toMatch(/no se puede inferir el tipo del parámetro `x`/);
  });
});

describe("sistema de tipos", () => {
  test("tipos literales y uniones", () => {
    expect(errors(`let d: "norte" | "sur" = "este";`)[0]).toMatch(/se esperaba `"norte" \| "sur"`/);
    expect(exec(`let d: "norte" | "sur" = "sur"; print(d);`)).toEqual(["sur"]);
  });
  test("objetos estructurales (como TS)", () => {
    const code = `
      type Punto = { x: number, y: number };
      fn norma(p: Punto): number { (p.x * p.x + p.y * p.y).sqrt() }
      let p3 = { x: 3, y: 4, z: 99 };
      print(norma(p3));`;
    expect(exec(code)).toEqual(["5"]);
  });
  test("campo faltante", () => {
    expect(errors(`type P = { x: number }; let p: P = { y: 1 };`)[0]).toMatch(/tipos incompatibles/);
  });
  test("no hay conversiones implícitas a boolean", () => {
    expect(errors(`if 1 { }`)[0]).toMatch(/se esperaba `boolean`/);
  });
  test("comparación imposible", () => {
    expect(errors(`let s: "a" | "b" = "a"; print(s == "c");`)[0]).toMatch(/siempre da false/);
  });
  test("arreglo vacío necesita anotación", () => {
    expect(errors(`let mut v = [];`)[0]).toMatch(/no se puede inferir el tipo de `v`/);
  });
  test("tipo desconocido con sugerencia", () => {
    const r = run("let x: int = 1;", () => {}, true);
    expect(r.diagnostics[0].hint).toBe("en uwu se escribe `number`");
  });
});

describe("genéricos", () => {
  test("inferencia de parámetros de tipo", () => {
    const code = `
      fn primero<T>(xs: T[]): T { xs[0] }
      let s: string = primero(["a", "b"]);
      let n: number = primero([1, 2]);
      print(s, n);`;
    expect(exec(code)).toEqual(["a 1"]);
  });
  test("el tipo inferido se respeta", () => {
    expect(errors(`fn id<T>(x: T): T { x } let s: string = id(5);`)[0]).toMatch(/se esperaba `string`, se encontró `number`/);
  });
  test("turbofish", () => {
    expect(exec(`fn vacio<T>(): T[] { [] } let v = vacio::<number>(); print(v.len());`)).toEqual(["0"]);
  });
  test("genéricos con closures", () => {
    const code = `
      fn aplicar<A, B>(x: A, f: (a: A) => B): B { f(x) }
      let largo: number = aplicar("hola", |s| s.len());
      print(largo);`;
    expect(exec(code)).toEqual(["4"]);
  });
  test("map / filter / fold", () => {
    expect(exec(`print([1, 2, 3].map(|x| x.to_string() + "!").join(" "), [1,2,3,4].filter(|x| x > 2), [1,2,3].fold("", |a, x| a + x));`))
      .toEqual(["1! 2! 3! [3, 4] 123"]);
  });
});

describe("match", () => {
  test("literales y comodín", () => {
    const code = `
      fn nombre(n: number): string { match n { 1 => "uno", 2 => "dos", _ => "muchos" } }
      print(nombre(1), nombre(2), nombre(7));`;
    expect(exec(code)).toEqual(["uno dos muchos"]);
  });
  test("narrowing por tipo en uniones", () => {
    const code = `
      fn describir(x: number | string | boolean): string {
        match x {
          n: number => "número " + (n * 2),
          s: string => "texto de " + s.len(),
          b: boolean => if b { "sí" } else { "no" },
        }
      }
      print(describir(21), describir("hola"), describir(false));`;
    expect(exec(code)).toEqual(["número 42 texto de 4 no"]);
  });
  test("exhaustividad con booleanos anidados", () => {
    const code = `
      type R = { ok: true, v: number } | { ok: false, e: string };
      fn f(r: R): string { match r { { ok: true, v } => "ok " + v, { ok: false, e } => e } }
      print(f({ ok: true, v: 1 }), f({ ok: false, e: "mal" }));`;
    expect(exec(code)).toEqual(["ok 1 mal"]);
  });
  test("exhaustividad partiendo uniones de literales", () => {
    expect(errors(`
      type C = { a: 1 | 2, b: boolean };
      fn f(c: C): number { match c { { a: 1 } => 1, { a: 2, b: true } => 2 } }`)[0])
      .toBe('`match` no exhaustivo: falta cubrir `{ a: 2, b: false }`');
  });
  test("los guardas no cuentan para la exhaustividad", () => {
    expect(errors(`fn f(b: boolean): number { match b { true if 1 > 0 => 1, false => 0 } }`)[0]).toMatch(/falta cubrir `true`/);
  });
  test("números necesitan comodín", () => {
    expect(errors(`fn f(n: number): number { match n { 0 => 1 } }`)[0]).toMatch(/falta cubrir `number`/);
  });
  test("patrón imposible", () => {
    expect(errors(`let s: "a" | "b" = "a"; match s { "z" => 1, _ => 2 };`)[0]).toMatch(/nunca coincide/);
  });
  test("acceder a campo de variante requiere match", () => {
    expect(errors(`type F = { t: "a", x: number } | { t: "b" }; let f: F = { t: "b" }; print(f.x);`)[0])
      .toMatch(/no todas las variantes/);
  });
});

describe("errores en ejecución", () => {
  test("índice fuera de rango", () => {
    expect(panic("let v = [1]; print(v[3]);")).toMatch(/índice fuera de rango/);
  });
  test("división por cero", () => {
    expect(panic("let a = 0; print(1 / a);")).toBe("división por cero");
  });
  test("panic y assert", () => {
    expect(panic(`panic("algo salió mal");`)).toBe("pánico: algo salió mal");
    expect(panic(`let a = 1; assert(a == 2);`)).toBe("la aserción falló");
  });
  test("recursión infinita", () => {
    expect(panic("fn f(n: number): number { f(n + 1) } f(0);")).toMatch(/desbordamiento de pila/);
  });
});

describe("errores de sintaxis", () => {
  test("falta punto y coma", () => {
    expect(errors("let a = 1\nlet b = 2;")[0]).toMatch(/se esperaba `;`/);
  });
  test("string sin cerrar", () => {
    expect(errors(`let s = "hola;`)[0]).toBe("string sin cerrar");
  });
  test("parámetro sin tipo", () => {
    expect(errors(`fn f(x) { }`)[0]).toBe("falta el tipo del parámetro `x`");
  });
});

describe("ejemplos", () => {
  const dir = path.join(__dirname, "../../examples");
  for (const file of readdirSync(dir).filter((f) => f.endsWith(".uwu") && f !== "errores.uwu")) {
    test(file, () => {
      expect(() => exec(readFileSync(path.join(dir, file), "utf8"))).not.toThrow();
    });
  }
  test("errores.uwu tiene exactamente 7 errores", () => {
    expect(errors(readFileSync(path.join(dir, "errores.uwu"), "utf8"))).toHaveLength(7);
  });
});
