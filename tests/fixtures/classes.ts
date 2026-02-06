// Classes test fixture

abstract class Shape {
    abstract area(): number;

    toString(): string {
        return `Shape(area=${this.area()})`;
    }
}

class Circle extends Shape {
    constructor(private readonly radius: number) {
        super();
    }

    area(): number {
        return Math.PI * this.radius ** 2;
    }
}

class Rectangle extends Shape {
    constructor(
        private width: number,
        private height: number,
    ) {
        super();
    }

    area(): number {
        return this.width * this.height;
    }

    get perimeter(): number {
        return 2 * (this.width + this.height);
    }

    set dimensions(value: [number, number]) {
        this.width = value[0];
        this.height = value[1];
    }
}

class Container<T> {
    private items: T[] = [];

    add(item: T): void {
        this.items.push(item);
    }

    get(index: number): T {
        return this.items[index];
    }

    get size(): number {
        return this.items.length;
    }
}

// Class implements interface
interface Serializable {
    serialize(): string;
}

class User implements Serializable {
    constructor(
        public name: string,
        public email: string,
    ) {}

    serialize(): string {
        return JSON.stringify({ name: this.name, email: this.email });
    }
}
