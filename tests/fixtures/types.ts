// Type annotations test fixture

// Primitive types
let num: number = 1;
let str: string = "hello";
let bool: boolean = true;
let nul: null = null;
let undef: undefined = undefined;

// Union and intersection
type A = string | number;
type B = { x: number } & { y: number };

// Array and tuple
let arr: number[] = [1, 2, 3];
let tuple: [string, number] = ["hello", 42];

// Generic types
function identity<T>(value: T): T {
    return value;
}

// Conditional types
type IsString<T> = T extends string ? true : false;

// Mapped types
type Readonly<T> = { readonly [P in keyof T]: T[P] };

// Indexed access types
type NameType = Point["x"];

// Template literal type
type Greeting = `hello ${string}`;

// Type alias
interface Point {
    x: number;
    y: number;
}

// Optional and readonly
interface Config {
    readonly host: string;
    port?: number;
}
