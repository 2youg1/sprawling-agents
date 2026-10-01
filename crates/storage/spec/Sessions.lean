-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# storage::sessions

规定 `sessions`、`sessions::from_ledger`（`crates/storage/src/` 下同名的文件）。每个 room 一份可弃的投影文件，在 Ledger 落盘之后写。本文件是 `crates/storage/Spec.lean` 的一个分部；下面每一节保留它在 storage 规格里的标签 §8-n，别处引作 `crates/storage/Spec.lean §8-n`，决定引作 `storage D<n>`。
-/

/-!
### 8-24 `storage::sessions`：账本投影到各楼的 sessions（形状 7 投影）

```rust
pub struct Sessions { /* layout、ledger、vfs、open、first_seen —— 私有 */ }
impl Sessions {
    pub(crate) fn for_ledger(ledger_dir: &Path) -> Option<Sessions>;   // 非城账本 → None
    pub fn absorb(&mut self, record: &EventRecord) -> Result<(), StorageError>;
}
impl JsonlLedger {
    /// 把切片交出去：此后这个写者不再写切片，由拿到它的线程按账本序逐条 absorb。非城账本、已交出过的答 None。
    pub fn hand_off_session_slices(&mut self) -> Option<Sessions>;
}
pub(crate) const SLICE_MAGIC: &str = "slices v1";
```

**账本不拆链。** 一条链、一个写者、一本完整历史（`docs/glossary.md` 的 one Ledger 与 one writer）一个字不改；工作区里出现的是一份**从账本投影出来、可删可重建的切片**：一个 room 一个文件，落在它所属 Building 的 `.sprawling/sessions/` 下，地址的嵌套就是文件的嵌套。**城自己的那条记录（地址即城名）不落任何切片**：那个地址是城，不是城里的 Building，给它落一份就会在城根里造一个与城同名的目录（kernel-SPEC 8-56 的 `city_address`）。路径由本模块的私有函数 `session_slice` 从 `CityLayout::root` 推出，目录名 `SESSIONS_DIR` 也是私有常量；crate 内唯一对外的是谓词 `pub(crate) fn is_session_projection(relative: &Path) -> bool`，供检查点判断一条路径是不是切片。可见性就是这句话的守卫：别的模块写不出切片的路径，也就写不出切片的读者。被击败的替代是把路径留在 `kernel::layout` 再用文本扫描拦住别的调用点——扫描只认拼写，绕开一次别名就放行。

**一条线程写，而且写在账本落盘之后。** 默认的入口是 `JsonlLedger::append_all`：一段 wave 全部 `sync_data` 之后才逐条 `absorb`。服务中的城在写者起好之后经 `hand_off_session_slices` 把切片交给视图线程（sprawling-SPEC 8-123）：视图线程收到的每条已提交记录，折完、广播之后再 `absorb`，所以缺文件时的整段重建与 `first_seen` 的那一次整段读取都不再占记账线程。交出之后写者一条也不再写，交出发生在写者线程上、观察者接上之前，两条线程之间没有一条记录会落空或写两次；切片因此比账本晚一个视图批次，这是投影可以有的落后。投影写失败不改变账本的返回值——历史已经在盘上，可弃物不得让它的调用方吃到一次假失败——但拒绝被报出（一行 `eprintln!`）而不是被吞掉。**投影永不进检查点。** `checkpoint::stage_scopes` 跳过任何 `sessions` 下的路径（`is_session_projection`）：它还在被账务线程追加，而 git 的暂存要读它以为已经知道的文件；一个仍在长的文件会把整波拒成 `E_WORKTREE_BUSY`（8-8）。可弃物因此从不进历史，也不进任何 diff 的读者面。

**头部是唯一的疑点。** 每份切片首行是 `slices v1 <first_seq>`，即该文件第一条记录的 seq。魔数不符、或首行 seq 与头部不符 → 整份重建：扫账本、按地址过滤、按账本序写下，不写迁移代码。文件不存在时先问常驻的 `first_seen`（地址 → 账本里它的第一条 seq）：第一条就是正在 absorb 的这条，账本里没有更早的行，切片就从这一条落下，一个段也不读；否则按上一句整份重建。进程重启后接上已有文件时，先校验头部与内容，再从账本把漏掉的尾巴补齐；段名带首 seq，故不可能含新记录的段不打开。断在半行的尾巴先截掉再续，与账本自己的 tail recovery 同一条读法。

**新 room 不扫段。** `first_seen` 在本进程第一次遇到缺文件的切片时从各段折叠一次，此后由每次 `absorb` 增量写入，所以派往新 room 的活只付一次 `exists` 与一次追加，而不是每个新 room 都把整本账本读一遍再解析——那样的答案永远是「除了这一条什么都没有」。被否：打开账本时就折叠——每个打开城账本的进程（包括只追加一行的命令）都要多读一遍全部段，而多数进程从不遇到缺文件的切片。剩下的一次折叠仍是整段读取，把它搬到 lane 上或并进启动时已有的 verify 那一遍，是 §3 的开放项。

**重建逐字节相同。** 每一行都是账本 `canonical_line` 的副本，头部由内容决定（首条记录的 seq），所以删掉整个 `sessions/` 再放一遍得到同样的字节——`deleting_the_sessions_directory_and_replaying_the_ledger_restores_the_bytes` 钉住它。

**产品不读它做判断。** 它只给人的眼睛与城外 agent 的 `read`；`runtime` 的 `read` 工具照旧拒绝 `.sprawling`，城里别的模块连它的路径都拼不出（路径推导是 `storage::sessions` 的私有项）。

**自带的 `RealFs`，不借账本那条缝。** `RealFs` 同一时刻只持一个追加句柄，写投影会把账本的热句柄挤掉；投影可弃，不该让主干为它付句柄开销。代价是投影不参与 `FaultFs` 的断电模型：断电后它可能落后，下一次 attach 补齐，这正是它可重建的含义。

**没有第二份 session 索引。** 「全城有哪些 session」走目录遍历；切片在追加时写，写者当场就知道 room 地址，索引只会在两者之间造出一个可失效的家。
-/
