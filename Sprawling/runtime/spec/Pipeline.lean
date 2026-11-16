-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::pipeline

规定 `pipeline`、`pipeline::adviser`、`pipeline::connector`、`pipeline::exec`（`crates/runtime/src/` 下同名的文件）。工具结果的信封、缩短的次序、窗口顾问与连接器结果。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `runtime::pipeline::adviser::tests`、`runtime::pipeline::connector::tests`、`runtime::pipeline::tests` 守住。
-/

/-!
### 8-7 runtime::pipeline（形状 1＋组装处）


```rust
pub struct PackContext<'a> {
    pub cap_bytes: u64,                       // 窗口余量推导的本次上限（调用方算；恒 ≥ 提示句预算）
    pub stamp: Option<ClockStamp>,            // clock::StampGate 的产出；None＝不携
    pub net_notice: bool,                     // gate::egress 首次公网放行信号
    pub steer: Option<(String, String)>,      // (source, text)；上一边界消费到的 Steer
    pub reminder: Option<ContextReminder>,    // 上下文提醒（§8-34）
    pub offload: Option<OffloadSite<'a>>,     // None＝无 CAS 可用（纯截断退路）
    pub sieve: Option<SieveRequest<'a>>,      // exec 结果才带；无站点即无 tee 即不压
    pub adviser: Option<Consultation>,        // 窗口顾问先跑，判断作参数从此入
}
pub struct Packaged { pub content: String, pub events: Vec<Payload> }   // events＝result_offloaded 载荷（入账归调用方）
pub fn package(result: &[u8], ctx: PackContext<'_>) -> Result<Packaged, AxError>;

// pipeline::adviser —— 顾问端口（形状 3 port＋1 判定）；三型在 pipeline 上重导出
pub use kernel::event::record::AdviserAsk;   // 问法与答案的词汇归 kernel，本模块只翻译与校验
pub struct Ask { pub kind: AdviserAsk, pub subject: String, pub options: Vec<String>,
                 pub material: Option<String> }
impl Ask { pub fn noul(subject: impl Into<String>, material: impl Into<String>) -> Ask;
           pub fn score(subject: impl Into<String>, material: impl Into<String>) -> Ask;
           pub fn choice(subject: impl Into<String>, options: Vec<String>) -> Ask; }
pub enum Consultation {
    Answered { ask: AdviserAsk, subject: String, answer: AdviserAnswer, elapsed_ms: u64 },
    FellBack { ask: AdviserAsk, subject: String, reason: AdviserFailure },
}
impl Consultation { pub fn answer(&self) -> Option<&AdviserAnswer>;
                    pub fn payloads(&self) -> Result<Vec<Payload>, AxError>; }   // adviser_asked＋答或回落
pub struct Adviser { /* answer —— 私有 */ }
impl Adviser { pub fn none() -> Adviser;
               pub fn with(answer: impl FnMut(&Ask, &Conversation) -> Result<AdviserAnswer, AdviserFailure>
                                 + Send + 'static) -> Adviser;
               pub fn consult(&mut self, ask: Ask, conversation: &Conversation, elapsed_ms: u64) -> Consultation; }
```

- 定序：**判定由 `compaction::plan` 一处给出**，答 `Shrink { Keep, Cut(Strategy), MustOffload }`。`Keep` →原样；`MustOffload`（结构化、未知内容、以及非 UTF-8 字节）与「`Cut` 且 `len ≥ OFFLOAD_MIN_BYTES`」→ 有 `OffloadSite` 就 offload；`Cut` 而无站点或不够大 → `compaction::shorten` 按已定的 `Strategy` 裁。**`MustOffload` 而无站点是一次带恢复语的 `Err`，不是私自的字节切**：截半的结构化数据看上去仍可解析，那正是它比缺席更糟的理由；pipeline 若自己切字节，就当场推翻了 compaction 的定规——一条规则两个家。标记由 `elision` 产出且只出现一次。
- **`Shrink::Cut(Strategy::Sections)` 不交给 offload。** 长文档的节标题骨架是该类存在的理由，而 offload 的替代体是文件头＋指向全文的指针，恰好把骨架丢掉；故 `Markup` 一律走 `shorten`，其余 `Cut` 仍按 `len ≥ OFFLOAD_MIN_BYTES` 入 store。
- **非 UTF-8 的静默回落已登记**：`Err(_) => Content::Unknown` 丢掉 `Utf8Error` 的原因，把「二进制」折成「未知」；`Unknown` 的整块离窗使它的行为安全，但名字不对。修法需要一个新内容类与它的 plan／shorten 臂，不止一行，故只登记不改。
- **顾问端口住 `runtime::pipeline::adviser`（形状 3 port＋1 判定），先于 `package` 跑。** 顾问装不进同步的 sieve 链（`Draft::step` 是 `impl FnOnce(&[String]) -> Vec<String>`，无 Result 无 async，而 sieve→package→package_exec 整链同步）；做法是顾问在链外先办，判断作 `PackContext.adviser` 喂进来。三条问法（`Noul` 是非＋概率／`Score` 有序打分／`Choice` 选一，后者仅用于开 session 选模型）与答案／回落载荷形状归 `kernel::event::record`，本模块只翻译与校验。
- **顾问是窗口的顾问，不是前缀的顾问。** 端口签名只拿得到 `&Conversation`（与一个已拼好的 `Ask`）：拿不到 `FrozenConfig`，也拿不到 `FrozenPrefix`。逐轮换模型／effort 是前缀失效，不是窗口调整，故在本端口里写不出来；要挡它们得在 `CallShape` 上挡（§8-3）。
- **顾问的影响只有两条臂，下界是「今天的每个数字」。** `Score { score_bp }` 按 basis points 缩放 cap 给能裁的内容类用；若缩放会把「本会整块保留」的 Structured／Unknown 变成 `MustOffload`，则沿城市自己的 plan 与预算（把一条密度分变成一次拒绝，正是 `Keep` 在防的那件事）。`Noul { keep: false, .. }` 只在「有 `OffloadSite` 且结果大于 cap」时把它移出窗口：offload 只存必须裁的东西，更小的结果没有放得下的去处。`Choice` 不进 `package`（只在开 session 选模型时用）。
- **失败策略：无回答即无调整，且写进账本。** 未装顾问（`Adviser::none`）、端点不可用／超时、答非所问（问 `Noul` 答 `Score`、选择不在选项内、概率越界）一律回落，`Consultation::payloads()` 产出 `adviser_asked` 加 `adviser_answered`／`adviser_fell_back` 两条载荷随 `Packaged::events` 出去。**`answer()` 返回 `None` 时上面每一个数字在原地不动，所以最坏情况恰好等于今天的行为。**
- **顾问端点走既有 `AttachEndpoint` 登记路径，不造第二套 provider 表。** 配置不新增 TOML 段（`ConfigLayer` 只有 effort／sandbox／mcp 且四处 `deny_unknown_fields`）；`gateway::adviser::AdviserClient` 收路由已产出的 `Chosen`，用 `adapter_for` 同一支笔、同一份凭证兑付与 deadline。`Choice` 问法在开 session 选模型时用，前缀成形之前。
- 信封三附件一处组装：正文后依序追加 clock 行／net_notice 行（恒一次：正在连接互联网提醒，英文定句）／steer 行（`user:`／`@ID:` 前缀）；三行字节不计入 cap（附件与负载分账，附件有自己的封顶常数在实现内断言）。
- 按内容分类的缩短判定住 `compaction`（八类内容、四种策略），sieve 住 §8-27；本模块只按它们的答案走「原样／offload／截断」三臂。
-/

/-!
#### 8-27-10 连接器结果：`runtime::pipeline::connector`


形状：decision（与 `pipeline::exec` 同形）。一台 MCP server 的一次回答 `{ content: [块…], … }` 进窗口之前，文本块合起来量一次长度：

```rust
pub const CONNECTOR_CAP_BYTES: u64 = 16_384;
pub fn package_connector(outcome: ToolOutcome, offload: OffloadSite<'_>) -> Result<ToolOutcome, AxError>;
```

- **声音块进 CAS，窗口里只留引用**：`type: "audio"` 的块与图片块在同一步、同一种位置上处理。`mimeType` 以 `audio/` 开头、`data` 解得开 base64、字节不为空，三条都成立时，字节以 `put_for` 存进 CAS，块换成一个文本块 `[recording attached: <mimeType>, <字节数> bytes, <locator>]`；任一条不成立，换成 `[recording left out: <原因>]`。它不进 `ToolOutcome.attachments`：那里是模型看得见的图片，而模型听不见声音；要用这段录音的是一件工具（例如 `transcribe`），它要的是 locator（D15）。
- **图片块进 CAS，窗口里只留引用**：`type: "image"` 的块在文本那一步之后、在它原来的位置上处理，所以替它的那行字不并入被存下分窗的文本，模型不用翻页就读得到。`mimeType` 是 `image/png`、`data` 解得开 base64、字节不超过 `IMAGE_MAX_BYTES`、PNG 头读得出宽高，四条都成立时，字节以 `put_for` 存进 CAS，`ToolOutcome.attachments` 多一张 `ImageRef`（与浏览器截图同一种形状），块换成一个文本块 `[picture attached: image/png <宽>x<高>, <locator>]`。任一条不成立，块换成一个说明为什么没带图的文本块（`[picture left out: <原因>]`）。base64 恒不进窗口，也恒不进账本。其余非文本块（资源）照旧按原顺序留在其后。
- 回答没有 `content` 数组，或既没有图片块也没有声音块且文本合计不超过 `CONNECTOR_CAP_BYTES`：原样返回，一个字节不动。
- 超过：全部文本块按原顺序以换行连成一份，交 `package`（`sieve: None`，带落盘处）；`content` 换成**一个**文本块，装 `package` 给出的替身（开头一段加 `read` 可分窗读的路径），非文本块（图片等）按原顺序留在其后；`package` 记下的 `ResultOffloaded` 放进结果的 `offload` 字段，与 `exec` 的 `sieve` 字段同一种账。
- 替身是什么由 `package` 一处决定，本模块不另判：`Markup`（Markdown 一类）按 8-7 走节标题骨架而不入 store，故一份转换出来的长 Markdown 进窗口的是骨架，没有可翻的路径，`offload` 为空表；其余文本按 `OFFLOAD_MIN_BYTES` 入 store。
- 失败只有 `package` 自己的失败（`E_INVALID_ARGS`，原样上抛）。

**上限与 `exec` 同值、各有其名**：两者今天取同一个数，是因为窗口里一件工具答案的代价与来源无关；分开命名，是因为改其中一个不该悄悄改另一个。**决定**：交 `package` 而不是在这里另写一套截法——截多少、存不存由它一处决定，连接器答案与 `exec` 答案在窗口里守同一条规则；不包装时一次回答可以把整整 `MESSAGE_CEILING`（8 MiB）送进窗口。备选「让 `agent_protocols::McpTool` 自己截」被否：协议层没有 CAS 也没有 room，落盘处只有装配层有。调用点只有一个：`bin::assembly` 的 `Placing` 对 effect 为 `Connector` 的调用（首答与重放同样）调它。图片在这一步进 CAS，理由与截长文本相同：协议层没有 CAS 也没有 room。只量 PNG 的理由与浏览器相同：别的格式要第二个解码器才量得出边长，一个量错的边长比没有更糟；不是 PNG 的图片以一句话告诉模型改要 png。
-/

/-! D15 连接器把声音块存进 CAS，交出的是 locator 而不是附件

**决定**：MCP 答复里的 `audio` 块与图片块走同一步：容器认得、base64 解得开、不为空时存进 CAS，块换成一行带 locator 的字（§8-27-10）。它不进 `ToolOutcome.attachments`。

**理由**：桌面 server 录下的声音落在它自己机器的临时目录里，城里没有工具读得到；答复是它交回城里的唯一一条路（`crates/desktop/Spec.lean` D13）。base64 留在窗口里会把一段两分钟的录音变成五百万个字符，账本也跟着记下它们，所以它与图片一样在这一步离开答复。附件是模型要看的东西，`ImageRef` 带宽高，provider 的线把它画成图片；声音没有一个模型能读的形状，要读它的是一件工具，工具要的是 locator。连接器不判容器：哪些容器送得出去，由读这段录音的那一方判（`gateway::AudioType` 是那张表），连接器只认 `audio/` 这个前缀，于是容器的认法仍只有一处，而 runtime 也不为一张表多一条到 gateway 的依赖边。

**被否**：①给 `ToolOutcome` 加一个声音附件的槽（kernel 的类型多一臂，provider 的两条线都要为一种它们画不出的东西写一条拒绝）；②原样留在窗口里（见上）；③存进 CAS 并照着落盘一个 rest 文件给 `read`（`read` 读的是文本，一段 wav 的字节对模型没有用）。

**重开参数**：provider 的线有了声音输入的形状，模型自己能听；那时声音进附件，与图片同形。
-/
