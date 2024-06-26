import moo from "moo";

import { uwu, owo } from "./rules";

const lexer = moo.compile({

    [uwu.rule]: {
        match: [owo.rule_use]
    },

    [uwu.definition]: {
        match: [owo.definition_function, owo.definition_const, owo.definition_let, owo.definition_class, owo.definition_context]
    },

    [uwu.stucture]: { match: /(?:\bif\b|\belse\b)/ },

    [uwu.type]: {
        match: [owo.type_string, owo.type_number, owo.type_null, owo.type_boolean, owo.type_dict, owo.type_list]
    },

    [uwu.identifier]: /[a-zA-Z_][a-zA-Z0-9_]*/,

    // Otros...
    [uwu.boolean]: { match: [owo.boolean_false, owo.boolean_true] },
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