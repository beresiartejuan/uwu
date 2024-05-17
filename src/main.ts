import { Parser } from "./core/parser";
import { readFile } from "fs";

readFile("./src/example.uwu", (err, data) => {

    if (err) {
        console.error(err);
        process.exit();
    }

    let code = data.toString("utf8")
    const parser = new Parser();

    try {
        parser.parser(code);
        parser.stack.show();
    } catch (error) {
        console.error(error!.toString());
    }

});