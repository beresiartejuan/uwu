import { InternalError } from "./InternalError";
import { InvalidName } from "./InvalidName";
import { UnexpectedToken } from "./UnexpectedToken";

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

    return {
        internal_error,
        invalid_name,
        unexpected_token
    }

}