import lexer from "../tokens/lexer";

export default class TokenizerModule {

    private static lexer: typeof lexer = lexer;

    private constructor(){};

    public static use(code: string){

        return Array.from(this.lexer.reset(code));

    }
}