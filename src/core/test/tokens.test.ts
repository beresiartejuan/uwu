import lexer from "../tokens/lexer";
import token_types from "../tokens/token_types";

describe("Lexer test", function(){

    const definition_str = "const hola: string = \"Hi! :D\";"
    const tokens = Array.from(lexer.reset(definition_str));

    it("Spaces detection", function(){
        expect(tokens.filter(
            t => t.type === token_types.WS
        )).toHaveLength(4);
    });

    it("Identifier token", function(){
        expect(tokens).toHaveLength(11);
        expect(tokens.at(2)).toHaveProperty("type", token_types.identifier);
    });

    it("String Match", function(){
        expect(tokens.at(-2)).toHaveProperty("type", token_types.string);
        expect(tokens.at(5)).toHaveProperty("type", token_types.type);
    });

    it("All Types detected", function(){
        
        const new_tokens = Array.from(lexer.reset("\"Hola a\";1223;\"223\";true;false;"));

        expect(new_tokens.at(0)).toHaveProperty("type", token_types.string);
        expect(new_tokens.at(2)).toHaveProperty("type", token_types.number);
        expect(new_tokens.at(4)).toHaveProperty("type", token_types.string);
        expect(new_tokens.at(6)).toHaveProperty("type", token_types.boolean);
        expect(new_tokens.at(8)).toHaveProperty("type", token_types.boolean);

    });

});