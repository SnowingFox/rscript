# rscript - TODO Tracker

> 用 Rust 重构 TypeScript 编译器，目标达到 tsgo 级别的性能收益。
> 每完成一项在对应条目前标记 `[x]`。

---

## Phase 0: 紧急修复 — CPU/内存安全问题

这些问题是之前对话导致电脑死机的根因，必须最先修复。

### 0.1 Parser 无限循环

- [ ] **P0-BUG: `generics.ts` fixture 导致 parser 死循环**

  - 复现: `cargo test -p rscript_parser -- test_parse_generics_fixture`
  - 原因: 解析 `keyof T`、`T[K]` 索引访问类型或 `Promise<infer U>` 时无限循环
  - 修复: 定位死循环的具体 parse 函数，添加 token 前进保护

- [ ] **P0-SAFETY: Parser 无递归深度限制**

  - `parse_type()` → `parse_union_or_intersection_type()` → `parse_postfix_type()` 等链路无深度限制
  - 深度嵌套的 `(((((...))))))` 或 `A extends B ? C extends D ? ...` 会 stack overflow
  - 修复: 添加 `recursion_depth: u32` 计数器，阈值 1000 时报错退出

- [ ] **P0-SAFETY: Template literal 循环缺少 EOF 守卫**

  - `parser.rs` L2028 和 L2737 的 `loop {}` 依赖 `is_tail` 标记退出
  - 如果 template 解析异常导致非 `TemplateTail`，则无限循环
  - 修复: 在 break 条件中添加 `|| self.current_token() == SyntaxKind::EndOfFileToken`

- [ ] **P1-SAFETY: `alloc_vec_in` 不是 panic-safe 的**
  - `parser.rs` L18-27 使用 `std::ptr::read` + `set_len(0)`
  - 如果 `alloc_slice_fill_with` 的回调 panic，会导致 double-free
  - 修复: 使用 `ManuallyDrop<Vec<T>>` 或 `Vec::into_boxed_slice()` + `Box::leak()`

### 0.2 Checker 无限递归 & 指数复杂度

- [ ] **P0-BUG: `is_type_assignable_to` 无环检测**

  - L1653: 对 union/intersection 递归时无 visited set
  - 循环类型如 `type A = B; type B = A;` 导致无限递归 → stack overflow
  - 修复: 添加 `&mut HashSet<(TypeId, TypeId)>` 参数或内部 visited 缓存

- [ ] **P0-BUG: `type_to_string` 无环检测**

  - L1587: 递归格式化 union/intersection/object type 时无 visited set
  - 循环类型导致无限递归
  - 修复: 添加 `&mut HashSet<TypeId>` visited 参数

- [ ] **P1-PERF: `create_union_type` / `create_intersection_type` O(n²) 去重**

  - L1534-1560: 使用 `Vec::contains()` 进行去重，每个元素 O(n)
  - 修复: 使用 `FxHashSet<TypeId>` 或 `IndexSet`

- [ ] **P1-PERF: `is_type_assignable_to` 指数级复杂度**

  - union-of-unions 产生指数级路径爆炸
  - 修复: 添加 memoization cache `HashMap<(TypeId, TypeId), bool>`

- [ ] **P2-PERF: 结构化类型检查 O(n\*m) 属性查找**

  - L1703-1716: `source_members.iter().find()` 对每个 target member 都线性搜索
  - 修复: 将 source_members 预建为 HashMap

- [ ] **P2-PERF: `check_property_access` O(n) 属性查找**

  - L941: 对 members Vec 线性搜索
  - 修复: ObjectType members 改为 IndexMap 或 HashMap

- [ ] **P2-PERF: `check_call_expression`/`check_new_expression` 不必要的 clone**
  - L806, L884: `call_signatures.clone()` / `construct_signatures.clone()` 拷贝整个 Vec
  - 修复: 重构借用关系，避免 clone

### 0.3 Binder 安全性

- [ ] **P2-SAFETY: scope chain 遍历无环检测**
  - `resolve_symbol()` L858 和 `resolve_name()` L871 沿 parent chain 遍历
  - 如果 scope tree 构建出错形成环，则无限循环
  - 修复: 添加深度限制或 visited set

### 0.4 LSP 性能

- [ ] **P2-PERF: Language Service 每次请求重新解析**
  - `get_diagnostics()` 每次调用都创建新的 `Bump` arena 并重新 parse/bind/check
  - 修复: 按 document URI + version 缓存已解析的 AST

---

## Phase 1: Scanner 完善 & 测试

### 1.1 功能完善

- [x] 基础 token 扫描 (标识符、数字、字符串、运算符)
- [x] 关键字识别
- [x] 模板字符串扫描 (`TemplateHead`, `TemplateMiddle`, `TemplateTail`)
- [x] 正则表达式扫描 (`rescan_slash_token`)
- [x] JSX 文本扫描
- [x] 数字分隔符 (`1_000_000`)
- [x] 二进制/八进制/十六进制字面量
- [x] Unicode 标识符支持 (`unicode-xid`)
- [ ] BigInt 字面量 (`100n`)
- [ ] Unicode 转义序列完整支持 (`\u{XXXXX}`)
- [ ] JSX 完整扫描 (自闭合标签 `<br/>`, 属性等)
- [ ] `#private` 标识符扫描
- [ ] Shebang (`#!/usr/bin/env node`) 支持

### 1.2 单测 (TypeScript 行为一致性)

- [x] 基础 token 扫描测试 (22 个)
- [ ] 所有运算符 token 覆盖测试
- [ ] 字符串字面量: 单引号、双引号、转义序列、未闭合检测
- [ ] 模板字面量: 嵌套模板、多行模板、标签模板
- [ ] 数字字面量: 整数、浮点、科学计数法、进制、分隔符、BigInt
- [ ] 正则表达式: 基本模式、标志、字符类、转义
- [ ] 注释: 单行、多行、嵌套、JSDoc
- [ ] Unicode: BMP 标识符、补充平面、零宽字符
- [ ] 边界情况: 空输入、只有空白、未终止字符串/注释

---

## Phase 2: Parser 完善 & 测试

### 2.1 功能完善

- [x] 语句解析 (变量声明、函数、类、if/else、for/while/do、switch、try/catch)
- [x] 表达式解析 (二元、一元、条件、赋值、逗号、对象/数组字面量)
- [x] 类型注解解析 (基本类型、union、intersection、函数类型、数组类型、元组)
- [x] 泛型声明和类型参数
- [x] 装饰器解析
- [x] async/await 解析
- [x] 解构赋值 (对象、数组、嵌套)
- [x] 模块声明 (import/export)
- [x] 枚举声明
- [x] 命名空间/module 声明
- [ ] **可选链 (`?.`) 完整处理** — 需验证 AST 节点正确性
- [ ] **Nullish coalescing (`??`)** — 需验证优先级
- [ ] **`satisfies` 表达式** (TS 4.9+)
- [ ] **`using` 声明** (TS 5.2+)
- [ ] **`import type` / `export type`** — 类型导入导出的 AST 区分
- [ ] **Comma expression** (`a, b, c`) — 当前标注为 TODO
- [ ] **解析错误恢复** — 更好的错误恢复以避免级联错误

### 2.2 单测 (TypeScript 行为一致性)

- [x] 基础语句解析测试 (67 个)
- [ ] 每种 statement 类型的 AST 结构验证
- [ ] 每种 expression 类型的 AST 结构验证
- [ ] 运算符优先级完整测试 (按 TypeScript 优先级表)
- [ ] 类型注解: 联合、交叉、条件、映射、模板字面量类型
- [ ] 泛型: 约束、默认值、多参数、嵌套泛型
- [ ] 类成员: 修饰符组合 (public/private/protected/static/readonly/abstract)
- [ ] 模块: 各种 import/export 语法变体
- [ ] 错误恢复: 缺少分号、括号不匹配等
- [ ] ASI (Automatic Semicolon Insertion) 行为验证

---

## Phase 3: Binder 完善 & 测试

### 3.1 功能完善

- [x] 基础符号表构建
- [x] 作用域链 (函数作用域、块作用域)
- [x] 变量声明提升 (var hoisting)
- [x] 函数声明提升
- [x] 符号解析 (scope chain traversal)
- [x] 控制流图构建 (基础)
- [x] 接口/命名空间声明合并 (基础)
- [ ] **`let`/`const` 的 TDZ (Temporal Dead Zone) 检测**
- [ ] **块级作用域正确性** — `for` 循环每次迭代的作用域
- [ ] **函数重载声明合并**
- [ ] **枚举成员符号绑定**
- [ ] **类成员可见性追踪 (public/private/protected)**
- [ ] **命名空间导出成员追踪**
- [ ] **`this` 类型绑定** — 类/方法中的 this 上下文
- [ ] **全局声明 (lib.d.ts)** — 内建类型/对象绑定

### 3.2 单测 (TypeScript 行为一致性)

- [x] 基础符号绑定测试 (20 个)
- [ ] 作用域: 函数作用域 vs 块作用域 (var vs let/const)
- [ ] 提升: var 提升到函数顶部、函数声明提升
- [ ] 遮蔽: 内部作用域同名变量遮蔽外部
- [ ] 声明合并: 接口合并、命名空间合并、枚举合并
- [ ] 重复声明检测: `let` 重复声明报错
- [ ] TDZ: 在声明前引用 `let`/`const` 报错
- [ ] 控制流: if/else、switch、try/catch 的流图正确性

---

## Phase 4: Checker (类型检查器) 完善 & 测试

### 4.1 类型解析

- [x] 基本类型解析 (string, number, boolean, void, null, undefined, never, any, unknown)
- [x] 联合类型解析 (`A | B`)
- [x] 交叉类型解析 (`A & B`)
- [x] 数组类型解析 (`T[]`, `Array<T>`)
- [x] 元组类型解析 (`[A, B, C]`)
- [x] 函数类型解析 (`(a: A) => B`)
- [x] 对象字面量类型解析
- [x] **类型别名解析** — 已实现 `check_type_alias_declaration`，解析底层类型并注册
- [x] **接口类型解析** — 已实现 `check_interface_declaration`，构建 ObjectType (属性、方法、索引签名、调用签名、声明合并、extends 继承)
- [ ] **类实例类型解析** — 当前返回 `any`
- [ ] **泛型类型实例化** — `Array<string>` → 具体化的数组类型
- [ ] **条件类型求值** — `T extends U ? X : Y` 的实际计算
- [ ] **映射类型求值** — `{ [K in keyof T]: T[K] }` 的实际计算
- [ ] **模板字面量类型** — 当前简化为 string
- [ ] **索引访问类型** — `T[K]` 的实际解析
- [ ] **`typeof` 类型查询** — 当前返回 `any`
- [ ] **`keyof` 运算符** — 键类型提取
- [ ] **工具类型 (Utility Types)** — Partial, Required, Pick, Omit 等 (当前全返回 any)

### 4.2 类型检查

- [x] 基本赋值兼容性检查
- [x] 联合类型赋值检查
- [x] 结构化类型兼容性 (duck typing)
- [x] 函数调用参数检查
- [x] 函数重载解析 (基础)
- [x] `new` 表达式检查
- [x] 属性访问检查
- [x] 索引访问检查
- [x] 未声明变量检查
- [x] `strictNullChecks` 支持
- [ ] **类型推导 (Type Inference)** — 变量初始化推导、函数返回值推导
- [ ] **上下文类型 (Contextual Typing)** — lambda 参数类型推导
- [ ] **类型缩窄 (Type Narrowing)** — typeof, instanceof, in, 真值检查, 等值检查
- [ ] **控制流分析 (CFA)** — 确定性赋值分析、可达性分析
- [ ] **泛型推断** — 调用泛型函数时的类型参数推断
- [ ] **字面量类型** — const 推导为字面量类型而非宽化类型
- [ ] **判别联合类型 (Discriminated Unions)** — tag 字段缩窄
- [ ] **类型守卫 (Type Guards)** — `is` 返回类型、自定义类型守卫
- [ ] **严格模式全家族** — strictFunctionTypes, strictBindCallApply 等
- [ ] **`as const` 断言** — 深度 readonly + 字面量类型
- [ ] **`satisfies` 运算符检查**

### 4.3 单测 (TypeScript 行为一致性)

- [x] 基本赋值兼容性测试 (85 个，含类型别名、接口、函数调用、控制流、压力测试)
- [ ] 结构化类型: 多余属性检查、嵌套对象兼容性
- [ ] 联合/交叉: 分配律 (`(A | B) & C = (A & C) | (B & C)`)
- [ ] 函数类型: 参数逆变、返回值协变
- [ ] 泛型: 约束检查、实例化、推断
- [ ] 条件类型: 分配条件类型、infer 推断
- [ ] 字面量类型: 字面量到宽化类型的赋值、反向不可赋值
- [ ] 枚举: 数值枚举赋值、字符串枚举不可互换
- [ ] 类: 私有成员兼容性 (名义类型)、继承兼容性
- [ ] 元组: 固定长度检查、可选元素、rest 元素
- [ ] 循环类型: 不会导致无限递归的正确处理

---

## Phase 5: Printer/Emitter 完善 & 测试

### 5.1 Printer

- [x] 基础 AST 到文本输出
- [x] 类型注解打印
- [x] 缩进和格式化
- [x] strip_types 模式 (去除类型注解输出 JS)
- [ ] **模板字符串打印** — 当前 head text 为空 (L900)
- [ ] **字符串字面量属性名** — 当前返回空字符串 (L1428)
- [ ] **装饰器打印完善**
- [ ] **注释保留和输出**
- [ ] **JSX 打印**
- [ ] **保持原始格式** — 尽量保持源码格式

### 5.2 Emitter

- [x] JS 输出协调
- [x] .d.ts 输出框架
- [x] Source map 输出框架
- [x] 输出路径计算 (outDir 支持)
- [x] 文件写入
- [ ] **正确的 .d.ts 生成** — 需要 NodeBuilder 支持
- [ ] **Source map 实际映射** — 当前是空 mappings 的占位符

### 5.3 Transformers

- [ ] **TypeScript 剥离 transformer** — 去除类型注解、enum 转换
- [ ] **JSX transformer** — JSX → React.createElement / jsx 函数
- [ ] **Decorator transformer** — 旧版装饰器转换
- [ ] **ES 降级 transformer** — async/await → generator 等

### 5.4 单测

- [x] Emitter 基础测试 (4 个)
- [ ] Printer: 每种 AST 节点的输出正确性
- [ ] Emitter: JS 输出 round-trip 验证 (parse → print → reparse 一致)
- [ ] strip_types: 类型注解完全去除验证
- [ ] .d.ts: 声明文件输出正确性
- [ ] Source map: VLQ 编码正确性、位置映射验证

---

## Phase 6: 模块解析 & 测试

### 6.1 功能完善

- [x] Node10 模块解析 (基础)
- [x] 文件扩展名探测 (.ts, .tsx, .d.ts, .js, .jsx)
- [x] package.json 解析 (基础)
- [x] node_modules 搜索
- [ ] **Node16/NodeNext 解析** — 当前回退到 Node10
- [ ] **Bundler 解析** — 当前回退到 Node10
- [ ] **package.json `exports` 字段** — 条件导出、子路径导出
- [ ] **package.json `imports` 字段** — 自引用导入
- [ ] **paths mapping** — tsconfig paths 别名解析
- [ ] **baseUrl 解析**
- [ ] **rootDirs 虚拟目录**
- [ ] **typeRoots / @types 解析**
- [ ] **模块解析缓存** — 避免重复文件系统 I/O

### 6.2 单测

- [ ] Node10: 相对路径、node_modules、index.ts、package.json main
- [ ] Node16: ESM 与 CJS 区分、.mts/.cts 扩展名
- [ ] Bundler: 与 webpack/vite 兼容的解析行为
- [ ] paths: 通配符匹配、多路径回退
- [ ] 错误情况: 找不到模块、循环依赖

---

## Phase 7: 基础设施 & 工具

### 7.1 Core

- [x] Arena 分配器 (bumpalo)
- [x] 字符串驻留 (lasso)
- [x] LineMap (行列号计算)
- [x] OrderedMap
- [ ] **UTF-16 代码单元计算** — 当前 LineMap 使用字节偏移量，TypeScript 用 UTF-16
- [ ] **Arena 安全性审查** — `alloc_vec_in` panic-safety

### 7.2 Diagnostics

- [x] 诊断消息框架
- [x] 大量诊断消息定义 (约 1700+ 条)
- [ ] **诊断消息参数格式化** — 正确插入 `{0}`, `{1}` 占位符
- [ ] **错误位置精确化** — 附加 span 信息到每条诊断

### 7.3 CLI

- [x] 基础 CLI (clap)
- [x] 文件编译
- [x] --noEmit 支持
- [x] --project/-p tsconfig 支持
- [x] --lsp 启动 LSP
- [ ] **Watch 模式** (notify crate 已引入但未实现)
- [ ] **--version 输出**
- [ ] **glob 文件匹配** (src/\*_/_.ts)
- [ ] **错误退出码** — 有类型错误时 exit 1

### 7.4 LSP

- [x] TextDocumentSync (打开/修改/关闭)
- [x] Diagnostics 推送
- [x] 补全 (关键字补全)
- [x] Hover (关键字信息)
- [ ] **Go to Definition** — 当前返回空
- [ ] **Find References** — 当前是朴素文本搜索
- [ ] **Document Symbols** — 当前使用占位符名称
- [ ] **AST 缓存** — 每次请求避免重新解析
- [ ] **增量更新** — 仅重新检查变更部分

### 7.5 Evaluator

- [ ] **常量表达式求值** — 枚举成员值计算
- [ ] **数值常量折叠** — 1 + 2 → 3
- [ ] **字符串常量折叠**

### 7.6 Source Map

- [ ] **VLQ 编码实现** — Base64 VLQ 编码/解码
- [ ] **映射收集** — printer 输出时记录位置映射
- [ ] **V3 source map JSON 输出**

### 7.7 NodeBuilder

- [ ] **合成 AST 节点构建** — 用于错误消息中的类型显示
- [ ] **.d.ts 声明生成** — 从类型信息生成声明节点

---

## Phase 8: 性能优化

- [ ] **并行编译 (rayon)** — 多文件并行 parse/bind/check
- [ ] **增量编译** — 只重新编译变更的文件
- [ ] **按需类型检查** — 惰性求值类型，避免检查未引用的代码
- [ ] **类型缓存** — 避免重复计算相同类型的属性
- [ ] **Release profile 优化** — LTO, codegen-units=1, strip
- [ ] **性能基准测试** — criterion benchmarks 覆盖关键路径

---

## Phase 9: 一致性测试

- [x] 内置一致性样本 (6 个测试)
- [ ] **TypeScript 官方测试套件集成** — 70K+ 测试用例
- [ ] **Parse 通过率 > 95%** — 当前未测量
- [ ] **Bind 通过率 > 90%**
- [ ] **Check 通过率 > 70%** (初始目标)
- [ ] **Check 通过率 > 90%** (中期目标)
- [ ] **Check 通过率 > 99%** (最终目标)
- [ ] **错误消息一致性** — 与 tsc 输出对比验证
