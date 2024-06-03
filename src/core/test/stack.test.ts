import Stack from "../helpers/Stack";

describe("Stack test", function(){

    it("Methods: Pop, Push and Get", function(){

        const number_stack = new Stack<number>();

        expect(number_stack.get()).toBeUndefined();

        number_stack.push(10);
        number_stack.push(8);
        number_stack.push(3);

        expect(number_stack.pop()).toBe(3);

        expect(number_stack.get()).toBe(8);

        expect(number_stack.get(0)).toBe(10);

        const stack = new Stack<number>([1,2,3]);

        expect(stack.get()).toBe(3);

        expect(stack.pop()).toBe(3);

        expect(stack.pop()).toBe(2);

    });

});