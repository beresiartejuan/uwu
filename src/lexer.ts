import moo from "moo";

const lexer = moo.compile({

    definition: { match: /(?:\bfunction\b|\bconst\b|\blet\b|\bclass\b)/ },

    stucture: { match: /(?:\bif\b|\belse\b)/ },

    type: { match: /(?:\bstring\b|\bnumber\b|\bdict\b|\blist\b|\bobject\b|\bnull\b|\bboolean\b)/ },

    identifier: /[a-zA-Z_][a-zA-Z0-9_]*/,

    // Otros...
    WS: { match: /[ \t]+/, lineBreaks: true },// Espacios en blanco
    newline: { match: /\r?\n/, lineBreaks: true }, // Nuevas lineas
    comment: /\/\/.*?$/, // Comentarios
    number: /0|[1-9][0-9]*/, // Números
    string: { match: /"(?:\\["\\]|[^\n"\\])*"/, value: s => s.slice(1, -1) }, // Cadenas de texto
    lparen: /\(/, // Paréntesis izquierdo
    rparen: /\)/, // Paréntesis derecho
    lbrace: /\{/, // Llave izquierda
    rbrace: /\}/, // Llave derecha
    semicolon: /;/, // Punto y coma
    assignment: /=/, // Signo de asignación
    comma: /,/, // Coma,
    plus: '+', // Suma
    minus: '-', // Resta
    colon: ':', // Dos puntitos
    lessThan: '<', // Token para "<"
    greaterThan: '>', // Token para ">",
    lbracket: '[',
    rbracket: ']',
});

export default lexer;