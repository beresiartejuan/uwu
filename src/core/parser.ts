import type { Token } from "moo";
import Stack from "./stack";
import Cursor from "./cursor";
import lexer from "./lexer";

import handler from "../errors/handler";

import { uwu, owo } from "./rules";

export const enum State {
    DEFINITION,
    NAMING,
    TYPING,
    ASSIGNMENT,
};

export class Parser {

    lexer: moo.Lexer;
    error;

    tokens: Token[];
    cursors: Cursor;
    token?: Token;
    sub_tokens: Token[] = [];
    stack: Stack = Stack.getInstance();
    states: State[] = [];
    code?: string;

    constructor() {
        this.error = handler;
        this.lexer = lexer;
        this.tokens = [];
        this.cursors = new Cursor(0);
    }

    setLexer(lexer: moo.Lexer) {
        this.lexer = lexer;
    }

    ignoreWS() {

        while (this.tokens[this.cursors.get()]?.type === uwu.WS) {
            this.cursors.next();
        }

    }

    define() { }

    evaluate() { }

    /**
     * Esta función se activa cuando el codigo detecta que algo
     * se debe guardar en memoria, como una función, clase, variables, ect...
     */
    stackeable() {

        this.cursors.create();

        this.states.push(State.DEFINITION);

        const token_type = this.tokens[this.cursors.get()];

        if (token_type.type !== uwu.definition) {
            this.error(this.token!, this.code!).unexpected_token();
        }

        this.stack.define(token_type.value === "let");

        this.states.push(State.NAMING);

        this.cursors.next();
        this.ignoreWS();

        const name_token = this.tokens[this.cursors.get()];

        if (name_token.type !== "identifier") {
            //throw new InternalError(`Unexpected token '${name_token.value}'`, name_token.line, name_token.col, this.code);
        }

        this.stack.naming(name_token.value);

        this.cursors.next();
        this.ignoreWS();

        //const assignment = this.tokens[this.cursors.get()];

        this.cursors.delete();

    }

    parser(code: string) {

        this.code = code;

        this.tokens = Array.from(this.lexer.reset(this.code));

        while (this.cursors.get() < this.tokens.length) {

            this.token = this.tokens[this.cursors.get()];

            if (!this.token) continue;

            if (!this.token.type) {
                this.error(this.token, this.code).unexpected_token();
            }

            this.ignoreWS();

            if (this.token.type === uwu.definition) {
                this.stackeable();
            }

            if (this.token.type === uwu.stucture) {
                this.evaluate();
            }

            this.cursors.next();

        }

    }

}