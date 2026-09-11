# eval-SPEC.md

> crate：`eval`。本 SPEC 先于代码存在；实现不多不少地遵守本文。
> 骨架：apostle-sdd 十七节；按模块分章、每章自足（ARCHITECTURE.md §5）。
> 动手前先读所用工具与依赖的**官方文档或官方 agent 指南**，再写本文的接口节。

## 1 需求分解

本 crate 是「城拿证据评估自己」的统计层。它**恒不是合并门**：EVAL 说一件事变差了，是给人与 UD 准入看的证据，不是 CI 的红灯。五个模块分三组落地，每组落地时先补齐本文对应章节（接口先行）：

| 模块 | 它回答的问题 |
|---|---|
| `suite`（含 `holdout` 判定） | 一批真实任务怎么组织、怎么跑两次而结果可比 |
| `probe` | 一个机制怎么被跨版本地测，而测量本身不改动被测对象 |
| `score`、`metabolism` | 哪些沉淀资产在升值、哪些该退场 |

**`holdout` 合并进 `suite`**。它只有一个消费者、没有自有状态——双集是 suite 的性质，第二个模块就是第二个「这道题在哪一半」的问法。模块表删行携 `Verdict:` 尾注。

## 2 验收标准

验收标准写在 ARCHITECTURE.md §10 的收口栏，本文在模块落地时把它展开成断言名。**本节现在空着是事实而非疏漏**：一个未施工模块的验收标准写在接口存在之前，只会在施工时被改掉。

## 3 假设与歧义

三条前提，本 crate 只消费不重议：

- **评分对象是资产不是 Agent**：会话冻结即终结，一个 Run 冻结后没有「它」可供记挂；Ephemeral 恒不进评分与 metabolism。
- **语料只取自真实工作**：不造训练数据、不做合成任务。一份合成任务集测出来的分数，测的是出题人。
- **登记归 `kernel::registry`**：Asset 是什么、登记在哪由它答；本 crate 只答「这份登记值多少」。

## 4 现状分析

`eval` 自 S0 起是空壳（只有 `lib.rs` 的 crate 文档）。它要消费的两件已建已测：`kernel::registry`（Registry 三本）与 `memory::attribution`（五维成本归因，`by_skill` 的真值随 P3 Library 才出现）。P1 期「Handoff probe 套件成形」未落地——`probe` 的首个对象仍是 Handoff，验收仍是「冻结前问卷 vs 恢复后问卷」两相对照。

## 5 权威信源

「成本与自迭代」的语义（自迭代材料只来自真实工作、UD 双验证准入、没有代谢的积累是负债、RSI 的诚实边界）与「判负条件」第 2 条（规模不划算——EVAL 是评估它的仪器，故 EVAL 自己的读数恒不可由被评估方提供）；ARCHITECTURE.md §12 模块图的 eval 段与 §9 七形状；`kernel-SPEC.md` 的 registry 章。

## 6 命名统一

**跨 crate 类型住处**：`kernel` 的门／计划／脊／事件／错误／弃置／秘密七面已切目录，`cargo public-api` 基线记其定义位簇路径（如 `error::shape::AxError`）；本 crate 经 `kernel` 顶层重导出引用，公共拼写不变，住处是 kernel 内政。

Suite｜held-in／held-out｜probe｜Asset｜asset scoring｜metabolism｜debt scan｜weakness clustering｜floor-area ratio｜promotion。概念名一律英文原词；该用什么词见 `docs/glossary.md`，不该用什么词见 `xtask/lexicon.toml`。

## 7 模块边界

**三件邻居的活，及它们各自的主人**（写「X 归 Y」而非「不做 X」：前者告诉施工者去哪，后者只告诉他别去哪里）：

- **性能与尺寸预算归 `xtask budget`**：设阈值的门在那里，依据是「机器两次量得一样」。本 crate 量的是模型行为，两次不一样是常态，故它出证据不出红灯。
- **成本读数归 `memory::attribution`**：五维摊回与权威计费额对账已在那里，`score` 消费它的输出，恒不另算一份钱。
- **资产的存在与登记归 `kernel::registry`**：本 crate 不建第二本资产簿；`metabolism` 的撤销动作也经由 registry 的登记面表达。

## 8 接口先行

### 8-1 eval::suite（形状 2 值类型＋形状 1 判定）

```rust
pub enum Half { HeldIn, HeldOut }
pub struct Task { pub id: String, pub at: Locator, pub half: Half }
pub struct Outcome { pub id: String, pub passed: bool }
pub struct Tally { pub tried: u32, pub passed: u32 }   // per_mille() 整数千分比
pub struct Report { pub held_in: Tally, pub held_out: Tally, pub unknown: u32 }
pub struct Suite { /* BTreeMap<String, Task> —— 私有 */ }
impl Suite {
    pub fn new(tasks: Vec<Task>) -> Result<Suite, AxError>;   // 同一 id 两次即拒（泄漏在构造点）
    pub fn half(&self, half: Half) -> Vec<&Task>;             // id 序＝执行序
    pub fn report(&self, outcomes: &[Outcome]) -> Report;
}
```

- **泄漏是构造点的拒绝，不是事后的告警**：同一个 id 出现两次即拒，无论落在同半还是异半。一份被看过的 held-out 集在它被看过之后就不值钱了，而那时再报警已经晚了。
- **任务只携 Locator 不携正文**：语料取自真实任务，抄一份正文进来就会与它来自的那件活漂开。
- **千分比整数**：判定路径禁浮点，且「两次量得一样」是这套东西存在的前提。
- **不认识的 outcome 计入 `unknown` 而非计入分母**：一次回答了没人问过的问题的运行，不是这份 suite 的运行。

### 8-2 eval::probe（形状 2 值类型）

```rust
pub struct ProbeId { pub name: String, pub version: u32 }
pub struct Probe { /* id、questions —— 私有 */ }
impl Probe {
    pub fn new(id: ProbeId, questions: Vec<String>) -> Result<Probe, AxError>;
    pub fn answered(&self, answers: Vec<String>) -> Result<Answers, AxError>;  // 数目对不上即拒
}
pub struct Comparison { pub kept: u32, pub lost: Vec<u32> }
pub fn compare(before: &Answers, after: &Answers) -> Result<Comparison, AxError>;
```

- **跨版本比较恒拒**：问题改过的探针是另一件仪器，混算测的是仪器不是被测物。
- **报位置不报分数**：`lost` 是问题的序号，人自己去读那两个答案——一个摘要在这里正好会掩盖它要报告的那类损失。
- **探针不去采集**：问问题的是一个 Run，本 crate 恒不在它所测量的那条回路里。

### 8-3 eval::score（形状 1 判定）

```rust
pub struct AssetUse { pub uses: u32, pub resident: ByteLen, pub billed: UsdMicros, pub idle_days: u32 }
pub struct Score { pub per_mille: u32, pub idle_days: u32 }
pub fn score(usage: &AssetUse) -> Score;
pub fn worst_first<T: Clone>(assets: &[(T, Score)]) -> Vec<(T, Score)>;
```

- **评的是资产不是 Agent**：会话冻结即终结，没有「它」可供记挂声誉；跨会话携带价值的是被写下来的东西。
- **分子是被取用次数，分母是常驻字节**：同样的有用程度，占的地方越大越贵——那是它在每一次披露它的 prompt 里都要付的账。
- **`idle_days` 并列而不折进分数**：便宜且无用与昂贵且不可或缺是两回事，一个把它们藏起来的数字比两个数字更糟。
- **它恒不自己做决定**：排好序给人看；决定归 `metabolism`，采纳归 mode。

### 8-4 eval::metabolism（形状 1 判定）

```rust
pub const ASSET_IDLE_DAYS: u32 = 90;
pub const ASSET_FLOOR_PER_MILLE: u32 = 1_000;
pub enum Disposal { Keep, Warn { because: String }, Retire { because: String } }
pub fn dispose(usage: &AssetUse, score: Score, warned_already: bool) -> Disposal;
pub fn sweep<T: Clone>(assets: &[(T, AssetUse, Score, bool)]) -> Vec<(T, Disposal)>;
```

- **最重的处置是 `Retire`，不是删除**：退场＝不再被披露，字节仍在盘上与历史里。唯一能移走东西的是 Discard 册，而它按构造可还原。
- **先警告后退场**：没有任何东西在第一次被注意到的同一轮里停止被提供——那一轮正是人说「它重要」的机会。
- **理由随处置同行**：一份没有解释就消失了的清单，教会的是不要相信那个让它消失的机制。

每个模块落地前先在本节开一个 `### 8-n <模块>（形状）` 子节，给出类型签名与它们为什么是这个形状，写法同 `collab-SPEC.md` §8。

## 8.5 两个设计

（两个实质不同的接口方案，按杠杆率与缝的位置比较；落选方案就地留痕。）

## 9 工作流程

## 10 实现逻辑

## 11 边界枚举

## 12 错误处理

（逐码回答「能否让它不可能发生」——设计规则十。）

## 13 依赖选型

拓扑硬约束：`kernel` 与 `memory`（ARCHITECTURE.md §2）。统计计算全用整数（判定路径禁浮点，§9 硬化）——比率以千分数（`per_mille`）表达，不引入统计库。新外部依赖逐个论证，无论证即不引。

## 14 硬编码声明

## 15 影响面

## 16 测试与约束

## 17 模型体验

（入窗什么｜token 代价｜对 prefix 缓存的影响；无贡献则写「零字节，因为……」。）

## 18 文档同步

### 8-x eval::nesting——格式由一次 eval 决定，不由偏好决定（形状 1 判定）

**它的产出是一个数字。** 计划树要住在一个模型每天编辑的文件里，而 TOML／JSON／Markdown 三选一此前是按口味争论的。口味不是证据：一个读起来舒服、每六次编辑错一次的格式，比一个没人喜欢的格式更糟。

**错法分布比错误率更要紧。** `Fault` 是穷尽枚举，且**按破坏力排序**：

| 取值 | 它是什么 | 为什么排在这里 |
|---|---|---|
| `LostField` | 能解析，少了一个字段 | **最坏**：唯一一种「文件仍然可读、而一个计划节点悄悄不存在了」的结局，下游没有任何东西会注意到 |
| `ChangedBystander` | 能解析，动了没让它动的值 | 可发现，但要有人去比 |
| `Unparseable` | 解析不了 | 当场变红 |
| `Truncated` | 结构中途停住 | 当场变红，且换一个更大的 token 上限就能修 |
| `NotApplied` | 一切完好，编辑没落上 | 当场看得出 |

`grade` **报最坏的那一个**：一个结果可以同时是好几种错，报最轻的那种会替这个格式说好话。`recommended` 先比错误率，**平手时比各自最坏的错法**——两个错得一样频繁的格式并不一样好。

**它不调用模型。** `Attempt` 是某个模型已经产出的东西。一个自己持有 provider 的 suite 无法离线跑、无法重放，而且量到的一半是网络。**语料自己解析不了时拒绝而不是记分**，否则语料会给自己打分。

三种格式读成同一组叶子（`path -> value`），因为问题问的是文档的叶子而不是它的语法；三条读法各读各的，就变成在比读法而不是在比格式。Markdown 那条**刻意严格**：一个会修复松散缩进的读法，会藏掉这个 suite 正在计数的那种失败。

### 8-5 eval::nesting 目录化

`nesting.rs` 原有 632 行，超出 400 行的文件上限，按「一个文件回答一个问题」切成三份：

- `nesting.rs`（293 行）——判定本身：`Shape`／`Fault`／`Attempt`／`Verdict`／`Grades` 五个类型，以及 `grade`／`tally`／`recommended`。它同时是索引位置，声明 `mod reading;` 并从中取用 `read` 与 `stops_early`，公开路径 `eval::nesting::*` 一个未变，crate 内其它文件的 `use` 一行未改。
- `nesting/reading.rs`（154 行）——语法一侧：`read`／`leaves`／`walk`／`scalar`／`markdown_leaves`／`stops_early`。把三种格式怎么读成同一组叶子，与「一次编辑错在哪里」的判定分开读。
- `nesting/tests.rs`（205 行）——原内联 `mod tests` 原样迁出，断言、名字与 13 个 `#[test]` 一个未动。

**无字段开放。** 跨文件只放宽了两个自由函数：`read` 与 `stops_early` 写作 `pub(super) fn`，仍不出 `nesting` 模块；`leaves`／`walk`／`scalar`／`markdown_leaves` 保持私有。

**apisync 未重写基线。** 搬走的全是私有项，`eval` 的公开面逐字节不变。

### 8-6 handoff 探针真的跑

`eval::handoff_probe() -> Probe`：名 `handoff`、版本 1、固定四问（任务是什么／做到哪了／下一步是什么／先读哪个文件）。它是数据，不是判定：问题改了就是版本 2，`compare` 对两个版本恒拒。谁问：装配层 `bin::assembly::probing`，在每次 succession 前后各问一次（runtime-SPEC §8-33），记 `eval_run`。本 crate 仍恒不在它所测量的回路里。

### 8-7 eval::ablation——City.md 有没有挣到它的长度（`ablation.rs` 形状 1 判定，`ablation/capabilities.rs` 形状 6 数据）

`docs/City.md` 由 `bin::assembly::genesis` 以 `include_str!` 编进二进制，是每个居民读到的第一份文本，因而它每多一段就向**每一次** prefix 收一次租。这里建的是一把尺：把这份文档按段切开，逐段拿掉，量一个居民因此**做不了什么**。产出是给下一次编辑那份文档的人的证据，不是墙。

**它恒不是门。** 入口是本模块自己的一条 `#[ignore]` 测试，只在有人点名时跑：

```
cargo nextest run -p eval --run-ignored all -E 'test(city_md)' --no-capture
```

`just check` 一个字节都不跑它。理由与 §1 同一条：这里量的是一份写给模型的文档，依据随文档而动，把它接成红灯只会让下一个编辑者去改依据。

**它不调用模型**，与 `nesting`（§8-x）同一个理由：一个自持 provider 的 suite 无法离线跑、无法重放，量到的一半是网络。可测量的替身是**能力与凭据**——一条 Capability 是「居民必须能做的一件事」，它的 `cue` 是文中授予这件事的那句逐字短语。段落拿掉之后凭据还在，能力就还在。

```rust
pub(crate) struct Capability { pub(crate) name: &'static str, pub(crate) cue: &'static str }
pub(crate) struct Passage { pub(crate) index: u32, pub(crate) opening: String, pub(crate) removed: ByteLen }
pub(crate) enum Cost {
    Untouched,                                    // 这一段不授予语料里的任何能力
    Restated { also_said: Vec<&'static str> },    // 它提到的话别处也说了，居民一件不少
    Sole { lost: Vec<&'static str> },             // 只有这里说，拿掉即失去
}
pub(crate) struct Charge { pub(crate) passage: Passage, pub(crate) cost: Cost }
pub(crate) struct Ablation { /* 私有：passages、corpus */ }
impl Ablation {
    pub(crate) fn new(document: &str, corpus: &'static [Capability]) -> Result<Ablation, AxError>;
    pub(crate) fn charges(&self) -> Vec<Charge>;
    pub(crate) fn costliest_first(&self) -> Vec<Charge>;
}
```

- **三值而非布尔**：`Restated` 是这套东西真正想看见的一格——一句在两处说了两遍的话，删掉其中一处不花钱。把它折进「有损／无损」两格，就再也看不出文档在哪里重复自己。
- **语料自己不能给自己打分**：`new` 在**整份**文档里找不到某条 cue 即拒（`E_INVALID_ARGS`，recovery 指向语料而非文档）。凭据漂开之后仍然记分，量到的是语料的陈旧程度。
- **切段规则**：空行切块，以 `- ` 开头的块并入上一段。City.md 的每张清单都是它上面那句话的展开；分开量，量到的是排版不是意思。
- **`costliest_first` 先比失去的能力数（降），平手比被删字节数（升）**：同样的损失，越短的段越是在挣它的长度；末位比 `index`，因为排序两次必须一样。
- **公共面零变化**：全部 `pub(crate)`。这把尺的唯一消费者是它自己的那条 ignored 测试，`eval` 的公开面逐字节不变，apisync 基线不动。

**它编不进产品库。** `lib.rs` 的 `mod ablation;` 携 `#[cfg(test)]`：这把尺唯一的消费者是它自己那条 ignored 测试，把它编进发布出去的库，发出去的是仪器而不是能力。dead_code 因此不是被 `#[allow]` 压掉的，是不存在的。

**一次真实运行**（`cargo nextest run -p eval --run-ignored all -E 'test(city_md)' --no-capture`，windows-msvc，`docs/City.md` 切出 12 段，语料 40 条）：

| 名次 | 段 | 字节 | 拿掉它，居民失去 |
|---|---|---|---|
| 1 | `Rules that the system enforces:` | 1266 | **8 条**——外来内容是数据、回收站、上传暂存区、`status` 问时间与待办、他人结果是主张、优先一手材料、`secret:` 引用、提交归城 |
| 2 | `Work:` | 729 | 7 条 |
| 3 | `Seeing and acting:` | 891 | 6 条 |
| 4 | `This building keeps its long work in markdown…` | 1026 | 6 条 |
| 5 | 协作段／`delegate` 段 | 412／504 | 各 3 条 |

**最贵的一段是 `Rules that the system enforces:`**：它是全文最长的一段，也是失去最多的一段，每 158 字节买回一条能力。**最省的是 `Update the roadmap and the memo…`**：93 字节买一条。**唯一不花钱的是标题行**（23 字节，0 条）。

**这次运行还测出一件没打算测的事：`Restated` 一次都没有出现。** 40 条能力在 City.md 里各只有一处凭据，全文没有一句话说了两遍。这份文档没有可以靠删重复来省的字节——它要短，就得有人决定放弃一条能力。
