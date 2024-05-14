import moo from "moo";

import { uwu, owo } from "./rules";

const lexer = moo.compile({

    [uwu.definition]: {
        match: [owo.definition_function, owo.definition_const, owo.definition_let, owo.definition_class]
    },

    [uwu.stucture]: { match: /(?:\bif\b|\belse\b)/ },

    [uwu.type]: {
        match: [owo.type_string, owo.type_number, owo.type_null, owo.type_boolean]
    },

    [uwu.identifier]: /[a-zA-Z_][a-zA-Z0-9_]*/,

    // Otros...
    [uwu.WS]: { match: /[ \t]+/, lineBreaks: true },// Espacios en blanco
    [uwu.newline]: { match: /\r?\n/, lineBreaks: true }, // Nuevas lineas
    [uwu.comment]: /\/\/.*?$/, // Comentarios
    [uwu.number]: /0|[1-9][0-9]*/, // Números
    [uwu.string]: { match: /"(?:\\["\\]|[^\n"\\])*"/, value: s => s.slice(1, -1) }, // Cadenas de texto
    [uwu.lparen]: /\(/, // Paréntesis izquierdo
    [uwu.rparen]: /\)/, // Paréntesis derecho
    [uwu.lbrace]: /\{/, // Llave izquierda
    [uwu.rbrace]: /\}/, // Llave derecha
    [uwu.semicolon]: /;/, // Punto y coma
    [uwu.assignment]: /=/, // Signo de asignación
    [uwu.comma]: /,/, // Coma,
    [uwu.plus]: '+', // Suma
    [uwu.minus]: '-', // Resta
    [uwu.colon]: ':', // Dos puntitos
    [uwu.lessThan]: '<', // Token para "<"
    [uwu.greaterThan]: '>', // Token para ">",
    [uwu.lbracket]: '[',
    [uwu.rbracket]: ']',
});

export default lexer;