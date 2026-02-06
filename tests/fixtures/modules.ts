// Modules test fixture

// Named exports
export const PI = 3.14159;
export function square(x: number): number {
    return x * x;
}

export interface MathResult {
    value: number;
    operation: string;
}

// Default export
export default class Calculator {
    add(a: number, b: number): number {
        return a + b;
    }

    subtract(a: number, b: number): number {
        return a - b;
    }

    multiply(a: number, b: number): number {
        return a * b;
    }

    divide(a: number, b: number): number {
        if (b === 0) throw new Error("Division by zero");
        return a / b;
    }
}

// Re-exports
export type { MathResult as Result };

// Enum export
export enum Direction {
    Up = "UP",
    Down = "DOWN",
    Left = "LEFT",
    Right = "RIGHT",
}

// Namespace export
export namespace Utils {
    export function clamp(value: number, min: number, max: number): number {
        return Math.min(Math.max(value, min), max);
    }
}
