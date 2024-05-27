import { Token } from "moo";
import lexer from "./lexer";
import { uwu } from "./rules";
import Cursor from "./cursor";

export default class ParserV2 {

    code: string = "";
    tokens: Token[] = []
    cursor: Cursor = new Cursor(0);

    constructor(code: string) {
        this.code = code;
        this.tokens = this.tokinzer(this.code);
        this.cursor.saveAs("init");
    }

    tokinzer(code: string) {
        return Array.from(lexer.reset(code)).filter(
            token => token.type !== uwu.WS
        );
    }

    register() {

    }

}