import type { Token } from "moo";
import Stack from "./stack";
import Cursor from "./cursor";

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

    constructor(tokens: Token[]) {
        this.tokens = tokens;
        this.cursors = new Cursor(0);
        this.token = this.tokens[this.cursors.getRoot()];
    }

    ignoreWS() {

        while (this.tokens[this.cursors.get()].type === "WS") {
            this.cursors.next();
        }

    }

    stackeable() {

        this.cursors.create();

        const token_type = this.tokens[this.cursors.get()];

        if (token_type.type !== "definition") {
            throw new Error(`Internal error: ${token_type.type} not must be a definition token`);
        }

        this.stack.define(token_type.value === "let");

        this.cursors.next();
        this.ignoreWS();

        const name_token = this.tokens[this.cursors.get()];

        if (name_token.type !== "identifier") {
            throw new Error(`Internal error: ${token_type.type} not must be a identifier`);
        }

        this.stack.naming(name_token.value);

    }

    parser() {

        while (this.cursors.getRoot() < this.tokens.length) {

            this.token = this.tokens[this.cursors.getRoot()];

            switch (this.token.type) {

                case "definition": {
                    this.stackeable();
                }

                default: {
                    break;
                }

            }

            this.cursors.edit(0, 1);

        }

    }

}