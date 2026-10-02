-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# accounting::worker::commanding::saving

规定 `crates/accounting/src/worker/commanding/saving.rs`、`crates/accounting/src/views/proposals.rs` 与 `crates/accounting/src/views/commits.rs` 读提交说明的那一步。本文件是 `crates/accounting/Spec.lean` 的一个分部；下面每一节保留它在 accounting 规格里的标签 §8-n，别处引作 `crates/accounting/Spec.lean §8-n`，决定引作 `accounting D<n>`。
-/

/-!
### 8-22 保存、修改提案与提交说明（`accounting::worker::commanding::saving`，形状 4 adapter；`accounting::views::proposals`，形状 7 投影）

```rust
// accounting::worker::commanding::saving（worker 线程）
impl RunWorker {
    pub(in crate::worker) fn put_range(&mut self, write: &wire::RangeWrite) -> Result<(), AxError>;
    pub(in crate::worker) fn decide_proposals(&mut self, decisions: &wire::ProposalDecisions) -> Result<(), AxError>;
}
// accounting::views::proposals（视图与 worker 共用的折叠，住 Governance 里）
pub struct Proposals { /* 开着的卡：身份 ↦ documents::Offer；处理过的卡：身份 ↦ 怎样处理的 */ }
impl Proposals {
    pub(crate) fn offered(&mut self, run: RunId, payload: &Payload) -> Result<(), AxError>;   // proposal_offered
    pub(crate) fn decided(&mut self, payload: &Payload) -> Result<(), AxError>;               // proposal_decided
    pub(crate) fn withdrawn(&mut self, payload: &Payload) -> Result<(), AxError>;             // proposal_withdrawn
    pub(crate) fn open_on(&self, doc: &Address, id: &B3Hash) -> Result<&documents::Offer, AxError>;   // 不开着、不在这份文档上 → E_INVALID_ARGS
    pub(crate) fn all_open_on(&self, doc: &Address) -> Vec<documents::Offer>;                // 按提出的先后
}
pub(crate) struct OfferTimes(BTreeMap<B3Hash, TimeMs>);   // 每张卡第一次提出那一行的时刻，住 Views、在 Governance 旁
impl Views { pub(in crate::views) fn open_proposals_answer(&self) -> wire::OpenProposalsAnswer; }   // 锁内，不读盘
pub(super) fn proposals_answer(city_root: &Path, doc: Address, open: Vec<documents::Offer>) -> wire::Answer;   // 锁外
// accounting::views::commits（锁外，与 parents 同一刻）
fn give_messages(city_root: &Path, commits: &mut [wire::CommitAnswer]);
```

- **一次保存是三步，都在 worker 线程上。** `put_range` 先拒保留子树（`Address::is_reserved`，`E_OUTSIDE_WRITE_DOMAIN`），再经 `city::revise_document`（`crates/city/Spec.lean` §8-40）在这份文档的锁里读出此刻的字节、交给 `documents::save` 判定并换上，最后写一行 `document_written`（`crates/kernel/Spec.lean` §8-83），它带着这条命令的 `idem`（`commanding::entrance::stamped`）。被拒的保存什么也不写，账上没有它。
- **决定修改提案也是一次保存。** `decide_proposals` 从 worker 的 `Governance.proposals` 找出点名的每一张卡（不在这份文档上、已经处理过、没有的都拒 `E_INVALID_ARGS`），经同一扇 `revise` 在锁里交给 `documents::decide`；有改动时写一行 `document_written`，然后每张卡一行 `proposal_decided`。卡的状态不在这里改：worker 写下的每一行都经 `RunWorker::absorb` 交给同一个折叠，重开的城读账本得到同一个答案。
- **提案的折叠住 `Governance`，视图与 worker 各持一份、折法一处**（D34）。`proposal_offered` 与 `proposal_withdrawn` 由 run 经工作台的 `proposal` 工具写下（§8-30）。`proposal_offered` 经 `documents::Offer::of` 读成一张卡，身份由它算出；读不出的一行（区间颠倒、超长）与别的读不出的治理行一样拒绝，让这座城停在打开那一步，而不是少一张人等着决定的卡（`crates/sprawling/Spec.lean` §8-74 的同一条理由）。`proposal_decided` 与 `proposal_withdrawn` 把卡从开着挪到处理过；处理过的卡只记身份与怎样处理的，不留原文与提议。
- **全城一问：`Query::OpenProposals` 答出这座城所有开着的卡**（`crates/wire/Spec.lean` D15），每张卡带文档、身份与提出的时刻，最新提出的在前；只有名字与时刻，所以在锁内答完、不碰文件。提出的时刻由视图在治理折叠旁另折一份（`OfferTimes`），不进折叠本身：那个折叠也是 worker 的，worker 写下的行交给它时不带时刻，它也不凭时刻判任何事。`OfferTimes` 不修剪，与处理过的卡同一增长类。信箱因此列出页面打开之前就提出的卡。
- **`Query::Proposals` 在锁内拷出这份文档上开着的卡，锁外读盘。** 文件此刻的版本要读一次全部字节（`B3Hash::digest`），所以与 `Document` 一样在快照放开之后做；卡的句子由 `Offer::review` 在那时切。文件缺失或读不了时 `version` 为 `None`，卡照答。
- **提交说明读自 git，与父提交同一刻。** `CommitsAsk::read` 在快照放开之后先经 `storage::parents_of` 读父提交，再经 `give_messages` 读说明：一次打开仓库（`git2::Repository::open`），每个 oid 一次 `find_commit`，`Commit::message` 不是 UTF-8 或对象不在时为 `None`（`crates/wire/Spec.lean` §8-54）。
- 验收：`worker::commanding::tests::saving`（`crates/wire/Spec.lean` §8-72、§8-73 列的那几条；`Query::Proposals` 的答复——开着的卡按提出的先后、文件此刻的版本——在其中经 `views::ask` 读）；`worker::commanding::tests::saving::the_city_lists_its_open_cards_newest_first_with_their_offer_time`；`views::commits::tests::a_page_of_commits_carries_each_ones_message_from_git`。
-/

/-! D34 修改提案的折叠住 Governance，提交说明经 git2 直接读

**(a) 修改提案的折叠住 `views::Governance`，与待答的审批同一个值。** 理由：一张提案卡与一条审批同是「等人决定的事」，worker 判一次决定要的状态与页面画卡要的状态是同一个，`Governance` 正是「一份定义、两处持有、`what_a_worker_holds_is_what_a_restart_rebuilds` 判它们相等」的那个值；放进去之后，快照、重开、worker 写下一行就折一行，都不需要新的接线。被否决的做法：①worker 另折一份 `Standing` 字段、视图另折一份——两份折法；②决定时按身份回账本找那一行——要一个按内容找行的索引，而且「已经处理过」还要再扫一遍。**(b) 提交说明在本 crate 用 `git2` 直接读，与 `storage::parents_of` 各开一次仓库。** 理由：本轮 storage 的公开契约不改（它的规格在迁移），而本 crate 已经为 playback 链接 `git2`；读说明只是 `find_commit` 之后的一个字段，一页至多 `HISTORY_MAX` 个提交多开一次仓库。被否决的做法：①在本 crate 里连父提交一起读、不再调 `parents_of`——两处各有一份「父提交怎样读」，`parents_of` 留下来没有调用方；②把说明写进账本——提交对象就是它的权威，账本记的是 oid。重开参数：storage 的契约下一次能动时，`parents_of` 换成一次读出父提交与说明的读者，本 crate 的 `give_messages` 删去。
-/
