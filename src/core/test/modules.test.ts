import TokenizerModule from "../modules/TokenizerModule";

describe("Module test", function(){

    it("Tokenizer Module", function(){
        
        expect(TokenizerModule).toHaveProperty("use");

        const t = TokenizerModule.use("use hola;");

        expect(t).toHaveLength(4);

        expect(t[0]).toHaveProperty("type");

    });

});