import { v4 as uuidv4 } from 'uuid';

type primitiveType = 'string' | 'number' | 'boolean' | 'null';

interface StackItem {
    name: string;
    value: any;
    primitiveType: primitiveType;
    mutable: boolean;
}

export default class Stack {
    private items: Map<string, StackItem>;

    constructor() {
        this.items = new Map();
    }

    push(name: string, value: any, primitiveType: primitiveType, mutable: boolean) {
        const id = uuidv4();
        this.items.set(id, { name, value, primitiveType, mutable });
        return id;
    }

    get(uuid: string) {
        return this.items.get(uuid);
    }

    findByName(name: string) {
        for (const [uuid, item] of this.items) {
            if (item.name === name) {
                return { uuid, item };
            }
        }
        return null;
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