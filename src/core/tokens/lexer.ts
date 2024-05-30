import moo from 'moo';

import token_types from './token_types';
import definition from './definition';
import types from './types';
import literal from './literal';
import structure from './structute';

const definitions_match = [
    definition.definition_class,
    definition.definition_const,
    definition.definition_context,
    definition.definition_function,
    definition.definition_let
];

const structures_match = [
    structure.structure_else,
    structure.structure_if,
    structure.structure_for,
    structure.structure_while
];

const types_match = [
    types.type_string,
    types.type_number,
    types.type_null,
    types.type_boolean,
    types.type_dict,
    types.type_list
];

const lexer = moo.compile({
    // Espacios en blanco, saltos de linea y comentarios
    [token_types.WS]: { match: /[ \t]+/, lineBreaks: true },
    [token_types.newline]: { match: /\r?\n/, lineBreaks: true },
    [token_types.comment]: /\/\/.*?$/,
    // Tipos
    [token_types.string]: { match: /"(?:\\["\\]|[^\n"\\])*"/, value: s => s.slice(1, -1) },
    [token_types.number]: /0|[1-9][0-9]*/,
    [token_types.boolean]: { match: [literal.false, literal.true] },
    // Tokens importantes
    [token_types.rule]: { match: [literal.use] },
    [token_types.definition]: { match: definitions_match },
    [token_types.stucture]: { match: structures_match },
    [token_types.type]: { match: types_match },
    [token_types.identifier]: /[a-zA-Z_][a-zA-Z0-9_]*/,
    // Operadores
    [token_types.assignment]: /=/,
    [token_types.dot]: /\./,
    [token_types.plus]: /\+/,
    [token_types.minus]: /-/,
    [token_types.asterisk]: /\*/,
    [token_types.slash]: /\//,
    [token_types.percent]: /%/,
    [token_types.caret]: /\^/,
    // Logica
    [token_types.lessThan]: /</,
    [token_types.greaterThan]: />/,
    [token_types.equal]: /==/,
    [token_types.notEqual]: /!=/,
    [token_types.lessThanOrEqual]: /<=/,
    [token_types.greaterThanOrEqual]: />=/,
    // Symbols
    [token_types.exclamation]: /!/,
    [token_types.question]: /\?/,
    [token_types.colon]: /:/,
    [token_types.lparen]: /\(/,
    [token_types.rparen]: /\)/,
    [token_types.lbrace]: /\{/,
    [token_types.rbrace]: /\}/,
    [token_types.lbracket]: /\[/,
    [token_types.rbracket]: /\]/,
    [token_types.semicolon]: /;/,
    [token_types.comma]: /,/,
});

export default lexer;
