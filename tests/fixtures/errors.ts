// Error cases test fixture
// This file contains intentional type errors for diagnostic testing

// Type mismatch
const x: number = "hello";

// Missing property
interface Required {
    name: string;
    age: number;
}

const obj: Required = { name: "test" };

// Duplicate identifier
const duplicate = 1;
const duplicate = 2;

// Using undeclared variable
console.log(undeclaredVariable);
