// Enums test fixture

// Numeric enum
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

// String enum
enum Color {
    Red = "RED",
    Green = "GREEN",
    Blue = "BLUE",
}

// Heterogeneous enum
enum Mixed {
    No = 0,
    Yes = "YES",
}

// Const enum
const enum HttpStatus {
    OK = 200,
    NotFound = 404,
    InternalServerError = 500,
}

// Computed enum members
enum FileAccess {
    None,
    Read = 1 << 1,
    Write = 1 << 2,
    ReadWrite = Read | Write,
}

// Using enums
function handleDirection(dir: Direction): string {
    switch (dir) {
        case Direction.Up:
            return "Going up";
        case Direction.Down:
            return "Going down";
        case Direction.Left:
            return "Going left";
        case Direction.Right:
            return "Going right";
    }
}
