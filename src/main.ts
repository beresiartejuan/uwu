import { InternalError } from "./InternalError";
import lexer from "./lexer";
import { Parser } from "./parser";
import { readFile } from "fs";

readFile("./src/example.uwu", (err, data) => {

    if (err) {
        console.error(err);
        process.exit();
    }

    let code = data.toString("utf8")

    lexer.reset(code);

    const tokens = Array.from(lexer);
    const parser = new Parser(tokens, code);

    try {
        parser.parser();
    } catch (error) {
        if (error instanceof InternalError) {
            console.error(error.toString());
        } else {
            console.error(error);
        }
    }

});