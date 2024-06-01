import lexer from "../tokens/lexer";
import token_types from "../tokens/token_types";

describe("Lexer test", function(){

    it("Identifier token", function(){

        const definition_str = "const hola: string = \"Hi! :D\""

        const tokens = Array.from(lexer.reset(definition_str));

        expect(tokens.length).toBe(10);

    });

});