import { v4 as uuidv4 } from "uuid";
import { Token } from "moo";
import lexer from "./lexer";
import { uwu, owo } from "./rules";
import Cursor from "./cursor";
import handler from "./../errors/handler";

export default class ParserV2 {

    code: string = "";
    tokens: Token[] = []
    cursor: Cursor = new Cursor(0);
    contexts: Map<string, Token[]> = new Map();
    error: typeof handler = handler;

    constructor(code: string) {
        this.code = code;
        this.tokens = this.tokinzer(this.code);
        this.cursor.saveAs("original");
        this.register();
        if (this.contexts.has("main")) {
            this.parser(this.contexts.get("main")!);
        }
    }

    tokinzer(code: string) {
        return Array.from(lexer.reset(code)).filter(
            token => !this.isSkipeable(token)
        );
    }

    isSkipeable(token: Token) {
        return token.type === uwu.comment || token.type === uwu.WS || token.type === uwu.newline;
    }

    register() {
        this.cursor.create();
        this.cursor.saveAs("init", 0);
        this.cursor.use("init");

        let ctx_name = ""
        let is_ctx = false
        let bracket_count = 0;

        while (this.cursor.get() < this.tokens.length) {

            const token = this.tokens[this.cursor.get()];

            if (!token) {
                this.cursor.next();
                continue;
            };

            if (!token.type) {
                this.error(token, this.code).unexpected_token();
            }

            if (this.isSkipeable(token)) {
                this.cursor.next();
                continue;
            }

            if (is_ctx) {

                if (token.type === uwu.rbrace && bracket_count == 0) {
                    is_ctx = false;
                    ctx_name = "";
                    this.cursor.next();
                    continue;
                }

                if (token.type === uwu.lbrace) bracket_count++;
                if (token.type === uwu.rbrace) bracket_count--;

                this.contexts.get(ctx_name)?.push(token);

            }

            if (token.type === uwu.definition && token.value === owo.definition_context) {

                this.cursor.next();

                const new_ctx_name = this.tokens[this.cursor.get()]

                if (new_ctx_name.type !== uwu.identifier) {
                    this.error(new_ctx_name, this.code).invalid_name(new_ctx_name.value);
                }

                this.cursor.next();

                const barcket = this.tokens[this.cursor.get()];

                if (barcket.type !== uwu.lbrace) {
                    this.error(barcket, this.code).unexpected_token();
                }

                is_ctx = true;
                ctx_name = new_ctx_name.value;
                this.contexts.set(new_ctx_name.value, []);

            }

            this.cursor.next();

        }

        this.cursor.delete();

    }

    parser(context_tokens: Token[]) {

        const cursor_name = uuidv4();
        this.cursor.saveAs(cursor_name, 0);
        this.cursor.use(cursor_name);

        while (this.cursor.get() < context_tokens.length) {

            const token = context_tokens[this.cursor.get()];

            if (!token) {
                this.cursor.next();
                continue;
            };

            if (!token.type) {
                this.error(token, this.code).unexpected_token();
            }

            if (this.isSkipeable(token)) {
                this.cursor.next();
                continue;
            }

            if (token.type === uwu.definition) {
                //this.stackeable();
            }

            this.cursor.next();

        }

        this.cursor.delete();

    }

}