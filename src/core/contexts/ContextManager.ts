export default class ContextManager {

    static instance: ContextManager | null = null;

    private constructor(){}

    public static use(): ContextManager {
        if(this.instance == null){
            this.instance = new ContextManager();
        }

        return this.instance;
    }

    public set(){

    }

}