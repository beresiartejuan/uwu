import { v4 as uuidv4 } from 'uuid';
import { owo } from './rules';

export type primitiveType = owo.type_string | owo.type_number | owo.type_boolean | owo.type_null;

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

    isPrimitive(value: string) {
        return (
            value === owo.type_string ||
            value === owo.type_number ||
            value === owo.type_boolean ||
            value === owo.type_null
        );
    }

    show() {
        const header = ["Name", "Type", "Value", "Mutable", "Cursor"];
        const rows = Array.from(this.cursors.entries()).map(([name, uuid]) => {
            const item = this.items.get(uuid)!;
            return [
                name,
                item.primitiveType,
                item.value.toString(),
                item.mutable ? "Yes" : "No",
                uuid
            ];
        });

        const colWidths = header.map((col, i) =>
            Math.max(
                col.length,
                ...rows.map(row => row[i].length + 2)
            )
        );

        const separator = colWidths.map(width => '-'.repeat(width + 2)).join('+');

        const formatRow = (row: string[]) =>
            row.map((cell, i) => ` ${cell.padEnd(colWidths[i])} `).join('|');

        console.log(separator);
        console.log(`|${formatRow(header)}|`);
        console.log(separator);
        rows.forEach(row => {
            console.log(`|${formatRow(row)}|`);
        });
        console.log(separator);
    }
}
