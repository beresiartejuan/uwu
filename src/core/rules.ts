/**
 * Enum "uwu":
 * - Define tipos de tokens utilizados en el lexer y el parser.
 * - Los nombres de los elementos están en mayúsculas y representan tipos de tokens.
 * - Los valores de los elementos son cadenas de texto que representan los nombres de los tipos de tokens.
 * - Utilizado para mantener una lista centralizada de tipos de tokens compartidos entre el lexer y el parser.
 * 
 * Enum "owo":
 * - Define los tokens específicos detectados por el lexer, relacionándolos con los tipos definidos en el enum "uwu".
 * - Los nombres de los elementos están en minúsculas y representan tokens específicos detectados por el lexer.
 * - Los valores de los elementos son las cadenas de texto asociadas a los tokens específicos detectados por el lexer.
 * - Utilizado para asignar nombres más descriptivos a los tokens detectados por el lexer, manteniendo la consistencia con los tipos definidos en "uwu".
 */

export const enum uwu {
    definition = "definition",
    stucture = "structure",
    type = "type",
    identifier = "identifier",
    boolean = 'boolean',
    WS = "WS",
    newline = "newline",
    comment = "comment",
    number = "number",
    string = "string",
    lparen = "lparen",
    rparen = "rparen",
    lbrace = "lbrace",
    rbrace = "rbrace",
    semicolon = "semicolon",
    assignment = "assignment",
    comma = "comma",
    plus = "plus",
    minus = "minus",
    colon = "colon",
    lessThan = "lessThan",
    greaterThan = "greaterThan",
    lbracket = "lbracket",
    rbracket = "rbracket",
}

export const enum owo {
    definition_function = "fn",
    definition_const = "const",
    definition_let = "let",
    definition_class = "class",
    structure_if = "if",
    structure_else = "else",
    type_string = "string",
    type_number = "number",
    type_dict = "dict",
    type_list = "list",
    type_object = "object",
    type_null = "null",
    type_boolean = "boolean",
    boolean_false = 'false',
    boolean_true = 'true'
}
