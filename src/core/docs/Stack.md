# Stack Class

## Overview

The `Stack` class is a generic stack implementation that supports saving and restoring specific positions using marks. This is particularly useful for operations where you need to backtrack or save specific states of the stack.

## Class Definition

```typescript
export default class Stack<T> {
    internal_stack: Array<T> = [];
    marks: Map<string, number> = new Map();

    constructor(inital_stack: T[]) {
        this.internal_stack.concat(inital_stack);
    }

    push(element: T) {
        this.internal_stack.push(element);
    }

    pop(): T | undefined {
        this.marks.forEach((index, key) => {
            if (index === this.internal_stack.length - 1) {
                this.marks.delete(key);
            }
        });
        return this.internal_stack.pop();
    }

    save(mark: string) {
        this.marks.set(mark, (this.internal_stack.length > 0) ? this.internal_stack.length - 1 : this.internal_stack.length);
    }

    get(index?: number): T | undefined {
        return this.internal_stack.at(index ?? -1);
    }

    use(mark: string): boolean {
        const exists = this.marks.has(mark);

        if (exists) this.push(this.get(this.marks.get(mark))!);

        return exists;
    }
}
```

## Constructor

### `constructor(inital_stack: T[])`

- **Description**: Initializes a new instance of the `Stack` class.
- **Parameters**:
  - `inital_stack`: An optional array to initialize the stack with.

## Methods

### `push(element: T)`

- **Description**: Pushes an element onto the stack.
- **Parameters**:
  - `element`: The element to be added to the stack.

### `pop(): T | undefined`

- **Description**: Pops an element from the stack.
- **Returns**: The element removed from the stack, or `undefined` if the stack is empty.

### `save(mark: string)`

- **Description**: Saves the current stack position with a mark.
- **Parameters**:
  - `mark`: The identifier for the mark.

### `get(index?: number): T | undefined`

- **Description**: Gets the element at a specific index or the top element if no index is provided.
- **Parameters**:
  - `index` (optional): The optional index of the element to retrieve.
- **Returns**: The element at the specified index, or the top element if no index is provided.

### `use(mark: string): boolean`

- **Description**: Pushes a marked element back onto the stack.
- **Parameters**:
  - `mark`: The identifier for the mark.
- **Returns**: `True` if the mark exists and the element was pushed, `false` otherwise.

## Example Usage

```typescript
const myStack = new Stack<number>([1, 2, 3]);

myStack.push(4);
myStack.save('top4');

// Push more elements onto the stack
myStack.push(5);
myStack.push(6);
console.log(myStack.pop()); // Output: 6

// Use the "top4" mark to push the element 4 back onto the stack
myStack.use('top4');

// Pop the top element off the stack (which should be 4 now)
console.log(myStack.pop()); // Output: 4
console.log(myStack.pop()); // Output: 5
console.log(myStack.pop()); // Outputs: 4
console.log(myStack.get()); // Outputs: 3
```