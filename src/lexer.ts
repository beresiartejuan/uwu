import moo from "moo";

const lexer = moo.compile({
    // keyword: ["let", "class", "function"],
    definition: { match: /(?:function|const|let|class)/ },
    type: { match: /(?:string|number|dict|list|object|null|boolean)/ },
    stucture: { match: /(?:if|else)/ },
    WS: { match: /[ \t]+/, lineBreaks: true },// Espacios en blanco
    newline: { match: /\r?\n/, lineBreaks: true }, // Nuevas lineas
    comment: /\/\/.*?$/, // Comentarios
    number: /0|[1-9][0-9]*/, // Números
    string: { match: /"(?:\\["\\]|[^\n"\\])*"/, value: s => s.slice(1, -1) }, // Cadenas de texto
    identifier: /[a-zA-Z_][a-zA-Z0-9_]*/, // Identificadores
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