export class InvalidName extends Error {
    constructor(word: string, line: number, column: number, code: string) {
        const firstMessage = "OnO Oopsie! You used a reserved token as a variable, constant, context or function name...";
        const separator = "-".repeat(firstMessage.length + 4);
        const secondMessage = `Nyaa... The word "${word}" is a reserved token and cannot be used as a name. ÒwÓ`;
        const codeLines = code.split('\n');
        const errorLine = codeLines[line - 1];
        const errorPointer = ' '.repeat(column - 1) + `^ Right here! at line ${line} uwur`;

        super();

        this.name = "Invalid Name";
        this.message = `\n\n${separator}\n\n${firstMessage}\n\n${secondMessage}\n\n${errorLine}\n${errorPointer}\n\n${separator}\n\n`
    }
}
