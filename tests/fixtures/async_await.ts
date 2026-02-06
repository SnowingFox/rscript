// Async/await test fixture

async function fetchData(url: string): Promise<string> {
    const response = await fetch(url);
    return response.text();
}

async function processItems<T>(
    items: T[],
    processor: (item: T) => Promise<void>,
): Promise<void> {
    for (const item of items) {
        await processor(item);
    }
}

// Async arrow function
const delay = async (ms: number): Promise<void> => {
    return new Promise((resolve) => {
        setTimeout(resolve, ms);
    });
};

// Async generator
async function* generateNumbers(): AsyncGenerator<number> {
    let i = 0;
    while (true) {
        yield i++;
        await delay(100);
    }
}

// Try/catch with async
async function safeFetch(url: string): Promise<string | null> {
    try {
        const data = await fetchData(url);
        return data;
    } catch (error) {
        console.error("Fetch failed:", error);
        return null;
    } finally {
        console.log("Fetch attempt completed");
    }
}
