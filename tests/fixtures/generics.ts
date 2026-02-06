// Generics test fixture

function map<T, U>(arr: T[], fn: (item: T) => U): U[] {
    const result: U[] = [];
    for (const item of arr) {
        result.push(fn(item));
    }
    return result;
}

function filter<T>(arr: T[], predicate: (item: T) => boolean): T[] {
    const result: T[] = [];
    for (const item of arr) {
        if (predicate(item)) {
            result.push(item);
        }
    }
    return result;
}

// Generic constraints
function getProperty<T, K extends keyof T>(obj: T, key: K): T[K] {
    return obj[key];
}

// Generic class
class Stack<T> {
    private items: T[] = [];

    push(item: T): void {
        this.items.push(item);
    }

    pop(): T | undefined {
        return this.items.pop();
    }

    peek(): T | undefined {
        return this.items[this.items.length - 1];
    }

    get length(): number {
        return this.items.length;
    }
}

// Generic interface
interface Repository<T> {
    findById(id: string): Promise<T | null>;
    findAll(): Promise<T[]>;
    save(entity: T): Promise<T>;
    delete(id: string): Promise<void>;
}

// Default type parameters
interface Paginated<T, M = { total: number }> {
    data: T[];
    meta: M;
}

// Conditional type with inference
type UnpackPromise<T> = T extends Promise<infer U> ? U : T;
type UnpackArray<T> = T extends Array<infer U> ? U : T;
