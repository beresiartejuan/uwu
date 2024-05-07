type DataType = "number" | "string" | "boolean" | "object" | "null" | "dict" | "list" | "function";

interface StackItem {
    value: any;
    constant: boolean;
    dataType: DataType;
}

interface Statament {
    is_mutable?: boolean,
    name?: string,
    value?: any,
    dataType?: DataType
}

export default class Stack {
    private static instance: Stack | null = null;

    private statament: Statament = {
        is_mutable: undefined,
        name: undefined,
        value: undefined,
        dataType: undefined
    };

    private items: { [key: string]: StackItem } = {};

    private constructor() { }

    static getInstance(): Stack {
        if (!Stack.instance) {
            Stack.instance = new Stack();
        }
        return Stack.instance;
    }

    define(is_mutable: boolean) {
        console.log("Se esta definiendo una variable");
        this.statament.is_mutable = is_mutable;
    }

    naming(name: string) {
        console.log("Variable con nombre ", name);
        this.statament.name = name;
    }

    push(key: string, value: any, constant: boolean, dataType: DataType) {
        this.items[key] = { value, constant, dataType };
    }

    get(key: string): any {
        return this.items[key]?.value;
    }

    isConstant(key: string): boolean {
        return !!this.items[key]?.constant;
    }

    getDataType(key: string): DataType | undefined {
        return this.items[key]?.dataType;
    }

    contains(key: string): boolean {
        return key in this.items;
    }

    pop(key: string): void {
        delete this.items[key];
    }

    getAll(): { [key: string]: StackItem } {
        return { ...this.items };
    }

    clear(): void {
        this.items = {};
    }
}