import { existsSync } from "fs";
import path from "path";
import Interpreter from "./core/Interpreter";

const filename = process.argv[process.argv.length - 1];

const real_path = path.resolve(path.normalize(filename));

const file_exist = existsSync(real_path);

if(!file_exist && path.extname(real_path) !== "uwu"){
    console.log(`The path "${real_path}" does not exist or is not a file without a uwu extension`);
}

new Interpreter(real_path);