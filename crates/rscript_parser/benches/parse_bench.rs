use criterion::{black_box, criterion_group, criterion_main, Criterion};
use bumpalo::Bump;
use rscript_parser::Parser;

// A medium-size TypeScript source (~100 lines) with various constructs
const TYPESCRIPT_SOURCE: &str = r#"
// TypeScript interface definitions
interface User {
    id: number;
    name: string;
    email: string;
    age?: number;
    preferences: UserPreferences;
}

interface UserPreferences {
    theme: 'light' | 'dark';
    notifications: boolean;
    language: string;
}

// Type aliases
type UserID = number;
type UserMap = Map<UserID, User>;

// Class definition
class UserService {
    private users: UserMap;
    private nextId: UserID;

    constructor() {
        this.users = new Map();
        this.nextId = 1;
    }

    createUser(name: string, email: string): User {
        const user: User = {
            id: this.nextId++,
            name,
            email,
            preferences: {
                theme: 'light',
                notifications: true,
                language: 'en'
            }
        };
        this.users.set(user.id, user);
        return user;
    }

    getUserById(id: UserID): User | undefined {
        return this.users.get(id);
    }

    updateUser(id: UserID, updates: Partial<User>): boolean {
        const user = this.users.get(id);
        if (!user) return false;
        this.users.set(id, { ...user, ...updates });
        return true;
    }

    deleteUser(id: UserID): boolean {
        return this.users.delete(id);
    }

    getAllUsers(): User[] {
        return Array.from(this.users.values());
    }
}

// Function with generics
function filterUsers<T extends User>(
    users: T[],
    predicate: (user: T) => boolean
): T[] {
    return users.filter(predicate);
}

// Async function
async function fetchUserData(id: UserID): Promise<User | null> {
    const service = new UserService();
    return service.getUserById(id) || null;
}

// Arrow function with type annotations
const processUsers = (users: User[]): number => {
    return users.reduce((count, user) => {
        if (user.age && user.age > 18) {
            return count + 1;
        }
        return count;
    }, 0);
};

// Export statements
export { User, UserService, UserPreferences };
export type { UserID, UserMap };
export default UserService;
"#;

fn bench_parse_typescript(c: &mut Criterion) {
    c.bench_function("parse_typescript_medium", |b| {
        b.iter(|| {
            let arena = Bump::new();
            let parser = Parser::new(&arena, "bench.ts", black_box(TYPESCRIPT_SOURCE));
            let source_file = parser.parse_source_file();
            black_box(source_file);
        });
    });
}

criterion_group!(benches, bench_parse_typescript);
criterion_main!(benches);
