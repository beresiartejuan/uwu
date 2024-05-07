import lexer from "./lexer";
import { Parser } from "./parser";

lexer.reset(`
let productos: list<string> = ["Hola", "fakwenf"];
let persona: dict<string, string> = {
    "1": "Juan",
    "2": "Alberto"
}
`);

const tokens = Array.from(lexer);

const parser = new Parser(tokens);
parser.parser();