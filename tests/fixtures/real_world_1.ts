// ============================================================================
// Real-world test: A simple HTTP-like API service with types
// ============================================================================

// --- Type definitions ---
interface HttpResponse<T> {
    status: number;
    data: T;
    headers: Record<string, string>;
    ok: boolean;
}

interface User {
    id: number;
    name: string;
    email: string;
    role: "admin" | "user" | "guest";
    createdAt: string;
    settings?: UserSettings;
}

interface UserSettings {
    theme: "light" | "dark";
    language: string;
    notifications: boolean;
}

type ApiError = {
    code: number;
    message: string;
    details?: string[];
};

type Result<T, E = ApiError> =
    | { success: true; data: T }
    | { success: false; error: E };

// --- Utility types ---
type Nullable<T> = T | null;
type ReadonlyDeep<T> = {
    readonly [K in keyof T]: T[K] extends object ? ReadonlyDeep<T[K]> : T[K];
};

// --- Enums ---
enum HttpMethod {
    GET = "GET",
    POST = "POST",
    PUT = "PUT",
    DELETE = "DELETE",
    PATCH = "PATCH",
}

enum StatusCode {
    OK = 200,
    Created = 201,
    BadRequest = 400,
    Unauthorized = 401,
    NotFound = 404,
    InternalError = 500,
}

// --- Constants ---
const API_BASE_URL = "https://api.example.com/v1";
const DEFAULT_TIMEOUT = 30000;
const MAX_RETRIES = 3;

// --- Helper functions ---
function isSuccess<T>(result: Result<T>): result is { success: true; data: T } {
    return result.success === true;
}

function assertNonNull<T>(value: Nullable<T>, message: string = "Value is null"): T {
    if (value === null) {
        throw new Error(message);
    }
    return value;
}

function delay(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
}

// --- Class definitions ---
class ApiClient {
    private baseUrl: string;
    private timeout: number;
    private headers: Map<string, string>;

    constructor(baseUrl: string = API_BASE_URL, timeout: number = DEFAULT_TIMEOUT) {
        this.baseUrl = baseUrl;
        this.timeout = timeout;
        this.headers = new Map();
        this.headers.set("Content-Type", "application/json");
    }

    setHeader(key: string, value: string): void {
        this.headers.set(key, value);
    }

    setAuth(token: string): void {
        this.setHeader("Authorization", `Bearer ${token}`);
    }

    async get<T>(path: string): Promise<Result<T>> {
        return this.request<T>(HttpMethod.GET, path);
    }

    async post<T>(path: string, body: unknown): Promise<Result<T>> {
        return this.request<T>(HttpMethod.POST, path, body);
    }

    async put<T>(path: string, body: unknown): Promise<Result<T>> {
        return this.request<T>(HttpMethod.PUT, path, body);
    }

    async delete<T>(path: string): Promise<Result<T>> {
        return this.request<T>(HttpMethod.DELETE, path);
    }

    private async request<T>(
        method: HttpMethod,
        path: string,
        body?: unknown
    ): Promise<Result<T>> {
        const url = `${this.baseUrl}${path}`;

        for (let attempt = 0; attempt < MAX_RETRIES; attempt++) {
            try {
                const response = await this.doFetch<T>(method, url, body);
                if (response.ok) {
                    return { success: true, data: response.data };
                }
                return {
                    success: false,
                    error: {
                        code: response.status,
                        message: `HTTP ${response.status}`,
                    }
                };
            } catch (e) {
                if (attempt === MAX_RETRIES - 1) {
                    return {
                        success: false,
                        error: {
                            code: 0,
                            message: e instanceof Error ? e.message : "Unknown error",
                        }
                    };
                }
                await delay(1000 * Math.pow(2, attempt));
            }
        }

        return {
            success: false,
            error: { code: 0, message: "Max retries exceeded" },
        };
    }

    private async doFetch<T>(
        method: HttpMethod,
        url: string,
        body?: unknown
    ): Promise<HttpResponse<T>> {
        // Simulated fetch
        return {
            status: 200,
            data: {} as T,
            headers: {},
            ok: true,
        };
    }
}

// --- User service using the API client ---
class UserService {
    private client: ApiClient;

    constructor(client: ApiClient) {
        this.client = client;
    }

    async getUser(id: number): Promise<Result<User>> {
        return this.client.get<User>(`/users/${id}`);
    }

    async createUser(data: Omit<User, "id" | "createdAt">): Promise<Result<User>> {
        return this.client.post<User>("/users", data);
    }

    async updateUser(id: number, data: Partial<User>): Promise<Result<User>> {
        return this.client.put<User>(`/users/${id}`, data);
    }

    async deleteUser(id: number): Promise<Result<void>> {
        return this.client.delete<void>(`/users/${id}`);
    }

    async listUsers(filters?: {
        role?: User["role"];
        search?: string;
        page?: number;
        limit?: number;
    }): Promise<Result<User[]>> {
        const params = new URLSearchParams();
        if (filters) {
            if (filters.role) params.set("role", filters.role);
            if (filters.search) params.set("q", filters.search);
            if (filters.page) params.set("page", String(filters.page));
            if (filters.limit) params.set("limit", String(filters.limit));
        }
        const query = params.toString();
        const path = query ? `/users?${query}` : "/users";
        return this.client.get<User[]>(path);
    }
}

// --- Usage example ---
async function main(): Promise<void> {
    const client = new ApiClient();
    client.setAuth("my-token-123");

    const userService = new UserService(client);

    // Create a user
    const createResult = await userService.createUser({
        name: "John Doe",
        email: "john@example.com",
        role: "user",
    });

    if (isSuccess(createResult)) {
        console.log("Created user:", createResult.data.name);

        // Get the user
        const getResult = await userService.getUser(createResult.data.id);
        if (isSuccess(getResult)) {
            const user = assertNonNull(getResult.data);
            console.log("User email:", user.email);
        }
    } else {
        console.error("Failed:", createResult.error.message);
    }

    // List all admin users
    const admins = await userService.listUsers({ role: "admin", limit: 10 });
    if (isSuccess(admins)) {
        for (const admin of admins.data) {
            console.log(`Admin: ${admin.name} (${admin.email})`);
        }
    }
}

export { ApiClient, UserService, HttpMethod, StatusCode };
export type { HttpResponse, User, ApiError, Result };
