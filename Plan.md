# rscript 技术规划

> 目标: 用 Rust 忠实重构 TypeScript 编译器 (tsc)，达到与 tsgo 相当或更优的性能收益。

---

## 1. 项目定位与策略

### 1.1 对标 tsgo 的策略

tsgo (TypeScript 7) 采用了 **忠实移植 (faithful port)** 策略，将 TypeScript 的 checker.ts
逐行移植到 Go。这个策略被证明是成功的——之前 stc 项目尝试用 Rust 从零重新实现
TypeScript 类型系统，最终失败了，因为 TypeScript 类型系统的语义非常复杂且充满边界情况。

rscript 采用与 tsgo 相同的忠实移植策略，但利用 Rust 的以下优势争取更好的性能:

| 特性       | Go (tsgo)    | Rust (rscript)                      | 优势                  |
| ---------- | ------------ | ----------------------------------- | --------------------- |
| 内存分配   | GC           | Arena allocation (bumpalo)          | 零碎片，O(1) 批量释放 |
| 字符串比较 | 字符串值比较 | String interning (lasso) → 整数比较 | O(1) 等值比较         |
| 并行化     | Goroutines   | Rayon work-stealing                 | 无 GC 暂停            |
| 内存布局   | 指针间接     | 连续 arena 布局                     | 更好的 cache locality |
| 类型大小   | 接口虚表     | Enum (tagged union)                 | 无虚表开销            |

### 1.2 性能目标

| 指标       | tsc (JS) | tsgo (Go) | rscript 目标 |
| ---------- | -------- | --------- | ------------ |
| 冷启动编译 | 10s      | 1s        | < 0.8s       |
| 内存占用   | 1x       | 0.5x      | < 0.3x       |
| 增量编译   | 5s       | 0.5s      | < 0.4s       |
| LSP 启动   | 10s      | 1.2s      | < 1s         |

---

## 2. 架构设计

### 2.1 编译管线

```
Source Text (.ts/.tsx)
    │
    ▼
┌─────────────┐
│   Scanner    │  字符流 → Token 流
│  (rscript_   │  逐字符扫描，处理字符串/模板/正则/注释
│   scanner)   │
└─────┬───────┘
      │ Token stream
      ▼
┌─────────────┐
│   Parser     │  Token 流 → AST
│  (rscript_   │  递归下降，arena 分配所有节点
│   parser)    │  产出: SourceFile AST
└─────┬───────┘
      │ AST
      ▼
┌─────────────┐
│   Binder     │  AST → 符号表 + 作用域链
│  (rscript_   │  遍历 AST 建立符号关系
│   binder)    │  产出: Symbol Table, Scope Chain, Flow Nodes
└─────┬───────┘
      │ AST + Symbols
      ▼
┌─────────────┐
│   Checker    │  类型解析 + 类型检查
│  (rscript_   │  TypeTable arena 存储所有类型
│   checker)   │  结构化类型兼容性检查
│              │  产出: Diagnostics, Type information
└─────┬───────┘
      │ AST + Types
      ▼
┌─────────────┐
│ Transformers │  AST → AST 变换
│  (rscript_   │  TS 剥离、JSX 转换、降级转换
│  transformers)│
└─────┬───────┘
      │ Transformed AST
      ▼
┌─────────────┐
│   Printer    │  AST → 文本
│  (rscript_   │  格式化输出，支持 strip_types
│   printer)   │
└─────┬───────┘
      │ Text
      ▼
┌─────────────┐
│   Emitter    │  协调输出
│  (rscript_   │  .js + .d.ts + .js.map
│   emitter)   │
└─────────────┘
```

### 2.2 Crate 依赖图

```
rscript_core ◄──── rscript_diagnostics
     ▲                    ▲
     │                    │
rscript_tspath        rscript_ast ◄────────┐
     ▲                    ▲                │
     │                    │                │
     │              rscript_scanner        │
     │                    ▲                │
     │                    │                │
     │              rscript_parser         │
     │                    ▲                │
     │                    │                │
     │              rscript_binder         │
     │                    ▲                │
     │              rscript_evaluator      │
     │                    ▲                │
     │              rscript_checker ───────┘
     │                    ▲
     │                    │
     │        ┌───────────┼───────────────┐
     │        │           │               │
     │  rscript_printer  rscript_transformers  rscript_nodebuilder
     │        ▲           ▲               ▲
     │        └───────────┼───────────────┘
     │                    │
     │              rscript_emitter
     │                    ▲
     │                    │
rscript_tsoptions    rscript_sourcemap
     ▲                    ▲
     │                    │
rscript_module ───────────┘
     ▲
     │
rscript_compiler
     ▲
     ├──── rscript_ls ──── rscript_lsp
     │
rscript_cli
```

### 2.3 核心数据结构设计

#### AST 节点 (Arena 分配)

```rust
// 所有 AST 节点分配在 Bump arena 中
// 生命周期 'a 绑定到 arena
struct SourceFile<'a> {
    statements: &'a [Statement<'a>],
    // ...
}

// 节点引用通过 &'a T，零开销
enum Statement<'a> {
    VariableStatement(&'a VariableStatement<'a>),
    FunctionDeclaration(&'a FunctionDeclaration<'a>),
    // ... 40+ 变体
}
```

**设计原理**: Arena 分配确保所有 AST 节点在连续内存中，cache locality 极好。
编译完成后一次性释放整个 arena，无需逐个 drop。

#### 类型系统 (TypeId Arena)

```rust
// 类型不用生命周期，通过 TypeId 引用
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
struct TypeId(u32);

struct TypeTable {
    types: Vec<Type>,  // TypeId 是索引
    // 预分配的固有类型
    any_type: TypeId,    // TypeId(0)
    string_type: TypeId, // TypeId(2)
    // ...
}
```

**设计原理**: TypeScript 的类型系统大量使用递归类型 (如 `type A = { next: A }`)。
使用 TypeId 索引而非引用避免了 Rust 生命周期的复杂性，同时保持了 O(1) 查找。

#### 字符串驻留 (Interning)

```rust
// 所有标识符只存储一次
struct InternedString(Spur); // 4 bytes

// 比较: O(1) 整数比较，而非 O(n) 字符串比较
impl PartialEq for InternedString {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0  // 一条 CMP 指令
    }
}
```

---

## 3. 实施路线图

### Phase 0: 紧急修复 (1-2 天)

**目标**: 修复所有导致 crash/hang 的 bug，确保现有测试全部通过。

1. 修复 `generics.ts` fixture 导致的 parser 无限循环
2. 添加 parser 递归深度限制
3. 添加 template literal 循环 EOF 守卫
4. 修复 `alloc_vec_in` panic-safety
5. 添加 checker `is_type_assignable_to` 环检测
6. 添加 checker `type_to_string` 环检测
7. 修复 O(n^2) 去重为 O(n) (FxHashSet)
8. 运行全部测试验证无回归

### Phase 1: Scanner 加固 (2-3 天)

**目标**: Scanner 达到 TypeScript 官方测试套件 > 99% 的 token 扫描通过率。

补全:

- BigInt 字面量
- Unicode 转义序列完整支持
- `#private` 标识符
- Shebang 支持

测试:

- 每种 token 类型至少 3 个正面测试 + 1 个负面测试
- 边界情况: 空输入、文件结尾截断、嵌套模板字符串

### Phase 2: Parser 加固 (3-5 天)

**目标**: Parser 达到 TypeScript 官方测试套件 > 95% 的 parse 通过率。

补全:

- `satisfies` 表达式
- `using` 声明
- Comma expression
- 解析错误恢复增强
- `import type` / `export type`

测试:

- AST 结构验证 (不只是 "不 panic"，而是验证生成的 AST 结构正确)
- 运算符优先级完整验证
- 每种 TypeScript 语法结构的覆盖

### Phase 3: Binder 增强 (3-5 天)

**目标**: Binder 正确构建所有声明的符号表。

补全:

- TDZ 检测
- 函数重载合并
- 枚举成员绑定
- 类成员可见性
- `this` 绑定

测试:

- 作用域正确性: 嵌套函数、块作用域、闭包捕获
- 声明合并: 接口合并语义正确性
- 控制流图: if/else/switch 分支正确性

### Phase 4: Checker 核心实现 (2-4 周)

**目标**: 类型检查器处理 TypeScript 最常见的 80% 类型检查场景。

这是工作量最大的部分，需要分步推进:

#### 4.1 类型解析基础 (5 天)

- 类型别名实际解析 (非返回 any)
- 接口类型解析
- 类实例类型
- 枚举类型

#### 4.2 泛型与推断 (5 天)

- 泛型类型参数
- 泛型实例化 (substitution)
- 调用时的类型推断
- 上下文类型推导

#### 4.3 控制流类型缩窄 (5 天)

- typeof 缩窄
- instanceof 缩窄
- 真值缩窄
- 等值缩窄
- 判别联合类型

#### 4.4 高级类型 (5 天)

- 条件类型 + infer
- 映射类型
- 模板字面量类型
- 索引访问类型
- keyof 运算符

### Phase 5: Emit 管线 (1-2 周)

**目标**: 能正确输出 .js 和 .d.ts 文件。

1. TypeScript 剥离 transformer
2. Printer 完善 (注释、格式保持)
3. .d.ts 生成 (需要 NodeBuilder)
4. Source map VLQ 编码
5. JSX transformer

### Phase 6: 模块解析 & 多文件 (1 周)

**目标**: 支持真实项目的多文件编译。

1. Node16/NodeNext 解析
2. Bundler 解析
3. paths / baseUrl 映射
4. @types 包解析
5. 模块解析缓存

### Phase 7: LSP & 工具 (1-2 周)

**目标**: 可用的编辑器体验。

1. AST 缓存 (避免重新解析)
2. Go to Definition
3. Find References (基于符号表)
4. Document Symbols
5. 增量更新

### Phase 8: 性能优化 (1-2 周)

**目标**: 达到性能目标。

1. Rayon 并行多文件
2. 增量编译
3. Release profile 调优
4. 性能 benchmark 建立

### Phase 9: 一致性验证 (持续)

**目标**: 通过 TypeScript 官方测试套件。

1. 集成 TypeScript tests/cases/conformance
2. 逐步提升通过率
3. 错误消息一致性对比

---

## 4. TypeScript 行为调研 — 测试必须覆盖的关键语义

以下是 TypeScript 类型系统中最重要且最容易实现出错的行为,单测必须覆盖。

### 4.1 结构化类型系统 (Structural Typing)

TypeScript 使用结构化类型系统 (duck typing)，而非名义类型系统:

```typescript
// 这在 TypeScript 中合法——因为结构兼容
interface Point {
  x: number;
  y: number;
}
interface Named {
  x: number;
  y: number;
  name: string;
}

let p: Point = { x: 1, y: 2, name: "origin" } as Named; // OK: Named 有 Point 的所有属性

// 但注意: 对象字面量有 "多余属性检查"
let p2: Point = { x: 1, y: 2, name: "origin" }; // ERROR: 多余属性 'name'
```

**测试要点**: 结构兼容性、多余属性检查 (仅对字面量)、嵌套对象兼容性。

### 4.2 联合类型分配律

```typescript
// 条件类型对联合类型是分配的 (distributive)
type ToArray<T> = T extends any ? T[] : never;
type Result = ToArray<string | number>; // string[] | number[], NOT (string | number)[]

// 但用元组包裹可以禁止分配
type ToArray2<T> = [T] extends [any] ? T[] : never;
type Result2 = ToArray2<string | number>; // (string | number)[]
```

**测试要点**: 条件类型分配律、`[T] extends [any]` 禁止分配、never 在联合中消失。

### 4.3 函数类型的变型 (Variance)

```typescript
// strictFunctionTypes 下:
// 参数位置是逆变的 (contravariant)
// 返回值位置是协变的 (covariant)

type Fn1 = (x: string | number) => string;
type Fn2 = (x: string) => string | number;

let f1: Fn1 = (x: string) => x; // ERROR: 参数逆变
let f2: Fn2 = (x: string | number) => ""; // OK: 参数逆变 + 返回协变
```

**测试要点**: strictFunctionTypes on/off 的行为差异、方法参数的双变型 (bivariant)。

### 4.4 类型缩窄 (Narrowing)

```typescript
function example(x: string | number) {
  if (typeof x === "string") {
    x; // 此处类型是 string
  } else {
    x; // 此处类型是 number
  }
}

function example2(
  x: { kind: "a"; value: string } | { kind: "b"; value: number }
) {
  if (x.kind === "a") {
    x.value; // string (判别联合缩窄)
  }
}
```

**测试要点**: typeof/instanceof/in/truthiness 缩窄、判别联合、赋值缩窄、
`x != null` 排除 null/undefined。

### 4.5 泛型推断

```typescript
function identity<T>(arg: T): T {
  return arg;
}
const result = identity("hello"); // T 推断为 "hello" (字面量类型)
const result2 = identity<string>("hello"); // T 显式为 string

function map<T, U>(arr: T[], fn: (x: T) => U): U[] {
  /* ... */
}
const mapped = map([1, 2, 3], (x) => x.toString()); // T=number, U=string
```

**测试要点**: 单参数推断、多参数推断、上下文推断 (lambda 参数)、推断与约束交互。

### 4.6 `never` 类型

```typescript
// never 是 bottom type: 可赋值给任何类型
const x: never = undefined as never;
const y: string = x; // OK

// 联合中 never 消失
type T = string | never; // = string

// 交叉中 never 吸收
type U = string & never; // = never

// 函数永不返回
function fail(): never {
  throw new Error();
}
```

**测试要点**: never 的赋值规则、联合/交叉中的行为、exhaustive check。

### 4.7 字面量类型与宽化 (Widening)

```typescript
let x = "hello"; // 类型: string (宽化)
const y = "hello"; // 类型: "hello" (字面量)

const obj = { x: 1 }; // { x: number } (属性宽化)
const obj2 = { x: 1 } as const; // { readonly x: 1 } (字面量 + readonly)
```

**测试要点**: let vs const 推导差异、as const 语义、字面量到宽化类型赋值。

### 4.8 枚举

```typescript
// 数值枚举: 值可以赋给 number，number 也可以赋给枚举
enum Direction {
  Up,
  Down,
  Left,
  Right,
}
let d: Direction = 42; // OK (TypeScript 允许)

// 字符串枚举: 严格名义类型
enum Color {
  Red = "RED",
  Blue = "BLUE",
}
let c: Color = "RED"; // ERROR: 不能赋值
```

**测试要点**: 数值枚举双向赋值、字符串枚举名义性、const enum 内联、
自增值计算、计算成员。

### 4.9 声明合并

```typescript
// 接口合并
interface Box {
  width: number;
}
interface Box {
  height: number;
}
// 合并结果: interface Box { width: number; height: number; }

// 命名空间与函数/类合并
function buildLabel(name: string): string {
  return name;
}
namespace buildLabel {
  export const suffix = "!";
}
```

**测试要点**: 接口属性合并、冲突属性检测、命名空间合并、枚举与命名空间合并。

### 4.10 `this` 类型

```typescript
class Builder {
  value = 0;
  add(n: number): this {
    this.value += n;
    return this;
  }
}

class AdvancedBuilder extends Builder {
  multiply(n: number): this {
    this.value *= n;
    return this;
  }
}

// this 类型使链式调用在继承中正确工作
new AdvancedBuilder().add(1).multiply(2); // OK
```

**测试要点**: this 的多态性、继承链中的 this 类型、this 参数。

---

## 5. 已知风险与缓解

### 5.1 TypeScript checker.ts 的复杂度

TypeScript 的 `checker.ts` 有 ~50,000 行代码，是单一文件中最复杂的类型检查器之一。
完整移植需要大量工作。

**缓解**: 按功能分批移植，优先覆盖最常用的 80% 功能。

### 5.2 递归类型导致的性能问题

TypeScript 允许任意深度的递归类型，checker 需要正确处理而不会 stack overflow
或指数爆炸。

**缓解**:

- 递归深度限制 (TypeScript 自身也有 ~50 的限制)
- 类型关系 memoization 缓存
- 环检测 (visited set)

### 5.3 测试覆盖率

TypeScript 有 70,000+ 个官方测试用例。逐步达到高通过率需要持续投入。

**缓解**: 先通过 parse 阶段测试，再逐步推进 bind 和 check 通过率。
每个 phase 都设定最低通过率门槛。

---

## 6. 与 tsgo 的关键差异

| 方面     | tsgo               | rscript                  |
| -------- | ------------------ | ------------------------ |
| 语言     | Go                 | Rust                     |
| GC       | 有 (Go GC)         | 无 (arena + RAII)        |
| 并发模型 | Goroutines         | Rayon (work-stealing)    |
| 内存布局 | 指针 + GC 对象     | Arena 连续布局           |
| 字符串   | Go string          | Interned (lasso Spur)    |
| AST 表示 | Go 接口            | Rust enum (tagged union) |
| 类型表示 | Go 指针            | TypeId arena             |
| 项目状态 | 微软官方，团队开发 | 个人项目，学习 + 实践    |

---

## 附录 A: 编译与测试命令

```bash
# 完整构建
cargo build --release

# 运行所有测试 (注意: 需先修复 Phase 0 的 bug)
cargo test

# 逐模块测试 (推荐)
cargo test -p rscript_core
cargo test -p rscript_scanner
cargo test -p rscript_parser
cargo test -p rscript_binder
cargo test -p rscript_checker
cargo test -p rscript_compiler

# 运行 TypeScript 一致性测试
TS_TEST_SUITE_PATH=/path/to/TypeScript/tests/cases cargo test -p rscript_compiler -- test_conformance

# 运行性能基准
cargo bench -p rscript_compiler

# 编译器使用
cargo run -p rscript_cli -- file.ts --noEmit
cargo run -p rscript_cli -- --lsp
```

## 附录 B: 参考资源

- [TypeScript Compiler Source](https://github.com/microsoft/TypeScript/tree/main/src/compiler)
- [tsgo (TypeScript 7)](https://github.com/nicolo-ribaudo/TypeScript-Go)
- [TypeScript Specification](https://github.com/nicolo-ribaudo/TypeScript-Go)
- [stc 项目 (失败案例分析)](https://github.com/nicolo-ribaudo/TypeScript-Go)
- [bumpalo Arena Allocator](https://docs.rs/bumpalo)
- [lasso String Interning](https://docs.rs/lasso)
