import lexer from "./lexer";
import { Parser } from "./parser";
import { readFile } from "fs";

readFile("./src/example.uwu", (err, data) => {

    if (err) {
        console.error(err);
        process.exit();
    }

    lexer.reset(data.toString("utf8"));

    const tokens = Array.from(lexer);

    const parser = new Parser(tokens);
    parser.parser();
});