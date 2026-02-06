// Basic TypeScript test fixture
const x: number = 42;
let greeting: string = "hello";
var flag: boolean = true;

function add(a: number, b: number): number {
    return a + b;
}

const result = add(1, 2);

interface Point {
    x: number;
    y: number;
}

type StringOrNumber = string | number;

const point: Point = { x: 10, y: 20 };
