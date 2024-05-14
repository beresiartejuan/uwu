import type { Token } from "moo";
import Stack from "./stack";
import Cursor from "./cursor";
import lexer from "./lexer";

import handler from "../errors/handler";

import { owo, uwu } from "./rules";

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
    stack: Stack = new Stack();
    code?: string;

    constructor() {
        this.error = handler;
        this.lexer = lexer;
        this.tokens = [];
        this.cursors = new Cursor(0);
    }

    ignoreWS() {

        while (this.tokens[this.cursors.get()]?.type === uwu.WS) {
            this.cursors.next();
        }

    }

    stackeable() {

        this.cursors.create();

        const definition_token = this.tokens[this.cursors.get()];

        if (definition_token.type !== uwu.definition) {
            this.error(definition_token, this.code!).unexpected_token();
        }

        this.cursors.next();
        this.ignoreWS();

        const name_token = this.tokens[this.cursors.get()];

        if (name_token.type !== uwu.identifier) {
            this.error(name_token, this.code!).invalid_name(name_token.value);
        }

        this.cursors.next();
        this.ignoreWS();

        const colon_token = this.tokens[this.cursors.get()];

        if (colon_token.type !== uwu.colon) {
            this.error(colon_token, this.code!).unexpected_token();
        }

        this.cursors.next();
        this.ignoreWS();

        const type_token = this.tokens[this.cursors.get()];

        this.cursors.next();
        this.ignoreWS();

        const assignment_token = this.tokens[this.cursors.get()];

        this.cursors.next();
        this.ignoreWS();

        const value_token = this.tokens[this.cursors.get()];

        if (type_token.type !== uwu.type) {
            if (type_token.type === uwu.assignment) {
                this.error(type_token, this.code!).type_expected(assignment_token);
            } else if (type_token.type === uwu.identifier) {
                this.error(type_token, this.code!).type_expected(value_token);
            } else {
                this.error(type_token, this.code!).unexpected_token();
            }
        }

        if (assignment_token.type !== uwu.assignment) {
            this.error(assignment_token, this.code!).unexpected_token();
        }

        this.stack.push(name_token.value, value_token.value, 'string', definition_token.value === owo.definition_let);

        this.cursors.delete();

    }

    parser(code: string) {

        this.code = code;

        this.tokens = Array.from(this.lexer.reset(this.code));

        while (this.cursors.get() < this.tokens.length) {

            this.token = this.tokens[this.cursors.get()];

            if (!this.token) {
                this.cursors.next();
                continue;
            };

            if (!this.token.type) {
                this.error(this.token, this.code).unexpected_token();
            }

            this.ignoreWS();

            if (this.token.type === uwu.comment) {
                this.cursors.next();
                continue;
            }

            if (this.token.type === uwu.definition) {
                this.stackeable();
            }

            this.cursors.next();

        }

    }

}