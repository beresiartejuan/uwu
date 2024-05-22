import type { Token } from "moo";
import Stack, { PrimitiveType, ComplexType } from "./stack";
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
    stack: Stack = new Stack();
    code: string = "";

    constructor() {
        this.error = handler;
        this.lexer = lexer;
        this.tokens = [];
        this.cursors = new Cursor(0);
    }

    stackeable() {

        this.cursors.create();

        const definition_token = this.tokens[this.cursors.get()];

        this.cursors.next();

        const name_token = this.tokens[this.cursors.get()];

        this.cursors.next();

        const colon_token = this.tokens[this.cursors.get()];

        this.cursors.next();

        const type_token = this.tokens[this.cursors.get()];

        if (definition_token.type !== uwu.definition) {
            this.error(definition_token, this.code).unexpected_token();
        }

        if (name_token.type !== uwu.identifier) {
            this.error(name_token, this.code).invalid_name(name_token.value);
        }

        if (colon_token.type !== uwu.colon) {
            if(colon_token.type === uwu.assignment){
                this.error(colon_token, this.code).type_expected();
            }

            this.error(colon_token, this.code).unexpected_token();
        }

        if (type_token.type !== uwu.type) {
            this.error(type_token, this.code).type_expected();
        }

        const type = type_token.value;

        switch (true) {

            case (this.stack.isPrimitive(type)): {

                this.cursors.next();
        
                const assignment_token = this.tokens[this.cursors.get()];

                this.cursors.next();

                const value_token = this.tokens[this.cursors.get()];

                if (assignment_token.type !== uwu.assignment) {
                    this.error(assignment_token, this.code!).unexpected_token();
                }

                this.stack.push(
                    name_token.value,
                    type as PrimitiveType,
                    definition_token.value === owo.definition_let,
                    value_token.value,
                );
                break;
            }

            case (this.stack.isComplex(type)): {
                
                this.cursors.next();
        
                const assignment_token = this.tokens[this.cursors.get()];

                this.cursors.next();

                const value_token = this.tokens[this.cursors.get()];

                if (assignment_token.type !== uwu.assignment) {
                    this.error(assignment_token, this.code!).unexpected_token();
                }

                this.stack.push(
                    name_token.value,
                    type as ComplexType,
                    definition_token.value === owo.definition_let
                )
                break;
            }

            default: {
                break;
            }

        }

        this.cursors.delete();

    }

    isSkipeable(token: Token){
        return token.type === uwu.comment || token.type === uwu.WS || token.type === uwu.newline;
    }

    parser(code: string) {

        this.code = code;

        this.tokens = Array.from(this.lexer.reset(this.code)).filter(
            token => token.type !== uwu.WS
        );

        while (this.cursors.get() < this.tokens.length) {

            this.token = this.tokens[this.cursors.get()];

            if (!this.token) {
                this.cursors.next();
                continue;
            };

            if (!this.token.type) {
                this.error(this.token, this.code).unexpected_token();
            }

            if (this.isSkipeable(this.token)) {
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
