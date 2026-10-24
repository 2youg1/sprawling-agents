-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::catalog

规定 `catalog`、`catalog::dormant`、`catalog::fit`、`catalog::guide`、`catalog::namespace`（`crates/runtime/src/` 下同名的文件）与 `tools::tool` 那一扇门。渐进披露与截断锁：一个 run 被告知哪些工具与 skill，各付多少字节。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。
-/

/-!
### 8-11 runtime::catalog（形状 6＋渲染）


```rust
pub struct CatalogEntry { pub name: String, pub disclosure: String, pub expansion: String,
                          pub hash: Option<B3Hash>,    // 架上那份文档被读到时的哈希
                          pub package: Option<String> } // 包目录，由 city 的扫描给出；单文档为 None
pub struct SkillPin { pub name: String, pub hash: B3Hash }
pub struct Catalog { /* tools: BTreeMap<ToolName,…>、skills: BTreeMap、mode: Option<Mode> —— 私有 */ }
impl Catalog {
    pub fn new() -> Catalog;
    pub fn admit_tool(&mut self, meta: &ToolMeta, namespace: Namespace) -> Result<(), AxError>; // disclosure 非空；重名＝E_INVALID_ARGS；命名空间由准入它的装配层说（§8-60）
    pub fn admit_skill(&mut self, entry: CatalogEntry) -> Result<(), AxError>; // 只收阅览室准入者（装配层按楼的 city::policy 规则求值后直供）；expansion 是城内地址
    pub fn admit_carried_skill(&mut self, entry: CatalogEntry) -> Result<(), AxError>; // 城外书架上的一件：expansion 是扫描读到的那份文档正文（§8-29-6）
    pub fn set_mode(&mut self, mode: Mode);                                    // Resident 段的 mode 行：run 开始时的 mode；会话中换 mode 不改它（§8-62）
    pub fn render(&self) -> String;              // Resident 段的 catalog 部分：段头一行、mode 行、dev 行，然后休眠索引（§8-60）；BTreeMap 序恒定
    pub fn tool_defs(&self) -> Vec<ToolDef>;     // ChatRequest.tools 的唯一来源：各 mode 常驻核心的并集，session 开始时定下（§8-60、D25）
    pub fn expand(&self, name: &str) -> Option<Expansion>;   // 第二级披露（怎么用），§8-6
    pub fn skill_pins(&self) -> Vec<SkillPin>;   // 本 Run 拿到了哪几份，当时各是什么字节
}
```

- **`hash` 是 `Option`，而那个 `None` 不是「没算」**：目录里另有两类条目的正文由本构建自己握着（mode 的纪律、dev 那一条），它们背后没有一份能在无人看着时改掉的文档。
- **pin 从 catalog 取，不重扫一遍书架**：catalog 已经是「本 Run 能够到什么」的权威，再扫一次就是在另一个时刻对同一个问题给第二个答案。
- **一件 skill 怎么交给 run，由它进 catalog 的那扇门定**：`admit_skill` 收城内书架上的一件，`expand` 答 `Expansion::Skill`，`read` 到那个地址去开；`admit_carried_skill` 收城外书架上的一件，`expand` 答 `Expansion::Said`，正文就是 catalog 手里那份。两扇门而不是一个布尔参数：`expansion` 这一格在两扇门后是两种东西（地址与正文），门名把这件事说在调用处。两扇门共用同一套卫生检查（名字与一行披露非空、不重名），重名跨两扇门同样拒。

**`render()` 与 `set_mode()` 的生产调用者是装配层的 prefix 组装**。常驻核心的工具走 `ChatRequest.tools` 到达模型，其余已准入的工具与 skill 走休眠索引；没有 `render()`，**阅览室准入的 SKILL 与本 Run 所处的 mode 就到不了任何模型**，`city::library` 的准入判定就是一道没有下游的门。

接法：`Catalog::render()` 追在 `identity.segment_bytes()` 之后，合成 Resident 段。**不另开第五个槽**：一个居民能够伸手取到什么，与它是谁同属一类常住事实，且两者都随 Run 冻结，故前缀在整个 Run 的寿命里仍可缓存。装配层因此把 prefix 的组装移到目录建好之后。

**第二级披露经 `read`**：SKILL 的 `expansion` 是 city 的 holding 地址 给的一个地址，坐在**保留前缀 `.sprawling/` 下**。`render()` 不印那个地址；模型按名字调 `read`，`read` 先查 catalog（§8-29），所以它不必知道、也读不到那个保留前缀下的路径。
-/

/-!
### 8-60 截断锁（`catalog`、`catalog::dormant`、`catalog::guide`、`catalog::namespace`、`mode::core_tools`，形状 1 判定）

一件能力（注册的工具、MCP 工具、阅览室里的 skill、`exec` 能调的命令行程序）对一次 session 处在三档之一，每一档付的字节由本节一处定：

1. **楼没准入**：它不进这栋楼的任何上下文——prefix、`ChatRequest.tools`、休眠索引、`tool` 的答复里都是零字节。准入沿用每一类已有的那道门：工具由装配层按楼的职业、规则与书（`accounting::worker::workbench`）决定登不登记，MCP 由楼的 `[[mcp]]` 行，skill 由楼的阅览室（`crates/city/Spec.lean` §8-8），命令行程序由楼的 exec 规则。catalog 只见到已准入的，所以「没准入＝零字节」是装配层不登记它的直接结果，不是 catalog 再判一次。
2. **已准入、本 session 不常驻**：工具按命名空间折进休眠索引，一个命名空间一行，不论它有几件；skill 一件一行。整份索引（段头、条目、`+N more`）至多 `DORMANT_INDEX_CEILING` 字节。
3. **要用**：模型经 `tool` 工具找它、看它、调它：`list` 展开一个命名空间，`find` 按词找，`detail` 取完整指南（工具的说明与入参 schema；skill 的一行披露与 `read` 它的名字），`run` 调它；工具表在 session 里恒不变（D21）。

```rust
// runtime::catalog
pub const DORMANT_INDEX_CEILING: usize = 1024;
pub const FIND_HITS: usize = 8;                                       // `find` 一次至多列几件（D22）
impl Catalog {
    pub fn list(&self, namespace: Option<&str>) -> String;            // 不给：每个命名空间一行（名字、件数、提示）；给：该命名空间每件一行，超过 LIST_CEILING 只列名字
    pub fn find(&self, query: &str) -> String;                        // 按等级匹配的至多 FIND_HITS 件，按命名空间分组
    pub fn detail(&self, name: &str) -> Result<String, AxError>;      // 整名＝完整指南；不认得的名字＝E_INVALID_ARGS，带最近的名字
    pub fn resolve_run(&self, call: &ToolCall) -> Result<ToolCall, AxError>; // `tool` 的 `run` 的唯一解包与按 schema 核对（§8-61）
}
// runtime::catalog::namespace
pub enum Namespace { Own(kernel::ReservedNamespace), Server(kernel::ServerLabel) }
// kernel
pub enum ReservedNamespace { Computer, Media, Record, Skill, Sprawling, Web } // ALL、as_str、named；city 的配置层拒绝用它们作 label 的 MCP 服务器
// runtime::mode
pub fn core_tools(mode: kernel::Mode) -> &'static [&'static str];     // 本 mode 允许执行的常驻工具名，含 `tool`
pub fn in_some_core(name: &str) -> bool;                               // 在某个 mode 的核心里：ChatRequest.tools 取的就是它（D25）
pub fn mode_admits(policy: &kernel::RunPolicy, name: &str) -> Result<(), AxError>; // 在某个核心里而不在生效 mode 的核心里 → E_GATE_DENIED
// runtime::tools（名字是关联常量，登记与 core_tools 读同一个）
impl ToolTool { pub const NAME: &'static str = "tool"; }
pub struct ToolTool { /* catalog: Arc<Mutex<Catalog>>、meta —— 私有 */ }
impl ToolTool { pub fn new(catalog: Arc<Mutex<Catalog>>) -> Result<ToolTool, AxError>; } // args：{op, namespace?, query?, name?, args?}；Effect::Read
```

- **一件工具的命名空间由准入它的装配层说**（`accounting::worker::workbench` 把每件工具连同一个 `Namespace` 交给 `admit_tool`）：只有那里知道一件工具是城自己的还是某台服务器的。城自己的工具归预留命名空间（`sprawling`：城与居民之间的协作与自身状态；`computer`：浏览器；`media`：识图与转写；`record`：回看与提案；`web`：联网搜索；`skill`：阅览室的 skill），外部服务器的工具归它的 label——它的每件工具拼作 `{label}_{tool}`，`ServerLabel` 不含 `_`，所以 `tool find` 去掉前缀得到的成员名唯一。预留名由 `kernel::ReservedNamespace` 一处列出，city 的配置层拒绝以它们为 label 的服务器（`crates/city/spec/ConfigLayers.lean` §8-4）：一台 MCP 服务器若叫 `sprawling`，`tool list sprawling` 会把外部工具当作城自己的能力列出。被否：按名字切第一个 `_` 推出命名空间——城自己的工具名（`web_search`）也含 `_`，推出的命名空间会是错的。
- **索引的写法**：一行段头；然后先工具的命名空间、后 skill，各按名字的字节序，一件一行。命名空间写作 `- <名> (<件数>): <提示>`：预留命名空间的提示是表里那一句，其余是成员去掉 `<label>_` 前缀后的名字，以 `, ` 相连；skill 写作 `- skill <名>: <提示>`，提示是披露的第一句（到第一个 `. ` 或行尾）。提示在 `HINT_MAX_BYTES` 处按字符边界截断。逐件贪心：带提示的那行放得下就放，放不下退到只有名字的那行，再放不下就停，剩下的件数写成末行 `+N more`。预算先扣掉 `+N more` 在 N 取全部件数时的长度，所以那一行恒放得下。没有休眠的件时整块不出现。
- **每个 mode 允许执行哪些常驻工具，由 `mode::core_tools` 一处回答**：`chat` 是 `read`、`search`、`status`、`tool`；`work` 再加 `edit` 与 `exec`。`ChatRequest.tools` 的常驻核心是 `Mode::ALL` 各自 `core_tools` 的并集，在 session 开始时定下（D25），所以会话中换 mode 不改工具表，只改门（§8-62）。表里的名字只有已准入的才常驻：楼没登记 `exec`（市政厅），`exec` 就不在任何一档。`status` 在派发时冻结的那几行进 Run 段之前留在常驻核心里，因为 `City.md` 让每个居民去调它。

下面的模型是索引装填的长度账、命名空间的折叠与三档的可见性；Rust 的 `catalog::dormant` 按同一贪心拼出字节，`catalog::tests` 对拍上限、折叠与三档（§16）。
-/

/-!
### 8-61 经 `tool` 的 `run` 的一次调用（`catalog::resolve_run`、`catalog::fit`、`turn::wave` 的 `resolve_call`）

```rust
pub trait ConcurrentInvoke {
    /// 模型发出的一次调用换成它所指的那件；默认原样交回。
    fn resolve_call(&self, call: ToolCall) -> ToolCall { call }
    // meta_of、ahead、admit、tool、account 不变
}
```

- **换在工具面的入口，一波开始时与推测起跑时各一次**：工具波在判只读前缀之前把每条调用交给 `resolve_call`，推测门在问 `reads_only` 之前同样换一次，所以登记、门、去重键、`tool_called` 与 `tool_result` 读到的都是那件工具本身的调用，它的 Effect 过它自己的门。只有 `op` 为 `run` 的 `tool` 调用会被换；`list`、`find`、`detail` 由 `ToolTool` 自己答。会话里那条 assistant 消息不动：模型说的是 `tool`，下一次请求照原样送回，`tool_result` 按 `id` 配对，所以换出来的调用沿用原 `id`。
- **`Catalog::resolve_run` 是唯一的解包**：参数缺 `name`、`name` 是 `tool` 自己、`name` 不是本 run 已准入的工具，或 `args` 不合那件工具的入参 schema，都拒，`E_INVALID_ARGS`；schema 不合的拒词带着那份 schema，候选名的拒词带最近的几个名字。装配层的工具面拿到拒绝时把原调用交回，它落到 `ToolTool`，`ToolTool::invoke` 再问同一个 `resolve_run`，把同一个拒绝作为 `tool_result` 交给模型；同一个函数问两次答同一句，所以拒因只有一个作者。
- **`fit` 只核顶层**：参数是对象；`required` 的每一项在；`properties` 里声明了 `type` 的每一项类型相符（`string`、`integer`、`number`、`boolean`、`array`、`object`、`null`，或它们的数组）；`additionalProperties: false` 时没有多余的键。更深的形状由那件工具自己的解析判，它照旧拒得了。
-/

namespace Runtime.Catalog

/-- D19（推断，见下）：休眠索引整份的上限，字节，按 UTF-8 计。

定规：截断锁的三档，与休眠索引约 1 KiB 的量级；以下是从这条定规推出的选择。上限管**整份索引**而不是每一件：按件封顶时四十件休眠工具仍要四十 KiB，不是「降低到最小」。用字节而不是 token：每家 provider 的分词不同，字节是确定的、与 provider 无关的，重放与分叉逐字节一致。被否的备选：每件 1 KiB；按 token 计的上限（每家一个数）；让模型概括（定规：二进制里没有模型；而且重放要确定）。重新打开它的参数：脚本化的 run 显示 agent 找不到它需要的能力时，上限改为 4 KiB（把量级读作约 1K token 时的字节数）。 -/
def dormantCeiling : Nat := 1024

/-- 索引里的一件，以它两种写法的字节数记：带提示的那行与只有名字的那行。 -/
structure Entry where
  hinted : Nat
  bare : Nat
  deriving Repr

/-- 一件在索引里是怎么出现的。 -/
inductive Shown where
  | hinted
  | bare
  deriving DecidableEq, Repr

/-- 装填的结果：逐件的写法、用掉的字节、没列出的件数。 -/
structure Packed where
  shown : List Shown
  used : Nat
  left : Nat
  deriving Repr

/-- `catalog::dormant` 的贪心：按稳定序逐件，带提示的那行放得下就放，放不下退到只有名字，再放不下就停。

D20（推断）：贪心而不是最优装填。它是确定的、一遍走完，先列出的件恒不因后面的件而退成只有名字；最优装填要在件与件之间权衡，换来的几十字节不值一个读者看不懂的次序。重新打开它的参数：索引常常截断到 `+N more`、且换一种装填能多列出几件。 -/
def pack : Nat → List Entry → Packed
  | _, [] => ⟨[], 0, 0⟩
  | room, e :: rest =>
    if e.hinted ≤ room then
      let p := pack (room - e.hinted) rest
      ⟨Shown.hinted :: p.shown, e.hinted + p.used, p.left⟩
    else if e.bare ≤ room then
      let p := pack (room - e.bare) rest
      ⟨Shown.bare :: p.shown, e.bare + p.used, p.left⟩
    else ⟨[], 0, rest.length + 1⟩

/-- 装进去的字节不超过给它的预算。 -/
theorem pack_used_le : ∀ (room : Nat) (es : List Entry), (pack room es).used ≤ room
  | _, [] => by simp [pack]
  | room, e :: rest => by
    have h1 := pack_used_le (room - e.hinted) rest
    have h2 := pack_used_le (room - e.bare) rest
    unfold pack
    split
    · dsimp only; omega
    · split
      · dsimp only; omega
      · dsimp only; omega

/-- 没列出的件数不超过全部件数。 -/
theorem pack_left_le : ∀ (room : Nat) (es : List Entry), (pack room es).left ≤ es.length
  | _, [] => by simp [pack]
  | room, e :: rest => by
    have h1 := pack_left_le (room - e.hinted) rest
    have h2 := pack_left_le (room - e.bare) rest
    unfold pack
    split
    · dsimp only; simp only [List.length_cons]; omega
    · split
      · dsimp only; simp only [List.length_cons]; omega
      · dsimp only; simp only [List.length_cons]; omega

/-- 带提示的各行加起来放得下时，每一件都带提示列出，没有 `+N more`：截断只在放不下时发生。 -/
theorem pack_whole_when_it_fits : ∀ (room : Nat) (es : List Entry),
    (es.map Entry.hinted).sum ≤ room →
      (pack room es).shown = es.map (fun _ => Shown.hinted) ∧ (pack room es).left = 0
  | _, [], _ => by simp [pack]
  | room, e :: rest, h => by
    simp only [List.map_cons, List.sum_cons] at h
    have ih := pack_whole_when_it_fits (room - e.hinted) rest (by omega)
    have hle : e.hinted ≤ room := by omega
    unfold pack
    rw [ite_eq_left hle]
    exact ⟨by simp [ih.1], ih.2⟩

/-- 一份索引的字节：段头、装进去的行、有件没列出时的 `+N more` 行。`marker n` 是那一行在 N＝n 时的长度；预算先扣掉 N 取全部件数时的长度。 -/
def indexBytes (header : Nat) (marker : Nat → Nat) (es : List Entry) : Nat :=
  let p := pack (dormantCeiling - header - marker es.length) es
  header + p.used + (if p.left = 0 then 0 else marker p.left)

/-- **整份索引不超过上限**：只要段头与最长的 `+N more` 行放得下（Rust 里两者都是短常量，N 是件数），且那一行的长度随 N 不减（十进制位数）。 -/
theorem index_within_ceiling (header : Nat) (marker : Nat → Nat) (es : List Entry)
    (room : header + marker es.length ≤ dormantCeiling)
    (monotone : ∀ a b, a ≤ b → marker a ≤ marker b) :
    indexBytes header marker es ≤ dormantCeiling := by
  unfold indexBytes
  dsimp only
  have hu := pack_used_le (dormantCeiling - header - marker es.length) es
  have hl := pack_left_le (dormantCeiling - header - marker es.length) es
  have hm := monotone _ _ hl
  split <;> omega

/-- 一件能力对一次 session 的档位。 -/
inductive Standing where
  | unadmitted
  | dormant
  | core
  deriving DecidableEq, Repr

/-- 一件能力：名字、档位，与它在索引里的两种写法。 -/
structure Capability where
  name : String
  standing : Standing
  entry : Entry
  deriving Repr

/-- `ChatRequest.tools` 里的件：只有常驻核心。 -/
def toolList (cs : List Capability) : List Capability :=
  cs.filter (fun c => c.standing == Standing.core)

/-- 休眠索引里的件：只有已准入而不常驻的。 -/
def indexed (cs : List Capability) : List Capability :=
  cs.filter (fun c => c.standing == Standing.dormant)

/-- 一次请求为这些能力付的字节：常驻工具的定义（`defBytes`）加休眠索引。 -/
def requestBytes (defBytes : Capability → Nat) (header : Nat) (marker : Nat → Nat)
    (cs : List Capability) : Nat :=
  ((toolList cs).map defBytes).sum + indexBytes header marker ((indexed cs).map Capability.entry)

/-- **没准入的件付零字节**：在一份能力表里加进一件没准入的，请求的字节一个不变——它既不在工具表，也不在索引。 -/
theorem unadmitted_costs_nothing (defBytes : Capability → Nat) (header : Nat) (marker : Nat → Nat)
    (cs : List Capability) (c : Capability) (h : c.standing = Standing.unadmitted) :
    requestBytes defBytes header marker (c :: cs) = requestBytes defBytes header marker cs := by
  simp [requestBytes, toolList, indexed, h]

/-- 常驻的件不进索引，休眠的件不进工具表：一件只付一档的价。 -/
theorem one_standing_one_place (cs : List Capability) (c : Capability) :
    ¬ (c ∈ toolList cs ∧ c ∈ indexed cs) := by
  simp only [toolList, indexed, List.mem_filter, beq_iff_eq]
  intro ⟨⟨_, hc⟩, ⟨_, hd⟩⟩
  rw [hc] at hd
  exact absurd hd (by decide)

/-- 一次经 `tool` 的 `run` 的调用：它的 `id`、它指的那件的名字与交给那件的参数。参数的形状对模型是抽象的。 -/
structure Outer (α : Type) where
  id : String
  target : String
  args : α

/-- 换出来的那次调用。 -/
structure Routed (α : Type) where
  id : String
  name : String
  args : α

/-- `resolve_run` 的三种拒绝。 -/
inductive Refusal where
  | itself
  | unknown
  | misfit
  deriving DecidableEq, Repr

/-- 目录那扇门自己的名字。 -/
def toolName : String := "tool"

/-- `Catalog::resolve_run` 的模型：`schemaOf` 答已准入工具的入参 schema（没准入＝`none`），`fits` 是 `catalog::fit`。

D21（推断）：**请求的工具表在 session 里恒不变**，第三档经会话而不经工具表。provider 把工具定义放在缓存前缀的最前（tools、system、messages），session 中途加一件定义会在每次启用时丢掉整份 prompt cache，代价可能大过它省下的字节。所以 `tool` 的 `detail` 答复是一次普通的工具结果，追加在缓存本来就在长的末尾；休眠的工具经 `tool` 的 `run` 调用，城按那件的 schema 核参数，不合就带着 schema 拒。两者都是普通的工具调用，账本本来就记，重放与分叉逐字节一致，启用不需要新的事件种类。被否的备选：把启用的定义追加进工具表（毁缓存，还要一种启用事件）。provider 自家的延迟加载（Anthropic 的 `tool_reference`、OpenAI Responses 的 `tool_search_output`）已让工具表中途增长而不失效缓存，但只有两张脸有，而城说三种兼容格式；它作为 gateway 兼容格式的能力在下一版加在 `tool` 的 `find`／`detail` 之后，`run` 仍是每张脸都能走的路。 -/
def resolveCall {α σ : Type} (schemaOf : String → Option σ) (fits : σ → α → Bool) (o : Outer α) :
    Except Refusal (Routed α) :=
  if o.target = toolName then .error .itself
  else match schemaOf o.target with
    | none => .error .unknown
    | some s => if fits s o.args then .ok ⟨o.id, o.target, o.args⟩ else .error .misfit

/-- **换出来的调用沿用原 `id`、不再是 `tool`、参数合它的 schema**：结果按 `id` 与会话里那条 `tool` 调用配对，门判的是那件工具本身。 -/
theorem routed_is_the_target {α σ : Type} (schemaOf : String → Option σ) (fits : σ → α → Bool)
    (o : Outer α) (r : Routed α) (h : resolveCall schemaOf fits o = .ok r) :
    r.id = o.id ∧ r.name = o.target ∧ r.name ≠ toolName ∧
      ∃ s, schemaOf o.target = some s ∧ fits s r.args = true := by
  unfold resolveCall at h
  split at h
  · exact absurd h (by simp)
  · rename_i hne
    split at h
    · exact absurd h (by simp)
    · rename_i s hs
      split at h
      · rename_i hf
        cases h
        exact ⟨rfl, rfl, hne, s, hs, hf⟩
      · exact absurd h (by simp)

/-- 没准入的名字经 `run` 也到不了：第一档对目录那扇门同样成立。 -/
theorem unadmitted_is_unreachable {α σ : Type} (schemaOf : String → Option σ)
    (fits : σ → α → Bool) (o : Outer α) (h : schemaOf o.target = none) (hn : o.target ≠ toolName) :
    resolveCall schemaOf fits o = .error .unknown := by
  unfold resolveCall
  rw [ite_eq_right hn, h]

/-- 休眠索引里工具的件：每个命名空间一件，按第一次出现的次序。`ns` 是各件工具在准入时得到的命名空间。 -/
def folded : List String → List String
  | [] => []
  | n :: rest => if n ∈ folded rest then folded rest else n :: folded rest

/-- 折叠前后认得的命名空间相同。 -/
theorem mem_folded : ∀ (n : String) (ns : List String), n ∈ folded ns ↔ n ∈ ns
  | _, [] => by simp [folded]
  | n, m :: rest => by
    have ih := mem_folded n rest
    unfold folded
    by_cases h : m ∈ folded rest
    · rw [ite_eq_left h]
      constructor
      · intro hn; exact List.mem_cons_of_mem _ (ih.mp hn)
      · intro hn
        rcases List.mem_cons.mp hn with rfl | hr
        · exact h
        · exact ih.mpr hr
    · rw [ite_eq_right h]
      simp only [List.mem_cons]
      exact or_congr_right ih

/-- **一个命名空间只占一行**：往一个已有的命名空间里再加一件工具，索引的件数不变——一台有两百件工具的 MCP 服务器与一台只有一件的付同样的行。 -/
theorem one_line_per_namespace (n : String) (ns : List String) (h : n ∈ ns) :
    folded (n :: ns) = folded ns := by
  have h' : n ∈ folded ns := (mem_folded n ns).mpr h
  simp [folded, h']

/-- 同一个命名空间的任意多件折成一件。 -/
theorem many_members_one_line (n : String) : ∀ k, folded (List.replicate (k + 1) n) = [n]
  | 0 => by simp [folded]
  | k + 1 => by
    rw [List.replicate_succ, one_line_per_namespace n _ (by simp [List.mem_replicate])]
    exact many_members_one_line n k

/-- 一个可实现的实例：一件常驻、两件休眠、一件没准入时，工具表只有一件，索引只列两件。 -/
example :
    let cs : List Capability :=
      [⟨"read", .core, ⟨20, 8⟩⟩, ⟨"apps_ping", .dormant, ⟨30, 13⟩⟩,
       ⟨"mail_send", .unadmitted, ⟨40, 13⟩⟩, ⟨"plan", .dormant, ⟨25, 8⟩⟩]
    (toolList cs).length = 1 ∧ (indexed cs).length = 2 := by decide

end Runtime.Catalog

/-!
### runtime 决定 D22、D23（截断锁的另两半）

- **D22（推断）：找是城里确定的等级匹配，指南是能力自己的文字、一字不删。** `find` 把问话按空白切成小写的词，每个词在每件已准入能力上取它够到的最高一级：名字（或去掉命名空间前缀的成员名）全等计 8、名字前缀计 4、名字子串计 2、正文子串计 1；正文是说明、入参名与入参说明。各词求和，按分降序、再按名字排，列出前 `FIND_HITS` 件，按命名空间分组。`detail` 按整名答那件的完整指南。所以索引可以很小，找回靠匹配而不靠读被截短的提示。skill 的正文仍只经 `read` 交出：`read` 是打开架上文件、核对读界与包目录的唯一一扇门（§8-29），`detail` 对一件 skill 答它的一行披露与「`read` 它的名字」。被否的备选：BM25（权重取决于语料统计，同一句问话在两栋楼排出两种次序）；`detail` 自己读 skill 正文（第二个读架子的地方）；嵌入向量检索（二进制里没有模型，且不确定）。重新打开它的参数：脚本化的 run 显示等级匹配找不回它要的能力，或问话常以 `/正则/` 写出。
- **D23（推断）：命令行程序不另立第二份目录。** 楼对命令的准入就是楼已有的 exec 规则；prefix 里没有任何一处列出命令；命令行程序的第三档是 agent 经 `exec` 跑它自己的 `--help`，今天就成立。所以本节对 CLI 只确认零字节：没有要移进休眠索引的列表。重新打开它的参数：某处开始把可用命令写进 prefix。
- **不在 v1 里（截断锁的现状）**：一件在一栋楼的多数 session 里都被用到的能力，升格为那栋楼的常驻工具——等账本里按能力计的调用次数量出来再定（`tool_called` 记的是换出来的那件，所以计数按能力而不是按 `tool`）；派活时点名子 agent 要用的能力，让它一开始就带着它们的指南——今天子 agent 是一次自己的 session，它自己 `find`。
-/

/-! D25 会话中可改运行策略之后，常驻核心是各 mode 核心的并集，mode 只改门与追加的一句

**决定**：`ChatRequest.tools` 的常驻核心在 session 开始时定成 `Mode::ALL` 各自 `core_tools` 的并集（今天即 `work` 的那一组：`read`、`search`、`status`、`tool`、`edit`、`exec`，仍只取楼已准入的）；`mode::core_tools(mode)` 继续回答「这个 mode 允许哪些常驻工具真正执行」，不再决定工具表：`mode::mode_admits` 读它，`edit` 与 `exec` 在每次调用的第一道门问它，读的是 run 的策略格（§8-62）；模式之间差的正是这两件，而它们本就持格的读方，所以门不放在 bench——bench 不持格，放那里要再造一个读方。不在任何核心里的工具经 `tool` 的 `run` 到达，这道门不判它。会话中改运行策略（kernel D21）只改两件事：门按新策略判，下一段消息末尾追加一句说明新策略。工具表与冻结的前缀一个字节都不动，所以提示缓存不失效。

**理由**：D21 要工具表在 session 里恒不变，因为中途改工具定义会丢掉整份 prompt cache；权限与模式又要能在会话中改（runtime D28）。两者同时成立的唯一办法是工具表从开头就够大，模式的差别落在门上。代价是 `chat` 的 session 多带两件工具的定义字节；它们由同一个前缀缓存，只在 session 的第一次请求付一次。

**被否**：①改 mode 时换工具表：违反 D21，每次改都整段缓存失效；②`chat` 里 `edit`／`exec` 只经 `tool` 的 `run` 走第三档：模型在 `work` 与 `chat` 之间看到同一件工具时有时直调、有时经 `run`，指南要说两套用法。

**重开参数**：某个 mode 的核心多出一件定义很大的工具、使并集的字节在小模型的窗口里显著时，重议是否只对能改策略的 session 用并集。
-/

/-! D110 一个 `tool` 工具、四个 op，取代 `describe` 与 `call` 两扇门；休眠索引一个命名空间一行

**决定**：常驻核心里能力目录只占一件工具 `tool`，`op` 取 `list`、`find`、`detail`、`run` 之一（§8-60）。休眠索引里的工具按命名空间折叠，一个命名空间一行（`one_line_per_namespace`），由 `tool list <命名空间>` 展开；skill 仍一件一行，因为 skill 的名字本身就是「何时用」的线索。

**理由**：两扇门各有一份定义字节、各有一段说明，模型要先学会哪扇门做什么；Claude Code 的 `ToolSearch`、Codex 的 `tool_search` 都是一件工具。索引一件一行时，一台两百件工具的 MCP 服务器就把 1 KiB 的预算吃光，别的命名空间只剩 `+N more`；一个命名空间一行，索引的长度随命名空间数而不随工具数增长。Codex 的延迟命名空间同样一行一个。

**被否**：①保留 `describe` 与 `call`，只改索引：常驻集仍多一件，两扇门的说明互相引用；②四个 op 拆成四件工具：常驻集变大；③索引仍一件一行、只提高上限：上限随最大的那台服务器涨，前缀为每个 run 付这份字节。

**重开参数**：某家 provider 让工具在会话中途原生加载而不失效缓存、且城说的每种兼容格式都支持它时，`find`／`detail` 改为原生加载命中的工具，`run` 留作兜底。
-/
