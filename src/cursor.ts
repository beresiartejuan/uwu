export default class Cursor {

    internal_cursors: number[] = [];

    constructor(first: number) {
        this.internal_cursors.push(first);
    }

    create() {
        this.internal_cursors.push(
            this.internal_cursors[this.internal_cursors.length - 1]
        );
    }

    key() {
        return this.internal_cursors.length - 1;
    }

    next(steps?: number) {
        if (steps) {
            this.edit(this.internal_cursors.length - 1, steps);
        } else {
            this.edit(this.internal_cursors.length - 1, 1);
        }
    }

    back(steps?: number) {
        if (steps) {
            this.edit(this.internal_cursors.length - 1, -steps);
        } else {
            this.edit(this.internal_cursors.length - 1, -1);
        }
    }

    delete() {
        this.internal_cursors.pop();
    }

    get() {
        return this.internal_cursors[this.internal_cursors.length - 1];
    }

    getRoot() {
        return this.internal_cursors[0];
    }

    edit(index: number, quantity: number) {
        if (index >= this.internal_cursors.length || index < 0) return;
        this.internal_cursors[index] = quantity + this.internal_cursors[index];
    }

}