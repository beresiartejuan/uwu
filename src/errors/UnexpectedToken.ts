export class UnexpectedToken extends Error {
    constructor(token: string, line: number, column: number, code: string) {
        const firstMessage = "OnO Oopsie! You used an unexpected token in your code...";
        const separator = "-".repeat(firstMessage.length + 4);
        const secondMessage = `Nyaa... The token "${token}" is unexpected and cannot be used in this context.`;
        const codeLines = code.split('\n');
        const errorLine = codeLines[line - 1];
        const errorPointer = ' '.repeat(column - 1) + `^ Right here! at line ${line} uwur`;

        super();

        this.name = "Unexpected Token";
        this.message = `\n\n${separator}\n\n${firstMessage}\n\n${secondMessage}\n\n${errorLine}\n${errorPointer}\n\n${separator}\n\n`
    }
}
