//import { Parser } from "./core/parser";
import { readFile } from "fs";
import ParserV2 from "./core/parserV2";

readFile("./src/example.uwu", (err, data) => {

    if (err) {
        console.error(err);
        process.exit();
    }

    let code = data.toString("utf8")

    try {
        const parser = new ParserV2(code);
        console.log(parser.contexts.keys())
    } catch (error) {
        console.error(error!.toString());
    }

});