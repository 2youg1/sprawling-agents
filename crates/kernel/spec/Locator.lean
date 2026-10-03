-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# kernel::locator

规定 `kernel::locator`（`crates/kernel/src/locator.rs`）：`cas:`／`file:` 文法、`B3Hash` 与 `GitOid`、它们的 serde。本文件是 `crates/kernel/Spec.lean` 的一个分部；下面每一节保留它在 kernel 规格里的标签 §8-n，别处引作 `crates/kernel/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `kernel::locator::tests` 守住。
-/

/-!
### 8-3 kernel::locator

```rust
pub struct B3Hash([u8; 32]);            // 全库唯一哈希值类型；hex64 小写呈现
impl B3Hash { pub const fn from_bytes(bytes: [u8; 32]) -> Self;  pub fn as_bytes(&self) -> &[u8; 32];
              pub fn digest(bytes: &[u8]) -> Self; }   // 链之外的全部内容哈希只经 digest 产出
pub struct GitOid([u8; 20]);            // 40 位十六进制小写；serde 与 B3Hash 同形
impl GitOid { pub fn parse(raw: &str) -> Option<Self> }   // 长度不对即拒，恒不补零
pub enum Range { Lines { from: u64, to: u64 },   // 1 起、闭区间
                 Bytes { from: u64, to: u64 } }  // 0 起、闭区间（HTTP Range 先例）
pub enum Locator {
    Cas  { hash: B3Hash, range: Option<Range> },
    File { address: Address, oid: GitOid, range: Option<Range> },
// `GitOid::parse` 是公开的：checkpoint 身份向两个方向旅行——wire 反序列化一个，
// 而一个被展示了 checkpoint 句子的客户端要把它变回 oid 才能对它动作。不开这个门，
// 调用方就自己解十六进制，而那正是 `Deserialize` 旁边那句「形状权威留在这里」要防的事。
}
impl Locator {
    /// Fail-closed: anything not exactly the grammar is E_LOCATOR_INVALID.
    /// Unknown scheme or algorithm tag is an error, never a fallback path.
    pub fn parse(raw: &str) -> Result<Self, AxError>;
    /// The whole object behind a digest the caller already holds; no range.
    pub const fn cas(hash: B3Hash) -> Self;
}
impl fmt::Display for Locator { /* 规范拼写往返：parse(x).to_string() == 规范形 */ }
```

- 文法：`cas:b3-<hex64>[#(L|B)<a>-<b>]`｜`file:<address>@<hex40>[#(L|B)<a>-<b>]`。`file:` 以最后一个 `@` 切分（address 段内允许 `@`，oid 恒不含）。
- **规范回声断言**：解析成功后另断言 `Display(结果) == 原串`，不等即 `E_LOCATOR_INVALID`（recovery 给出规范拼写）——一条规则封死大写 hex、前导零、`+` 号等全部非规范变体。
- 十六进制恒小写（规范字节唯一化）；大写拒。`b3-` 外的算法标签拒（对扩展开放：新标签＝新 variant，旧解析不宽容）。
- `SecretRef`（`secret:`）恒不入本文法——`secret:` 前缀命中即 `E_LOCATOR_INVALID`，两套解析器分立（类型层理由）。
- serde：字符串形（Display/parse 往返）。`B3Hash` 与 `GitOid` 的 serde 在人读的格式里是同一种字符串形、在二进制格式里写字节本身，见 §8-84。
- `Locator::cas(hash)` 是手里已有 `B3Hash`（通常是 `Cas::put` 的回答）时的唯一构造法：一个 digest 本就合文法，拼成文本再 `parse` 回来只是多了一次分配、一次解析，外加一个恒不发生的错误分支，调用方还得为它写 `?`。
- `B3Hash::from_bytes([u8;32])`／`to_hex()`；`Range` 构造校验 `from<=to`（Lines 另 `from>=1`）。
- `B3Hash::digest(bytes: &[u8]) -> B3Hash`（blake3 直算）：全库内容哈希的唯一产地——prefix 分段哈希与 stall 指纹均经此，不在 kernel 外直呼 blake3（一个哈希一个家）。`chain_hash` 保留为链语义专名（内部改经 digest）。
-/

/-!
### 8-84 `B3Hash` 与 `GitOid` 的 serde：人读的格式写十六进制，二进制格式写字节本身（`kernel::locator`，形状 2 值类型）

```rust
// kernel::locator —— shape: value
impl Serialize for B3Hash { /* is_human_readable：64 位小写十六进制；否则 [u8; 32] 的 32 个字节 */ }
impl Serialize for GitOid { /* is_human_readable：40 位小写十六进制；否则 [u8; 20] 的 20 个字节 */ }
impl<'de> Deserialize<'de> for B3Hash { /* 人读的格式只收规范十六进制、长度不对即拒；二进制格式读回 32 个字节 */ }
impl<'de> Deserialize<'de> for GitOid { /* 同上，20 个字节 */ }
```

- **一条规则，两种摘要。** 两种摘要的 serde 问同一个问题 `is_human_readable()`，写与读各经一个私有函数（`write_digest`、`read_digest`），两个类型只交出各自的字节与拼写。账本行、线上帧、`serde_json::Value` 都是人读的格式，拼写照旧是小写十六进制，读回只收 `decode_hex_fixed` 认的那一种拼写；视图与 Standing 的快照经 postcard 编码，是二进制格式，写 N 个字节、读回 N 个字节，不分配字符串、不解十六进制。`RunId` 早已这样做（`kernel::event::identity` 的 `read_run_id`：uuid 在二进制格式里写 16 个字节）。
- **什么不变。** 账本的规范字节（`canonical_line`）、线上的拼写、`golden-p0`／`golden-s1`、wire 的两份 golden 都不变：它们全经 `serde_json` 写。变的只有快照格式；快照格式由 `VIEWS_FOLD_RULES` 与 `STANDING_FOLD_RULES` 的夹具摘要钉住（`crates/accounting/Spec.lean` §8-24），所以旧快照按「版本不符」从创世折一次，没有第二种读法。
- **为什么。** 40 万行夹具城开城时视图快照解码约 135 ms，其中约 105 ms 是提交折叠里 64,000 个 oid 的十六进制：每个 oid 在 `commits`、`commit_seqs`、`last_commit` 里各解一次（`crates/sprawling/Spec.lean` §8-144）。改后的读数在 `crates/sprawling/Spec.lean` §8-154。
- **被否：快照里每个摘要字段各标一个 `#[serde(with = …)]`。** 摘要散在视图的十几个字段与 wire 的类型里（例如 `wire::CommitAt`），逐个标注就是同一条规则的十几份拼写；漏标一处既不报错，也看不出慢，只是悄悄留着十六进制。serde 的 `is_human_readable` 正是为这种区分设的。**被否：给快照另起一个 `GitOidBytes` 新类型。** 视图里存的 `GitOid` 也是线上答复里的那个值，换类型就要在折叠与作答之间来回转换。
-/

/-! D16 摘要在二进制格式里写字节，在人读的格式里写十六进制

**决定**：`B3Hash`、`GitOid` 的 `Serialize` 与 `Deserialize` 按 `is_human_readable()` 分两臂（§8-84）。

**理由**：十六进制拼写是给人和线上读者的；二进制格式只由这个二进制自己读回，十六进制在那里只让一个摘要长一倍，读一次多一次解码。规则住在类型旁边，所以视图、Standing 与以后任何二进制快照都不必知道它。

**被否**：①快照字段逐个 `#[serde(with)]`（§8-84）；②全城改用字节拼写——账本行与线上帧是 JSON，十六进制拼写是它们的契约，改它要动账本的规范字节与 `WIRE_V`。

**重开参数**：出现第二种二进制格式、它的读者要另一种形状（例如带长度前缀、或要人能读的转储）时。
-/

/-! D27 `read` 与 `edit` 答出的版本就是 `plan finish` 收的那个形式

**决定**：`read` 的答复带这份文件的完整版本，`edit` 的 `E_VERSION_CONFLICT` 也带完整版本，两者都是这份内容完整的 `cas:b3-<hex64>`（`Locator::cas(B3Hash::digest(bytes))` 的规范拼写），模型把它原样交给 `plan finish` 即可，不补不删；`plan finish` 照旧只收本节文法的两种完整形式。工作树里的文件不答 git oid：`file:<address>@<oid>` 的 oid 是一次提交（`storage::blob_at` 按提交取文件），一份尚未落进检查点的文件没有这样一个提交。版本由 `Locator` 的 `Display` 拼出，文法仍只有这一个家。工具答复是自由的 `Payload`，所以没有新的事件种类，也不改信封。

**理由**：测试城里 `plan finish` 四次被拒，因为没有 `exec` 的 run 拿不到 40 位 oid，而 `edit` 只给 16 位前缀（roadmap F4）。让工具给出完整形式，文法仍只有一个家；让 `plan finish` 接受前缀，就要城去解析前缀，前缀在大仓库里不唯一。

**被否**：`plan finish` 收 16 位前缀、由城补全：文法多一种形式，补全还会遇到歧义。

**重开参数**：工作树换成 SHA-256 对象格式时（§3 第 3 条），两种答复一起改长度。
-/
