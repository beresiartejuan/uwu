export class TypeExpected extends Error {
    constructor(infer_type: string, line: number, column: number, code: string) {
        const firstMessage = "OnO! Expected a type declaration here, programmer-kun...";
        const separator = "-".repeat(firstMessage.length + 4);
        const secondMessage = `Huh... I think it may be... ${infer_type}`;
        const codeLines = code.split('\n');
        const errorLine = codeLines[line - 1];
        const errorPointer = ' '.repeat(column - 1) + `^ Mmh... Type expected at line ${line} ( ￣^￣)`;

        super();

        this.name = "TypeExpectedError";
        this.message = `\n\n${separator}\n\n${firstMessage}\n\n${secondMessage}\n\n${errorLine}\n${errorPointer}\n\n${separator}\n\n`
    }
}
