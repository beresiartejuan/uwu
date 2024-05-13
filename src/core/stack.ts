import { v4 as uuidv4 } from 'uuid';

type primitiveType = 'string' | 'number' | 'boolean' | 'null';

interface StackItem {
    value: any;
    primitiveType: primitiveType;
    mutable: boolean;
}

export default class Stack {
    private items: Map<string, StackItem>;
    private cursors: Map<string, string>;

    constructor() {
        this.items = new Map();
        this.cursors = new Map();
    }

    push(name: string, value: any, primitiveType: primitiveType, mutable: boolean) {
        const id = uuidv4();
        this.items.set(id, { value, primitiveType, mutable });
        this.cursors.set(name, id);
        return id;
    }

    copy(uuid: string) {

        const id = uuidv4();
        const element = this.items.get(uuid);

        if (element) {
            this.items.set(id, element);
            return id;
        }

        return null;
    }

    get(uuid: string) {
        return this.items.get(uuid);
    }

    getCursor(name: string) {
        return this.cursors.get(name);
    }

    update(uuid: string, newValue: any) {
        const item = this.items.get(uuid);
        if (item) {
            if (item.mutable) {
                item.value = newValue;
                return true;
            } else {
                return false;
            }
        }
        return false;
    }

    pop(uuid: string) {
        return this.items.delete(uuid);
    }
}