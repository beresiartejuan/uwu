export class InternalError extends Error {
    constructor(message: string, line: number, column: number, code: string) {
        const firstMessage = "OnO Wow! Your code has an error, programmer-kun...";
        const separator = "-".repeat(firstMessage.length + 4);
        const secondMessage = `Huh... I think it may be... ${message}`;
        const codeLines = code.split('\n');
        const errorLine = codeLines[line - 1];
        const errorPointer = ' '.repeat(column - 1) + `^ Right here! at line ${line} uwur`;

        super();

        this.name = "Internal Error";
        this.message = `\n\n${separator}\n\n${firstMessage}\n\n${secondMessage}\n\n${errorLine}\n${errorPointer}\n\n${separator}\n\n`
    }
}