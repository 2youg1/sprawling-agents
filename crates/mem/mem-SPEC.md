# mem-SPEC.md

`mem` 是 Zig 性能内核（`zig/` 一个静态库，产物 `libmem.a`／`mem.lib`）的薄 FFI 适配层。Zig 侧只存字节级叶子：字节输入 `(ptr,len)` 进、扁平结构与整数码出，无域模型、无错误文案、无 canonical 语义、无时钟调用、无回调进 Rust。领域规则、canonical 字节语义、错误三段式的文案全在 Rust。每个手写加速器三件套齐备才进主干：proptest 等价（serde 参考实现当 oracle）、fuzz 靶（`std.testing.Smith` 与 cargo fuzz 各一侧）、`zig build test` 加 ReleaseFast UB 测试。

## 1 需求分解

- 字节级热叶子的手写加速器骨架：构建链（`build.rs` 调 `zig build` 链静态库）、FFI 边界形制、验证三件套的落点。
- 首个样板件：envelope span 扫描器——为信封的五个键取值的 span，工具参数式整段借出，一次解析多用。
- 非目标：不改 `runtime::replay` 的生产探查路径（迁移另立、证一个迁一个）；不做类型解码；不做 canonical 复验。

## 2 验收标准

- `zig build test` 与 `zig build -Doptimize=ReleaseFast test` 全绿，含 `std.testing.Smith` fuzz 靶一枚。
- proptest 等价两策略（结构化生成、字节扰动）在 §3 陈述的域上与 serde 参考实现恒同解：接受面一致、五个 span 的原始文本一致。
- cargo fuzz 靶一枚（`fuzz/fuzz_targets/fuzz_envelope_span.rs`），打 Rust 面；Zig 面由 Smith 靶直打导出函数。
- Zig 只回整数码；未知码恒拒为 `E_WIRE_MISMATCH`，不猜、不兜底。内核零时钟调用。

## 3 假设与歧义

镜像口径：扫描器的接受面以 serde_json 的实际行为逐项实测定形（serde_json 是 oracle），下表是镜像结果。任何一条与 oracle 不合，改扫描器，不改口径。

| 输入性状 | 口径（serde_json 实测） |
|---|---|
| 顶层 | 恰好一个 JSON 对象；前后空白允许；尾随垃圾拒绝 |
| 字符串内字节 | 严格 UTF-8：过长编码、代理对字节、裸控制符拒绝 |
| `\uXXXX` | 四位十六进制即通过，不验代理对配对（与 `RawValue` 校验一致） |
| 数字 | RFC 8259 严格：`01` 拒，`1e999`、`-0` 收 |
| 未知键 | 跳过并完整校验其值的语法；未知名可重复（镜像 derive 跳过未知字段） |

**已知偏差一处**：嵌套深度超过 `MAX_DEPTH` 时扫描器拒绝（整数码 -3），serde 参考实现在其自身栈耗尽之前接受更深的嵌套；等价性质在深度不超过 `MAX_DEPTH` 的域上陈述。重开条件：真实信封的嵌套接近该值，届时改为迭代式跳值或上调该常量。

## 5 权威信源

- 键表拼写：`mem::envelope::EnvelopeKey::name` 与 `zig/src/lib.zig` 的键表各持一份，proptest 等价套件是两份拼写一个含义的执行者（§12-2）。
- 错误三段式与 `AxCode`：`kernel::error`。本 crate 只把整数码接到那一家上。
- 整数码表：`zig/src/lib.zig` 的 `Code` 枚举是发出侧，Rust `KernelRefusal` 是接收侧，§8-1 表是两者的对照。

## 6 命名统一

envelope、span、canonical 等概念名沿用 `docs/glossary.md` 与实现设计 §8.8 的用语；本 crate 不为既有概念另造名字。

## 7 模块边界

`mem::envelope` 是本 crate 的唯一模块；`zig/src/lib.zig` 是同一事实的字节级实现，两者由等价套件锁合。本 crate 不被任何 crate 依赖，迁移动作（未来把 `runtime::replay` 的探查换到这上面）另立变更集。

## 8 接口先行

### 8-1 mem::envelope（形状 4 adapter）

薄、无政策：换掉 Zig 内核不改任何政策——键表、span 语义、失败码的对照全在本 crate 的类型与等价套件里。

```rust
pub enum EnvelopeKey { V, Seq, Prev, Kind, Ig }   // + ALL: [EnvelopeKey; 5]、name()
pub struct EnvelopeSpans<'input>;                  // 私有字段，一个取值门
impl<'input> EnvelopeSpans<'input> {
    pub fn get(&self, key: EnvelopeKey) -> Option<&'input [u8]>;   // None = 该键不在对象里
}
pub fn scan(input: &[u8]) -> Result<EnvelopeSpans<'_>, AxError>;
```

`get` 返回的切片借自 `input`，即值的原始 JSON 文本（字符串带引号与转义，未解码）。零分配：不复制、不逃逸、不缓存。

失败码对照（整数码的唯一家在 `zig/src/lib.zig`，本表是两界的对照；映射单点在 `KernelRefusal::into_error`）：

| 整数码 | 含义 | Rust 接收 | AxCode | recovery 要点 |
|---|---|---|---|---|
| 0 | 成功 | `Ok` | — | — |
| -1 | 不是一个合规 JSON 对象 | `Malformed` | `E_INVALID_ARGS` | 改写成恰好一个 JSON 对象再送 |
| -2 | envelope 键名重复 | `DuplicateName` | `E_INVALID_ARGS` | 每个键名只留一次 |
| -3 | 嵌套深过 `MAX_DEPTH` | `TooDeep` | `E_INVALID_ARGS` | 把值拍平或拆段 |
| 其他 | 未知码 | `UnknownCode` | `E_WIRE_MISMATCH` | 从同一棵树重建 zig/ 与 crates/mem |

常量表（数字的唯一家）：

| 常量 | 值 | 家 |
|---|---|---|
| envelope 键表 | `v`, `seq`, `prev`, `kind`, `ig` | Rust `EnvelopeKey::name` 与 Zig 键表，等价套件锁合 |
| `MAX_DEPTH` | 1024 | `zig/src/lib.zig` |
| 整数码 0/-1/-2/-3 | 见上表 | `zig/src/lib.zig` 的 `Code`，Rust `KernelRefusal` 镜像 |

键重复的裁定序镜像 serde derive：语法错误（-1）优先于键重复（-2），与 serde 先解析后判重复的次序一致；重复的 envelope 键记先见的 span，但该次调用必以 -2 拒绝。

## 10 实现逻辑

扫描器一趟走完、零分配、无递归爆栈（跳值递归以 `MAX_DEPTH` 为限）：顶层对象逐成员取名（字符串含转义与严格 UTF-8 校验）、按需对五个键名做转义感知比较（`"\u0076"` 与 `v` 同名）、值取 `[起,止)` 的 span 后整段跳过（完整校验语法）。输出是扁平结构：一个 `found` 位图加五个定长槽。时间是参数的纪律在本内核以更强的形式满足：内核零时钟调用。

## 11 边界枚举

空输入、非对象顶层（`null`／数组／字面量）、截断、尾随垃圾、键名转义（`"\\u0076"`）、envelope 键重复（明写与转义各一）、未知名重复、值内非法 UTF-8、过长编码、代理对字节、`01` 与 `1e999`、`-0`、深度恰在 `MAX_DEPTH` 与其上一层。

## 12 决策（Decisions，含被击败备选）

1. **Zig 只回整数码，Rust 单点映射 `AxError`，未知码恒拒。** 理由：错误三段式的文案家留在 Rust 不动，跨界的只有可穷尽的整数。被击败：Zig 回错误字符串（把文案家推过 FFI，两个家各写各的句子）；未知码兜底成一个默认错（把失败擦成默认，正是 ClaimUnknown 同形所拒）。
2. **envelope 键表两处拼写，等价套件锁合。** 理由：固定键表让 Zig 侧用 comptime 完美哈希（`std.StaticStringMap`）并给出固定槽位的零分配输出；`export fn` 无法携带 Rust 穿举，镜像由 proptest 等价承担——这正是手写加速器三件套的本职。被击败：每次调用传键表（键名单一家在 Rust，但无法 comptime 建查找表，每次调用重建匹配，输出槽位仍需两侧约定）。
3. **扫描器只出 span，不解码值。** 理由：类型解码是域语义，留 Rust 按 span 整段借出后做；一次解析多用由此成立。被击败：扫描器直接出类型化值（把域模型推进 Zig，违反本内核的形制）。
4. **键重复的形制镜像 serde derive：五个 envelope 名重复即拒，未知名重复放行。** 理由：迁移后行为与今日 `runtime::replay` 的探查观察等价；裁定序保持"语法错误先于键重复"。被击败：一律拒重复（会拒掉 serde 今天接受的行）；后键覆盖（把一个判定悄悄换成默认值）。
5. **`crates/mem` 的 lints 表是工作区表的副本，`unsafe_code` 从 `forbid` 放宽为 `deny`。** 理由：Cargo 不允许 `[lints]` 继承与覆盖并存，而 FFI 边界必须有 `unsafe`；先例 `desktop/`（desktop-SPEC §8.5），这是全库第二个也是最后一个放宽点，每个 `unsafe` 块一条 `SAFETY:` 前置条件。被击败：把 FFI 移到 workspace 之外的第二个包（为一层薄适配把 crate 移出依赖法的管辖，得不偿失）。此放宽提交需 `Verdict:` trailer。

## 13 依赖选型

`kernel`（`AxError`／`AxCode`，文案与码表的家）。dev：`proptest`（等价性质）、`serde`＋`serde_json` 含 `raw_value` feature（参考实现取 `Box<RawValue>` 捕获值的原始文本——§8.8 的整段借出形）。构建侧 `build.rs` 用 `std::process::Command` 调 `zig build`，不引新 crate；不引 `cc`，因为要链的是 Zig 产物而不是 C 源。

## 14 硬编码声明

§8-1 的常量表是全部。键表、`MAX_DEPTH` 与整数码之外，本 crate 无第二个数字。
