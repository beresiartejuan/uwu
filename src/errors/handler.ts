import { InternalError } from "./InternalError";
import { InvalidName } from "./InvalidName";
import { UnexpectedToken } from "./UnexpectedToken";
import { TypeExpected } from "./TypeExpected";
import { owo, uwu } from "src/core/rules";

export default function handler(token: moo.Token, code: string) {

    function internal_error(message: string) {
        throw new InternalError(message, token.line, token.col, code);
    }

    function invalid_name(name: string) {
        throw new InvalidName(name, token.line, token.col, code);
    }

    function unexpected_token() {
        throw new UnexpectedToken(token.value, token.line, token.col, code);
    }

    function type_expected(token_value: moo.Token) {

        throw new TypeExpected(token_value.type!, token.line, token.col, code);
    }

    return {
        internal_error,
        invalid_name,
        unexpected_token,
        type_expected
    }

}