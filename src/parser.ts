import type { Token } from "moo";
import Stack from "./stack";
import Cursor from "./cursor";
import { InternalError } from "./InternalError";

export const enum State {
    DEFINITION,
    NAMING,
    TYPING,
    ASSIGNMENT,
};

export class Parser {

    tokens: Token[];
    cursors: Cursor;
    token: Token;
    sub_tokens: Token[] = [];
    stack: Stack = Stack.getInstance();
    states: State[] = [];
    code: string;

    constructor(tokens: Token[], code: string) {
        this.tokens = tokens;
        this.cursors = new Cursor(0);
        this.token = this.tokens[this.cursors.getRoot()];
        this.code = code;
    }

    ignoreWS() {

        while (this.tokens[this.cursors.get()].type === "WS") {
            this.cursors.next();
        }

    }

    stackeable() {

        this.cursors.create();

        this.states.push(State.DEFINITION);

        const token_type = this.tokens[this.cursors.get()];

        if (token_type.type !== "definition") {
            throw new InternalError(`Unexpected token '${token_type.value}'`, token_type.line, token_type.col, this.code);
        }

        this.stack.define(token_type.value === "let");

        this.states.push(State.NAMING);

        this.cursors.next();
        this.ignoreWS();

        const name_token = this.tokens[this.cursors.get()];

        if (name_token.type !== "identifier") {
            throw new InternalError(`Unexpected token '${name_token.value}'`, name_token.line, name_token.col, this.code);
        }

        this.stack.naming(name_token.value);

        this.cursors.next();
        this.ignoreWS();

        //const assignment = this.tokens[this.cursors.get()];

        this.cursors.delete();

    }

    parser() {

        while (this.cursors.getRoot() < this.tokens.length) {

            this.token = this.tokens[this.cursors.getRoot()];

            switch (this.token.type) {

                case "definition": {
                    this.stackeable();
                    break;
                }

                case "type": {
                    console.log(this.token.value);
                    break;
                }

                default: {
                    break;
                }

            }

            this.cursors.edit(0, 1);

        }

    }

}