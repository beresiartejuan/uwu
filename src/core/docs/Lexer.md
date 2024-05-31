# Lexer and Tokens

## Introduction
This document provides an overview of the lexer used for tokenizing the input code. The lexer is implemented using the `moo` library and is designed to recognize various types of tokens defined by enums such as `definition`, `structure`, `types`, `literal`, and `token_types`.

## Enums

### `definition`
The `definition` enum contains keywords used for defining various constructs in the language.

```typescript
const enum definition {
    definition_context = "ctx",
    definition_function = "fn",
    definition_const = "const",
    definition_let = "let",
    definition_class = "class",
}
```

### `structure`
The `structure` enum contains keywords for control structures.

```typescript
const enum structure {
    structure_if = "if",
    structure_else = "else",
    structure_while = "while",
    structure_for = "for"
}
```

### `types`
The `types` enum contains keywords for data types.

```typescript
const enum types {
    type_string = "string",
    type_number = "number",
    type_dict = "dict",
    type_list = "list",
    type_object = "object",
    type_null = "null",
    type_boolean = "boolean",
}
```

### `literal`
The `literal` enum contains specific literal values.

```typescript
const enum literal {
    use = "use",
    false = "false",
    true = "true"
}
```

### `token_types`
The `token_types` enum defines all possible token types that the lexer can recognize.

```typescript
const enum token_types {
    definition = "definition",
    rule = "rule",
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
    dot = "dot",
    asterisk = "asterisk",
    slash = "slash",
    percent = "percent",
    caret = "caret",
    equal = "equal",
    notEqual = "notEqual",
    lessThanOrEqual = "lessThanOrEqual",
    greaterThanOrEqual = "greaterThanOrEqual",
    exclamation = "exclamation",
    question = "question"
}
```

## Lexer Definition
The lexer is defined using the `moo.compile` method, which takes an object where keys are token types and values define how to match those tokens.

```typescript
import moo from 'moo';
import definition from './definition';
import structure from './structure';
import types from './types';
import literal from './literal';
import token_types from './token_types';

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
    // Whitespace, newlines, and comments
    [token_types.WS]: { match: /[ \t]+/, lineBreaks: true },
    [token_types.newline]: { match: /\r?\n/, lineBreaks: true },
    [token_types.comment]: /\/\/.*?$/,

    // Data types
    [token_types.string]: { match: /"(?:\\["\\]|[^\n"\\])*"/, value: s => s.slice(1, -1) },
    [token_types.number]: /0|[1-9][0-9]*/,
    [token_types.boolean]: { match: [literal.false, literal.true] },

    // Important tokens
    [token_types.rule]: { match: [literal.use] },
    [token_types.definition]: { match: definitions_match },
    [token_types.structure]: { match: structures_match },
    [token_types.type]: { match: types_match },
    [token_types.identifier]: /[a-zA-Z_][a-zA-Z0-9_]*/,

    // Operators
    [token_types.assignment]: /=/,
    [token_types.dot]: /\./,
    [token_types.plus]: /\+/,
    [token_types.minus]: /-/,
    [token_types.asterisk]: /\*/,
    [token_types.slash]: /\//,
    [token_types.percent]: /%/,
    [token_types.caret]: /\^/,

    // Logic
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
```

### Token Types Explained

- **Whitespace and Comments**
  - `WS`: Matches spaces and tabs, allowing for line breaks.
  - `newline`: Matches newlines, allowing for line breaks.
  - `comment`: Matches single-line comments starting with `//`.

- **Data Types**
  - `string`: Matches string literals enclosed in double quotes.
  - `number`: Matches integers.
  - `boolean`: Matches `true` and `false` literals.

- **Important Tokens**
  - `rule`: Matches the `use` keyword.
  - `definition`: Matches keywords for various definitions such as `class`, `const`, `ctx`, `fn`, `let`.
  - `structure`: Matches control structures such as `if`, `else`, `for`, `while`.
  - `type`: Matches data type keywords such as `string`, `number`, `null`, `boolean`, `dict`, `list`.
  - `identifier`: Matches identifiers (variable names, function names, etc.).

- **Operators**
  - `assignment`: Matches the `=` operator.
  - `dot`: Matches the `.` operator.
  - `plus`: Matches the `+` operator.
  - `minus`: Matches the `-` operator.
  - `asterisk`: Matches the `*` operator.
  - `slash`: Matches the `/` operator.
  - `percent`: Matches the `%` operator.
  - `caret`: Matches the `^` operator.

- **Logical Operators**
  - `lessThan`: Matches the `<` operator.
  - `greaterThan`: Matches the `>` operator.
  - `equal`: Matches the `==` operator.
  - `notEqual`: Matches the `!=` operator.
  - `lessThanOrEqual`: Matches the `<=` operator.
  - `greaterThanOrEqual`: Matches the `>=` operator.

- **Symbols**
  - `exclamation`: Matches the `!` symbol.
  - `question`: Matches the `?` symbol.
  - `colon`: Matches the `:` symbol.
  - `lparen`: Matches the `(` symbol.
  - `rparen`: Matches the `)` symbol.
  - `lbrace`: Matches the `{` symbol.
  - `rbrace`: Matches the `}` symbol.
  - `lbracket`: Matches the `[` symbol.
  - `rbracket`: Matches the `]` symbol.
  - `semicolon`: Matches the `;` symbol.
  - `comma`: Matches the `,` symbol.