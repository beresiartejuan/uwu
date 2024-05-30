/**
 * A generic Stack class that supports saving and restoring specific positions using marks.
 */
export default class Stack<T> {
    // Internal array to hold the stack elements
    internal_stack: Array<T> = [];

    // Map to hold marks with their corresponding indices
    marks: Map<string, number> = new Map();

    /**
     * Creates an instance of Stack.
     * @param {T[]} inital_stack - An optional array to initialize the stack with.
     */
    constructor(inital_stack: T[] = []) {
        this.internal_stack.concat(inital_stack);
    }

    /**
     * Pushes an element onto the stack.
     * @param {T} element - The element to be added to the stack.
     */
    push(element: T) {
        this.internal_stack.push(element);
    }

    /**
     * Pops an element from the stack.
     * @returns {T | undefined} - The element removed from the stack, or undefined if the stack is empty.
     */
    pop(): T | undefined {
        this.marks.forEach((index, key) => {
            if (index === this.internal_stack.length - 1) {
                this.marks.delete(key);
            }
        });
        return this.internal_stack.pop();
    }

    /**
     * Saves the current stack position with a mark.
     * @param {string} mark - The identifier for the mark.
     */
    save(mark: string) {
        this.marks.set(mark, (this.internal_stack.length > 0) ? this.internal_stack.length - 1 : this.internal_stack.length);
    }

    /**
     * Gets the element at a specific index or the top element if no index is provided.
     * @param {number} [index] - The optional index of the element to retrieve.
     * @returns {T | undefined} - The element at the specified index, or the top element if no index is provided.
     */
    get(index?: number): T | undefined {
        return this.internal_stack.at(index ?? -1);
    }

    /**
     * Pushes a marked element back onto the stack.
     * @param {string} mark - The identifier for the mark.
     * @returns {boolean} - True if the mark exists and the element was pushed, false otherwise.
     */
    use(mark: string): boolean {
        const exists = this.marks.has(mark);

        if (exists) this.push(this.get(this.marks.get(mark))!);

        return exists;
    }
}