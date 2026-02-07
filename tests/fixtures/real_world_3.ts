// ============================================================================
// Real-world test: Data processing pipeline with async iterators
// ============================================================================

// --- Types ---
interface DataRecord {
    id: string;
    timestamp: number;
    value: number;
    tags: string[];
    metadata: Record<string, string | number | boolean>;
}

interface AggregateResult {
    count: number;
    sum: number;
    avg: number;
    min: number;
    max: number;
    stddev: number;
}

interface TimeSeriesBucket {
    start: number;
    end: number;
    records: DataRecord[];
    aggregate: AggregateResult;
}

type SortOrder = "asc" | "desc";
type FilterPredicate<T> = (item: T) => boolean;
type MapFunction<T, U> = (item: T) => U;
type CompareFunction<T> = (a: T, b: T) => number;

// --- Pipeline builder ---
class Pipeline<T> {
    private operations: Array<(data: T[]) => T[]> = [];

    static from<T>(data: T[]): Pipeline<T> {
        const pipeline = new Pipeline<T>();
        pipeline.data = data;
        return pipeline;
    }

    private data: T[] = [];

    filter(predicate: FilterPredicate<T>): Pipeline<T> {
        this.operations.push((data) => data.filter(predicate));
        return this;
    }

    sort(compareFn?: CompareFunction<T>): Pipeline<T> {
        this.operations.push((data) => [...data].sort(compareFn));
        return this;
    }

    take(n: number): Pipeline<T> {
        this.operations.push((data) => data.slice(0, n));
        return this;
    }

    skip(n: number): Pipeline<T> {
        this.operations.push((data) => data.slice(n));
        return this;
    }

    unique(keyFn: (item: T) => string | number): Pipeline<T> {
        this.operations.push((data) => {
            const seen = new Set<string | number>();
            return data.filter((item) => {
                const key = keyFn(item);
                if (seen.has(key)) return false;
                seen.add(key);
                return true;
            });
        });
        return this;
    }

    execute(): T[] {
        let result = [...this.data];
        for (const op of this.operations) {
            result = op(result);
        }
        return result;
    }
}

// --- Statistical functions ---
function computeAggregate(values: number[]): AggregateResult {
    if (values.length === 0) {
        return { count: 0, sum: 0, avg: 0, min: 0, max: 0, stddev: 0 };
    }

    const count = values.length;
    const sum = values.reduce((a, b) => a + b, 0);
    const avg = sum / count;
    const min = Math.min(...values);
    const max = Math.max(...values);

    const squaredDiffs = values.map((v) => Math.pow(v - avg, 2));
    const variance = squaredDiffs.reduce((a, b) => a + b, 0) / count;
    const stddev = Math.sqrt(variance);

    return { count, sum, avg, min, max, stddev };
}

// --- Time series bucketing ---
function bucketByTime(
    records: DataRecord[],
    intervalMs: number
): TimeSeriesBucket[] {
    if (records.length === 0) return [];

    const sorted = [...records].sort((a, b) => a.timestamp - b.timestamp);
    const buckets: TimeSeriesBucket[] = [];
    let currentStart = sorted[0].timestamp;
    let currentBucket: DataRecord[] = [];

    for (const record of sorted) {
        while (record.timestamp >= currentStart + intervalMs) {
            if (currentBucket.length > 0) {
                const values = currentBucket.map((r) => r.value);
                buckets.push({
                    start: currentStart,
                    end: currentStart + intervalMs,
                    records: currentBucket,
                    aggregate: computeAggregate(values),
                });
            }
            currentStart += intervalMs;
            currentBucket = [];
        }
        currentBucket.push(record);
    }

    if (currentBucket.length > 0) {
        const values = currentBucket.map((r) => r.value);
        buckets.push({
            start: currentStart,
            end: currentStart + intervalMs,
            records: currentBucket,
            aggregate: computeAggregate(values),
        });
    }

    return buckets;
}

// --- Pattern matching on tagged data ---
type ProcessingResult =
    | { status: "success"; processedCount: number; duration: number }
    | { status: "partial"; processedCount: number; failedCount: number; errors: string[] }
    | { status: "failed"; error: string };

function processRecords(records: DataRecord[]): ProcessingResult {
    const startTime = Date.now();
    const errors: string[] = [];
    let processedCount = 0;

    for (const record of records) {
        try {
            if (record.value < 0) {
                throw new Error(`Negative value for record ${record.id}`);
            }
            if (!record.tags || record.tags.length === 0) {
                throw new Error(`No tags for record ${record.id}`);
            }
            processedCount++;
        } catch (e) {
            if (e instanceof Error) {
                errors.push(e.message);
            }
        }
    }

    const duration = Date.now() - startTime;

    if (errors.length === 0) {
        return { status: "success", processedCount, duration };
    } else if (processedCount > 0) {
        return {
            status: "partial",
            processedCount,
            failedCount: errors.length,
            errors,
        };
    } else {
        return { status: "failed", error: errors.join("; ") };
    }
}

// --- Decorator pattern ---
interface Logger {
    info(message: string): void;
    warn(message: string): void;
    error(message: string): void;
}

class ConsoleLogger implements Logger {
    private prefix: string;

    constructor(prefix: string = "") {
        this.prefix = prefix;
    }

    info(message: string): void {
        console.log(`[INFO] ${this.prefix}${message}`);
    }

    warn(message: string): void {
        console.warn(`[WARN] ${this.prefix}${message}`);
    }

    error(message: string): void {
        console.error(`[ERROR] ${this.prefix}${message}`);
    }
}

// --- Usage ---
function demo(): void {
    const records: DataRecord[] = [
        { id: "1", timestamp: 1000, value: 10, tags: ["a"], metadata: { source: "sensor1" } },
        { id: "2", timestamp: 1500, value: 20, tags: ["b"], metadata: { source: "sensor2" } },
        { id: "3", timestamp: 2000, value: 15, tags: ["a", "b"], metadata: { source: "sensor1" } },
        { id: "4", timestamp: 2500, value: 30, tags: ["c"], metadata: { source: "sensor3" } },
        { id: "5", timestamp: 3000, value: 25, tags: ["a"], metadata: { source: "sensor1" } },
    ];

    // Pipeline usage
    const highValues = Pipeline.from(records)
        .filter((r) => r.value > 15)
        .sort((a, b) => b.value - a.value)
        .take(3)
        .execute();

    console.log("High values:", highValues.length);

    // Time series bucketing
    const buckets = bucketByTime(records, 1000);
    for (const bucket of buckets) {
        console.log(`Bucket [${bucket.start}-${bucket.end}]: avg=${bucket.aggregate.avg}`);
    }

    // Process records
    const result = processRecords(records);
    switch (result.status) {
        case "success":
            console.log(`Processed ${result.processedCount} records in ${result.duration}ms`);
            break;
        case "partial":
            console.log(`Processed ${result.processedCount}, failed ${result.failedCount}`);
            break;
        case "failed":
            console.error(`Processing failed: ${result.error}`);
            break;
    }

    // Logger
    const logger = new ConsoleLogger("[App] ");
    logger.info("Application started");
}

export { Pipeline, computeAggregate, bucketByTime, processRecords, ConsoleLogger };
export type { DataRecord, AggregateResult, TimeSeriesBucket, ProcessingResult, Logger };
