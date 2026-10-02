-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# accounting::worker::workbench::tools

规定工作台里本 crate 写的三件工具：`crates/accounting/src/worker/workbench/tools/ocr.rs`、`crates/accounting/src/worker/workbench/tools/transcribe.rs` 与 `crates/accounting/src/worker/workbench/tools/proposal.rs`。本文件是 `crates/accounting/Spec.lean` 的一个分部；下面每一节保留它在 accounting 规格里的标签 §8-n，别处引作 `crates/accounting/Spec.lean §8-n`，决定引作 `accounting D<n>`。
-/

/-!
### 8-20 工作台的两件读外来字节的工具：`ocr` 与 `transcribe`（`accounting::worker::workbench::tools::ocr`、`…::tools::transcribe`，形状 4 适配器；`crates/sprawling/Spec.lean` §8-131、§8-142）

```rust
// accounting::worker::workbench::tools::endpoints（登记里的一段）
impl Laying {
    // 造一个 BoundReader，按序交回这座楼可用的 `transcribe` 与 `ocr`；lay_out_workbench 把它们接在按用途加的工具之后。
    pub(super) fn endpoint_tools(&self, site: &Site, bound: &runtime::ReadBound)
        -> Result<Vec<Box<dyn kernel::Tool>>, AxError>;
}
// accounting::worker::workbench::tools::{transcribe, ocr}
impl Laying {
    pub(super) fn transcription_tool(&self, site: &Site, reader: runtime::BoundReader)
        -> Result<Option<TranscribeTool>, AxError>;
    pub(super) fn ocr_tool(&self, site: &Site, reader: runtime::BoundReader)
        -> Result<Option<OcrTool>, AxError>;
}
pub(super) struct TranscribeTool { /* reader、transcriber: Mutex<Transcriber>、meta —— 私有 */ }
pub(super) struct OcrTool { /* reader、policy、recogniser: Mutex<Recogniser>、meta —— 私有 */ }
// 两件都是 kernel::Tool；参数 `{ path }`；答 `{ path, text }`
```

- **两件工具读字节只经 `runtime::BoundReader`**（`crates/runtime/Spec.lean` §8-59）。`endpoint_tools` 用交给 `read` 与 `search` 的同一个 `ReadBound`、同一个 run 的树根与城的块仓造一个 `BoundReader`，克隆给两件工具；它是工作台登记里自成一段的一步，因为这两件工具共用这一扇门，而 `lay_out_workbench` 已在函数与文件的长度上限边上。本 crate 不判路径：`Address::is_within`、`Address::is_reserved` 与 `storage::WriteTarget::within` 曾在 `transcribe` 里替这扇门判，门落地后删去。
- **有没有这件工具，是一次 `select`。** `transcribe` 读 `ModelTag::Transcribe`，`ocr` 读 `ModelTag::Ocr`，都按 run 所在那座楼的楼规（`site.rules.policy()`）问端点账本；拒了，工具不上表。设施由 gateway 造：`gateway::transcriber_for` 与 `gateway::recogniser_for`，后者带上 `credentials::dialect_headers` 给这个 face 的头，与主模型的适配器同一张。
- **容器的认法：** 图按 `runtime::pipeline::connector::png_picture` 认（读界判过的字节整份读进来，再交它），录音按 `Named` 分：文件看扩展名（`gateway::AudioType::of_file_name`，`file:` Locator 也是文件），块看开头的字节（`gateway::Recording::read_unlabelled`）。
- **`ocr` 的设施在一把锁后面**，理由同 `transcribe`：凭据解析器是 `Send` 而不是 `Sync`，`recognise` 又要 `&mut`；同一个 run 的两次 OCR 轮流进行。
- **在表上的位置**：`transcribe` 之后、`playback` 之前，两件都在内置那一段（`crates/sprawling/Spec.lean` §8-142）。
- 验收：两件工具各自模块的测试经一个没有设施的工具判拒绝（别楼的机密路径、reserved subtree、认不得的容器、不在的文件、没有任何楼的 `cas:`），设施的拒绝原样交回；接上设施的那一半由 `crates/sprawling/tests/acceptance/` 的 `ocr` 与 `transcribe` 测试经回环端点证明。
-/

/-!
### 8-30 工作台的 `proposal`：run 提出与收回修改提案（`accounting::worker::workbench::tools::proposal`，形状 4 适配器；documents D35、D36）

```rust
// accounting::worker::workbench::tools::proposal
pub(in crate::worker) struct Proposing { /* relay: worker::Relay、clock —— 私有 */ }
impl Proposing {
    pub(in crate::worker) fn new(relay: Relay, clock: Arc<dyn Clock + Send + Sync>) -> Proposing;
}
impl Laying {
    pub(super) fn proposal_tool(&self, site: &Site, room: &Address, bound: &runtime::ReadBound)
        -> Result<ProposalTool<Relay>, AxError>;
}
pub(super) struct ProposalTool<L: kernel::Ledger> { /* reader、filing、desk: Mutex<Desk<L>>、meta —— 私有 */ }
// accounting::worker::workbench::tools::proposal::quoting（形状 1 判定：引文在城里那一版里是哪一段）
pub(super) fn offered(reader: &runtime::BoundReader, asked: &Offering) -> Result<ProposalOffered, AxError>;
// kernel::Tool。参数 { action: "offer", path, old, new } 或 { action: "withdraw", proposal }；
// offer 答 { proposal, doc, baseline, start, end }，withdraw 答 { proposal, withdrawn: true }
```

- **一件工具两个动作，按写登记。** `Effect::Write { domain: 房间 }`，与 `goal`、`signal` 同形：它写的是账本行，所以同一波里的 `offer` 与 `withdraw` 按序执行，不被当作读提前跑；`writes` 答 `Nothing`，因为树里没有文件动。它在表上排在 `playback` 之后、城外工具之前，所以每栋楼的 run 都有它。
- **`offer`：读城里那一版，找出原文，判长度，写一行。** `path` 经一个建在城根上的 `runtime::BoundReader` 打开（交给 `read` 的同一个读界，D32）：保留子树、读界关着的楼由那扇门拒；Locator 与块被拒，因为提案是关于城里那份文件此刻的字节（documents D35）。读出的字节不是文本（`Reading::Opaque`）时拒；是文本就整份解码，`old` 必须恰好出现一次，零次与多次各一句拒词，带次数；区间按那一版的编码换成字节：UTF-8 的两种，解码出的文字就是版本的字节（documents D5），UTF-16 的两种，每个码元两个字节。`old` 为空或与 `new` 相同时拒。然后 `documents::Offer::of` 判长度（documents D18），工具写一行 `proposal_offered`，记在这次 run、这个居民、这个房间名下，回答卡的身份（documents D13）。同一张卡再 `offer` 一次不再写行，答同一个身份；收回过的卡再 `offer` 被拒（documents D19）。
- **行经 lane 的 relay 写下。** relay 是 lane 唯一能写账本的门（D11）；记账线程写下这一行之后把它交给 `Governance` 的折叠（§8-22），`Query::Proposals` 从此答出这张卡，人的决定也从同一个折叠找它。
- **`withdraw` 只收这次 run 自己提出、还开着的卡。** 判它的是工具自己的一本小账（身份 ↦ 开着／收回过）：run 的身份每次派活新铸（`run_id_for` 读时刻），所以这次 run 提出的卡只出自这件工具的这一个实例。别的 run 的卡、没提出过的身份、收回过的卡都拒 `E_INVALID_ARGS`，不写行。
- **当前状态：人在 run 还在跑时决定了它的一张卡，run 随后收回同一张卡，会多写一行 `proposal_withdrawn`。** 工具看不见那次决定；`views::proposals::Proposals::close` 照最后写下的一行把卡记成收回过。卡上的字节已经由人的决定落下，`open_on` 对两种处理都拒，所以没有字节写错，错的是折叠记下的「怎样处理的」，与 documents D19「处理过的卡不再动」不合。补法是折叠对已经处理过的卡不再改（`close` 保留第一次处理，`views/proposals.rs` 里一行），它也让任何一条迟到的收回成为被拒的一步；另一条路是收回改走记账线程的问询，像 `goal` 的登记那样由 `Governance` 当场判（`crates/sprawling/Spec.lean` §8-42-8），那要在 `relay::Wake` 加一臂。
- 验收：`worker::workbench::tools::proposal::tests`（一次 `offer` 恰写一行、文档字节不动；收回别人的卡、收回两次、重提收回过的卡都被拒；UTF-16 文档上引出的原文经 `documents::decide` 落得下）；`crates/sprawling/tests/acceptance/` 的 catalogue 里 `proposal` 一段（`views::ask` 的 `Query::Proposals` 答出这张卡，属于这次 run，文档字节不动）；citysim 的 `tests/proposal_baseline.rs`（citysim D22：文档被城外的写者挪动之后，人接受这张卡以 `E_VERSION_CONFLICT` 被拒，文档留着挪动之后的字节，卡仍开着）。
-/

/-! D32 城的工具读别楼的文件与 `cas:` 块，只经 runtime 的 `BoundReader`

理由：读界与 reserved subtree 的判定住 `runtime::tools::chosen_path`，`read` 与 `search` 用它；一件读字节的工具若在本 crate 自己判，就是那份判定的第二个权威，而且只判得了本楼（`is_within`），连接器存进 CAS 的录音与截图都读不到（§8-20）。被否决的做法：①保留「只收本楼」并为 `cas:` 另写一段（两套判定，一套跟着 `read` 变，一套不跟）；②在本 crate 复制 `admit` 与 `land`（同上，且链接的判定要拷两遍）。重开参数：要读的字节不在读界之内（例如人拖进来、只给这一次 run 的文件），那时它是一个新的入口，而不是放宽这扇门。
-/

/-! D42 提案的原文由 run 引出，不给字节区间；工具经 lane 的 relay 写行；收回由工具自己那本小账判（§8-30）

(a) 引文：模型读文件经 `read`，看到的是文字，不是字节偏移；它给出原话，区间由工具在那一版里找出，找不到或不止一处就拒，与 `edit` 的 `old` 同一种约定。被否决的做法：①收 `start`、`end` 字节偏移——模型要自己按编码数字节，数错一个就切进字符中间或切错句子；②收行号——要第二套「行怎样数」的规则，而 `documents` 的区间都按字节。

(b) 写行经 relay：relay 是 lane 唯一能写账本的门（D11），记账线程写下之后把这一行交给每个折叠，与 lane 写的 `tool_called` 同一条路。被否决的做法：把卡放进一张桌子、落地时由结算写——卡要等 run 结束才出现在人面前，而 `offer` 在行写下之前就把身份答给了模型。

(c) 收回的判定在工具里，开着与否的权威仍是 `Governance`：工具只判「这是不是我提出的、我收回过没有」，这两件只有它知道。被否决的做法：派活时把 `Governance.proposals` 拷进工具——这次 run 在那一刻还没有任何一张卡，拷来的只会是别人的，而且拷贝是折叠的第二份。重开参数：§8-30 当前状态那一条在真实的城里出现。
-/

/-! ### 接口仍写在 sprawling 规格里的模块

下面这些模块的接口与取舍今天写在 `crates/sprawling/Spec.lean` 的这几节里，按标签列出；`architecture.toml` 里它们的行指向本分部，这张表把读者带到那一节。它们搬进本 crate 的规格是 D15 记下的下一步。

| sprawling 的标签 | 模块 |
|---|---|
| §8-39 | `accounting::worker::workbench::tools`、`accounting::worker::workbench::tools::reading_room` |
| §8-87 | `accounting::worker::workbench::tools::kept`、`accounting::worker::workbench::tools::kept::tests` |
| §8-131 | `accounting::worker::workbench::tools::transcribe` |
| §8-132 | `accounting::worker::workbench::tools::playback`、`accounting::worker::workbench::tools::playback::tests` |
| §8-142 | `accounting::worker::workbench::tools::ocr` |
-/
