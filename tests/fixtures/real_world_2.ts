// ============================================================================
// Real-world test: State management with generics and discriminated unions
// ============================================================================

// --- Event system with discriminated unions ---
type AppEvent =
    | { type: "USER_LOGIN"; payload: { userId: string; timestamp: number } }
    | { type: "USER_LOGOUT"; payload: { userId: string } }
    | { type: "DATA_LOADED"; payload: { items: unknown[]; total: number } }
    | { type: "ERROR"; payload: { message: string; code: number } }
    | { type: "NAVIGATION"; payload: { from: string; to: string } };

// --- Generic store ---
interface Store<S> {
    getState(): S;
    dispatch(event: AppEvent): void;
    subscribe(listener: (state: S) => void): () => void;
}

interface AppState {
    user: {
        id: string | null;
        isLoggedIn: boolean;
        lastLoginAt: number | null;
    };
    data: {
        items: unknown[];
        total: number;
        isLoading: boolean;
    };
    ui: {
        currentPath: string;
        previousPath: string | null;
        errors: string[];
    };
}

// --- Reducer pattern ---
type Reducer<S> = (state: S, event: AppEvent) => S;

function createStore<S>(initialState: S, reducer: Reducer<S>): Store<S> {
    let state = initialState;
    const listeners: Array<(state: S) => void> = [];

    return {
        getState() {
            return state;
        },

        dispatch(event: AppEvent) {
            state = reducer(state, event);
            for (const listener of listeners) {
                listener(state);
            }
        },

        subscribe(listener: (state: S) => void) {
            listeners.push(listener);
            return () => {
                const index = listeners.indexOf(listener);
                if (index > -1) {
                    listeners.splice(index, 1);
                }
            };
        },
    };
}

// --- Concrete reducer ---
function appReducer(state: AppState, event: AppEvent): AppState {
    switch (event.type) {
        case "USER_LOGIN":
            return {
                ...state,
                user: {
                    id: event.payload.userId,
                    isLoggedIn: true,
                    lastLoginAt: event.payload.timestamp,
                },
            };

        case "USER_LOGOUT":
            return {
                ...state,
                user: {
                    id: null,
                    isLoggedIn: false,
                    lastLoginAt: state.user.lastLoginAt,
                },
            };

        case "DATA_LOADED":
            return {
                ...state,
                data: {
                    items: event.payload.items,
                    total: event.payload.total,
                    isLoading: false,
                },
            };

        case "ERROR":
            return {
                ...state,
                ui: {
                    ...state.ui,
                    errors: [...state.ui.errors, event.payload.message],
                },
            };

        case "NAVIGATION":
            return {
                ...state,
                ui: {
                    ...state.ui,
                    currentPath: event.payload.to,
                    previousPath: event.payload.from,
                },
            };

        default:
            return state;
    }
}

// --- Type guard ---
function isErrorEvent(event: AppEvent): event is { type: "ERROR"; payload: { message: string; code: number } } {
    return event.type === "ERROR";
}

// --- Generic utility functions ---
function pick<T, K extends keyof T>(obj: T, keys: K[]): Pick<T, K> {
    const result = {} as Pick<T, K>;
    for (const key of keys) {
        result[key] = obj[key];
    }
    return result;
}

function omit<T, K extends keyof T>(obj: T, keys: K[]): Omit<T, K> {
    const result = { ...obj };
    for (const key of keys) {
        delete result[key];
    }
    return result as Omit<T, K>;
}

function groupBy<T>(items: T[], keyFn: (item: T) => string): Record<string, T[]> {
    const result: Record<string, T[]> = {};
    for (const item of items) {
        const key = keyFn(item);
        if (!result[key]) {
            result[key] = [];
        }
        result[key].push(item);
    }
    return result;
}

// --- Conditional types ---
type ExtractPayload<E extends AppEvent, T extends AppEvent["type"]> =
    E extends { type: T; payload: infer P } ? P : never;

type LoginPayload = ExtractPayload<AppEvent, "USER_LOGIN">;
type ErrorPayload = ExtractPayload<AppEvent, "ERROR">;

// --- Template literal types ---
type EventHandler<T extends string> = `on${Capitalize<T>}`;
type UpperFirst<S extends string> = S extends `${infer F}${infer R}` ? `${Uppercase<F>}${R}` : S;

// --- Mapped types ---
type EventHandlers = {
    [K in AppEvent["type"]as `handle${Capitalize<Lowercase<K>>}`]: (payload: ExtractPayload<AppEvent, K>) => void;
};

// --- as const ---
const ROUTES = {
    home: "/",
    login: "/login",
    dashboard: "/dashboard",
    profile: "/profile",
    settings: "/settings",
} as const;

type RoutePath = typeof ROUTES[keyof typeof ROUTES];

// --- Class with generics ---
class EventEmitter<Events extends Record<string, unknown[]>> {
    private handlers: Map<string, Array<(...args: unknown[]) => void>> = new Map();

    on<K extends keyof Events & string>(
        event: K,
        handler: (...args: Events[K]) => void
    ): void {
        if (!this.handlers.has(event)) {
            this.handlers.set(event, []);
        }
        this.handlers.get(event)!.push(handler as (...args: unknown[]) => void);
    }

    emit<K extends keyof Events & string>(event: K, ...args: Events[K]): void {
        const handlers = this.handlers.get(event);
        if (handlers) {
            for (const handler of handlers) {
                handler(...args);
            }
        }
    }

    off<K extends keyof Events & string>(
        event: K,
        handler: (...args: Events[K]) => void
    ): void {
        const handlers = this.handlers.get(event);
        if (handlers) {
            const index = handlers.indexOf(handler as (...args: unknown[]) => void);
            if (index > -1) {
                handlers.splice(index, 1);
            }
        }
    }
}

// --- Usage ---
const initialState: AppState = {
    user: { id: null, isLoggedIn: false, lastLoginAt: null },
    data: { items: [], total: 0, isLoading: false },
    ui: { currentPath: "/", previousPath: null, errors: [] },
};

const store = createStore(initialState, appReducer);

const unsubscribe = store.subscribe((state) => {
    console.log("State changed:", state.user.isLoggedIn);
});

store.dispatch({
    type: "USER_LOGIN",
    payload: { userId: "user-1", timestamp: Date.now() },
});

store.dispatch({
    type: "NAVIGATION",
    payload: { from: "/", to: "/dashboard" },
});

const currentState = store.getState();
console.log("User logged in:", currentState.user.isLoggedIn);
console.log("Current path:", currentState.ui.currentPath);

unsubscribe();

export { createStore, appReducer, EventEmitter, ROUTES };
export type { AppEvent, AppState, Store, Reducer, RoutePath };
