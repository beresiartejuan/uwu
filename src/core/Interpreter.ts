import Stack from "./helpers/Stack";

const enum InterpreterState {

}

export default class Interpreter {

    main_file: string;
    state: Stack<InterpreterState>;

    constructor(url_file: string){
        this.main_file = url_file;

        this.state = new Stack<InterpreterState>();
    }
}