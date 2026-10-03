-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::answer::proposals

规定 `answer::proposals`（`crates/wire/src/` 下同名的文件）。一份文档上还开着的修改提案卡，与这份文档此刻的版本。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的帧形状由 Rust 的类型与 `crates/wire/tests/wire_contract.rs` 钉住的 wire schema（`wire::schema_hash`）守住。
-/

/-!
### 8-73 修改提案：`Query::Proposals` 与 `Command::DecideProposals`

```rust
// Query（紧接在 Range 之前）
Proposals(Address),                                   // 这份文档上还没决定的提案 → Answer::Proposals(Box<ProposalsAnswer>)
OpenProposals,                                        // 全城还开着的提案 → Answer::OpenProposals(OpenProposalsAnswer)
pub struct OpenProposalsAnswer { pub open: Vec<OfferedCard> }   // 最新提出的在前
pub struct OfferedCard {
    pub doc: Address,
    pub id: B3Hash,
    pub at: Option<TimeMs>,                           // 写下 proposal_offered 那一行的时刻
}
pub struct ProposalsAnswer {
    pub doc: Address,
    pub version: Option<B3Hash>,                      // 文档此刻的版本；缺失或读不了时为 None
    pub open: Vec<ProposalCard>,                      // 按提出的先后
}
pub struct ProposalCard {
    pub id: B3Hash,                                   // 提案身份（documents D13）
    pub run: RunId,                                   // 提出它的 run
    pub baseline: B3Hash,                             // 它所基于的版本；不等于 version 即已过期
    pub span: documents::Span,                        // 那一版里被提议替换的一段
    pub slices: Vec<documents::Slice>,                // 逐句的 diff：卡的正文
    #[serde(default)] pub offered: Option<Seq>,       // 提出它的那一行，在 run 的会话里（D44）
}
pub struct Slice { pub kind: SliceKind, pub text: String, pub lead: String, pub trail: String }   // documents 定义
pub enum SliceKind { Same, Delete, Insert }           // 线上 "same" | "delete" | "insert"
// Command（紧接在 PutRange 之后）
DecideProposals(ProposalDecisions)
pub struct ProposalDecisions { pub doc: Address, pub decisions: Vec<ProposalDecision>, pub idem: IdemKey }
pub struct ProposalDecision { pub proposal: B3Hash, pub verdicts: Vec<kernel::event::record::SliceVerdict> }
```

- **提案是一次 run 对一份文档的一处建议，卡的正文是逐句的 diff。** 一处提案带着它所基于的版本与那一版的一段字节，以及那一段的原文与提议的文本；城把两段文本切成句子、对齐，答成一串 `Slice`：`Same` 两边都有，`Delete` 只在原文里，`Insert` 只在提议里（documents D14、D15）。按序把非 `Insert` 的句子接起来就是原文，把非 `Delete` 的接起来就是提议，逐字节，空白也在内。
- **决定是逐句的。** 一句改动过的句子（`Delete` 或 `Insert`）可以被接受（`accept`）；插入的那一句还可以改后接受（`amend`，带人改过的文本）。没被点名的改动算拒绝：`verdicts` 为空就是整张拒绝。点名一句没改动的句子、一个不存在的序号、同一句两次、对删除的句子 `amend`，都拒 `E_INVALID_ARGS`（documents D16）。
- **一次决定可以带同一份文档上的几张卡，接受的部分作为一次保存落下**（documents D17）。只要有一张卡接受了什么，所有接受了什么的卡都必须基于文档此刻的版本，否则整次拒 `E_VERSION_CONFLICT`，什么都不写，卡都还开着——这是过期的提案；它们的区间互不重叠，否则拒 `E_INVALID_ARGS`。整张拒绝的卡不看基线：拒绝一张过期的卡不写文档。点名一张不在这份文档上、已经决定过、已经被收回、或根本没有的提案，拒 `E_INVALID_ARGS`；同一张卡点两次也拒。
- **落下的顺序**：有改动时先像 §8-72 一样在文档锁里换上新版本、写一行 `document_written`，再为每一张卡写一行 `proposal_decided`；都带这条命令的 `idem`，所以一次重发由 `commanding::entrance` 认出、不再写第二次。
- **提出与收回不在线上。** 提案是 run 提出的，收回也是它；人对一张卡只有决定。`proposal_offered` 与 `proposal_withdrawn` 由提出它的 run 经工作台的 `proposal` 工具写下（`crates/accounting/Spec.lean` §8-30，`crates/city/Spec.lean` §8-40 的当前状态）。
- **`version` 只答一次，卡上只带各自的基线。** 页面比较两者就知道哪张卡已经过期；卡本身不带「过期」这一格，因为过期是此刻的事实，一张卡在账上是什么不随文件而变。
- **全城一问只答名字与时刻。** `OpenProposals` 答每一张还开着的卡在哪份文档上、何时提出，最新的在前；卡的正文与文档此刻的版本仍由 `Proposals(doc)` 按文档答，没人打开的卡不为了被数一遍而切句。见 D15。
- **`WIRE_V` 不另进位**：`Proposals`、`OpenProposals`、`DecideProposals` 是新名字（D1）。
- 验收：accounting 的 `worker::commanding::tests::saving`——整张接受、改后接受、拒绝各落下它该落的字节与行；收回的卡不能再决定；同一个 `idem` 的重发不再写；一张卡被决定之后再决定被拒；重开的城从账本折回同样的卡；基线已动的卡被拒、拒绝它却可以；全城一问按最新在前列出开着的卡、各带提出时刻，收回的卡不在其中。
-/

/-! D15 全城开着的提案是一问，答文档与提出时刻，不答正文

**决定**：`Query::OpenProposals` 不带参数，答 `OpenProposalsAnswer { open: Vec<OfferedCard> }`，每张卡只带 `doc`、`id`、`at`，最新提出的在前。时刻由读面在折叠 `proposal_offered` 那一行时记下（`accounting::views::proposals::OfferTimes`），不进治理折叠：那个折叠也是工作者的，工作者不按时刻做决定。`at` 可缺，只为一张答者没见过其行的卡。

**理由**：页面原先只能按它自己听到的 `proposal_offered` 去问各份文档，于是页面打开之前提出的卡只在打开那份文档时才出现，信箱的「待决」说不全。一问列全城，信箱与它的计数读同一个答案。正文按文档问，因为切句与比对版本要读文件，而计数与列表不需要。

**被否**：①给 `Proposals` 的地址改成可缺——一个参数两种答案形状，页面要按形状分支；②全城一问连正文一起答——每次计数都要为每张卡读文件、切句；③在页面里继续折叠 `proposal_offered`——页面打开之前的卡永远缺。

**重开参数**：开着的卡多到一次答不完（数百张）时，加分页的 `before`。
-/

/-! D44 一张卡说出提出它的那一行，信件按这一行画发信 run 的相关对话

```rust
pub struct ProposalCard {
    // …既有字段…
    #[serde(default)]
    pub offered: Option<Seq>,   // 提出它的 `proposal_offered` 那一行的 seq，在 `run` 的会话里
}
```

**决定**：`Proposals(doc)` 作答时，读面在每张卡的 `run` 的会话窗口里（`LedgerAsk::records_of`，与 `Rounds` 同一个窗口）找身份等于这张卡的 `proposal_offered`，填它的 `seq`；一个 run 的窗口只读一次，同一个 run 的几张卡共用。找不到（窗口外、行读不回）为 `None`。信件的「对话」读法（client D90）按它只画提出这张卡的那一回合与前后各一回合，并给出整段会话的链接；`None` 时画整段，与以前一样。

**理由**：A25 要的是发信 run「相关的」对话：一段长会话里，这张卡是哪一回合提出的、提出之前读了什么、之后说了什么。整段画出来，人要自己在几十个回合里找那一处。行的 seq 是回合与调用共用的坐标（`Turn.opened`、`Call.at`），页面不需要第二种对齐法。

**被否**：①在 `OfferTimes` 里同时记 seq：那张表随快照编码，改它的形状要动折叠规则的版本，而 seq 只有打开一封信时才用得上；②页面按调用参数里的文档路径去对：同一回合里对同一份文档的两次提案分不开，参数被窗口截断时更对不上。

**重开参数**：一张卡常常在离它的会话窗口（`HISTORY_MAX`）之外被打开时，给索引加一张按提案身份的表。

**三个平台**：只读账本，Windows、macOS、Linux 相同。与本波其他改形同一次 `WIRE_V` 进位（D22）。
-/
