export class TypeExpected extends Error {
    constructor(token: string, line: number, column: number, code: string) {
        const firstMessage = "OnO! Expected a type declaration here, programmer-kun...";
        const separator = "-".repeat(firstMessage.length + 4);
        const codeLines = code.split('\n');
        const errorLine = codeLines[line - 1];
        const errorPointer = ' '.repeat(column - 1) + `^ Mmh... Type expected at line ${line} before '${token}' ( ￣^￣)`;

        super();

        this.name = "TypeExpectedError";
        this.message = `\n\n${separator}\n\n${firstMessage}\n\n${errorLine}\n${errorPointer}\n\n${separator}\n\n`
    }
}
