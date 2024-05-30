import { v4 as uuidv4 } from 'uuid';
import { owo } from './../rules';

export type PrimitiveType = owo.type_string | owo.type_number | owo.type_boolean | owo.type_null;
export type ComplexType = owo.type_list;

interface Item {
    value: any;
    mutable: boolean;
}

interface PrimitiveItem extends Item {
    value: string;
    type: PrimitiveType;
}

interface ComplexItem extends Item {
    value: Map<string, string>;
    type: ComplexType;
    //    rules: Array<string>;
}

export default class Stack {
    private items: Map<string, PrimitiveItem | ComplexItem>;
    private cursors: Map<string, string>;

    constructor() {
        this.items = new Map();
        this.cursors = new Map();
    }

    push(name: string, type: ComplexType | PrimitiveType, mutable: boolean, value?: string | number | boolean | null) {

        const id = uuidv4();

        if (this.isPrimitive(type)) {

            const item: PrimitiveItem = {
                type: (type as PrimitiveType),
                mutable,
                value: (value ? value.toString() : "")
            };

            this.items.set(id, item);
            this.cursors.set(name, id);
            return id;

        }

        if (this.isComplex(type)) {

            const item: ComplexItem = {
                type: (type as ComplexType),
                mutable,
                value: new Map()
            }

            this.items.set(id, item);
            this.cursors.set(name, id);
            return id;

        }

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

    isComplex(value: string) {
        return (
            value === owo.type_dict ||
            value === owo.type_list
        );
    }

    show() {
        const header = ["Name", "Type", "Value", "Mutable", "Cursor"];
        const rows = Array.from(this.cursors.entries()).map(([name, uuid]) => {
            const item = this.items.get(uuid)!;

            if (this.isPrimitive(item.type)) {
                return [
                    name,
                    item.type,
                    item.value.toString(),
                    item.mutable ? "Yes" : "No",
                    uuid
                ];
            } else {
                return [
                    name,
                    item.type,
                    "-",
                    item.mutable ? "Yes" : "No",
                    uuid
                ];
            }
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
