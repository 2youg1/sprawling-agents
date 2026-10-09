-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::reception

规定 `reception`、`reception::tests`、`answer::history`（`crates/wire/src/` 下同名的文件）。能不能绑、这个对端能不能录凭证、能不能欢迎它、它的帧此刻是什么意思，以及丢帧之后的区间。一个请求进不进得了门住在 `spec/Reception/Entry.lean`，浏览器的配对与会话住在 `spec/Reception/Pairing.lean`。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-41 丢帧可见、区间补拉、每一面的凭据

三件事写在一节里，因为它们是同一条链上的三处：慢会话丢掉的记录要说出来、丢了之后要能把那一段要回来、以及每一面都要凭据。

#### 一 丢掉的记录要说出来：`ServerFrame::Lagged { from, to }`

```rust
pub struct Lagged { pub from: Seq, pub to: Seq }   // 两端都含
pub enum ServerFrame { …, Lagged(Lagged) }
```

**两端都取自会话自己数得出的记录，不取自广播报的那个数。** `RecvError::Lagged(u64)` 只说跳过了多少条，一个端点也不给；一个只拿到跳过量的人无法把丢掉的那一段要回来。所以会话新增一份状态——**上一条已发的 `EventRecord.seq`**——`from` 是它之后的第一条；`to` 是**恢复后首条回退一位**，因为「这一段到哪结束」只有在下一条记录到达时才成为事实。同一个值同时装着「已发到哪」和「还欠不欠一段范围」，故它是 `Stream::{Even, Owed}` 而不是一个 bool 加一个 `Option<Seq>`。

**`Even` 里的 seq 断口同样是一段欠账。** 一条记录可能根本没进广播（§8-47：拼不出帧），这时订阅不报 `Lagged`，会话仍是 `Even`；已发过记录的 `Even(Some(last))` 遇到 `next > last + 1`，照样先发 `Lagged { last + 1, next - 1 }`。判定只有 `decide_lag` 一处；`Even(None)` 不算——会话的视图从欢迎开始，欢迎之前的记录是问题而不是欠账。

**四路语义不同，故四路分开陈述**（四路指一个会话的四个接收臂：自己的拒绝、事件、增量、日志）：

| 臂 | 缓冲 | 拉下时 | 原因 |
|---|---|---|---|
| 自己的拒绝 | unbounded mpsc | 不会发生 | 拒绝的速率是一个人犯错的速率，不是城的速率，且一条都不能丢 |
| 事件 | broadcast 1024 | 发 `Lagged { from, to }` | 记录在账本里，那一段可以要回来 |
| 增量 | broadcast 256 | 一言不发 | 增量不落账本、不可重放，为它报一个区间等于报一段不存在的记录；settled 文本随后作为记录到达 |
| 日志 | broadcast 深度见 `serving::journal` | 一言不发 | 日志是诊断不是历史，且它携的位置是账本的位置而不是自己的序号，所以任何区间都放不回读者原来的地方 |

**`Closed` 只有一种语义：会话结束。** 一个已关闭的 broadcast 接收端**立刻**返回 `Closed`，把它当空转处理的臂在 `select!` 里会变成热转。发端全没了就是城要走了，四条臂一律 `return`。

**「重连从账本补」只在事件一路成立**（`Delta` 的 doc 明写 never written to the Ledger, cannot be replayed，日志同理）；另两路的静默在表里各自有理由。

**还没被欢迎的会话不报区间。** 它没有给对端看过任何一条记录，它的视角从 welcome 开始——而欢迎之前发生的事，页面要的话是一个提问（`Query::History`），不是一帧。

**客户端的闭包臂先封掉。** `client/src/core/link.ts` 末尾原有一个 `return [link, { kind: "nothing" }]`：**忘记处理一帧因此是零编译错误的静默失败**，而新帧刚好要落在这个位置上。现在那条路径是一个参数类型为 `never` 的函数，穷尽时它的实参收敛成 `never`、漏一臂时保持原类型——于是「忘了处理」是编译错误而不是沉默。

#### 二 丢了之后要能要回来：`Query::HistoryRange { from, to, limit }`

`Query::History` 只有 `before: Option<Seq>` 的向后翻页，回答的是「这之前发生了什么」；区间两头都有，从尾巴走到区间的近端要为此付掉中间每一条记录的代价。故新增一条查询：**答面是 `HistoryRangeAnswer { from, to, records, next }`，不是复用 `HistoryAnswer`。**

- **两个端点回声**：提问没有游标可握（两个 seq 来自一帧，那一帧早已被丢掉），所以答必须说出自己切的是哪一段——否则同一页同时开着记录视图与补拉时，两条答无法分辨。
- **`next` 而不是 `more`**：服务端知道账本里这一段的下一条在哪，答一个游标就是把它已经知道的告诉你；客户端因此不做序号算术。
- **`HISTORY_MAX = 500` 由服务端钳，客户端要不到更多**；一次补拉是多页，`next` 说下一页从哪起。
- **走不到的那一步结束区间而不是指一个越界的游标**：一条读不回来的行，或者一条越过账本尾部的区间，都让 `next` 是 `None`。反过来会让客户端对着同一个 `from` 永远问下去。
- **`QUERY_NAMES` 从 33 增到 34**，schema 哈希随之变（33→34 名字表长了一项，故哈希无论如何都要动，版本因此同集进位）。旧页面在握手期被明确拒绝，这正是该机制存在的理由。
- **实现住 `sprawling::views::answering::history_range`**，与 `history` 并排、共用同一个 `LedgerIndex::reader`；本 crate 不读盘。

**客户端的补拉走的是「一次一页、答的游标推进」，不是「一次一个可替换的窗口」**：区间排成队列（两次丢失是两个区间），队首一页在飞，答回来了才前进或出队。可替换的窗口在「一页在飞时又来一次丢失」这一刻会丢掉中间那一段。补到的记录**像别的记录一样折进 belief**（`store.apply`），且**不使任何已答的问题变旧**：它们是旧记录而不是新闻，为它们把所有答案标旧等于把整座城重问一遍。视图要不要直接画这一段，是前端道的事（本波只有 socket 半边问它）。

**客户端的 socket 半边**因此是本层唯一会「问一句、等一句、再问」的地方：`client/src/core/socket.ts` 的 `askGap()` 与 `filled()` 两小段，判断仍全在 `link.ts`。

#### 三 每一面都要凭据：把凭据装进面里

```rust
pub enum BindFace {
    Loopback { key: B3Hash },   // 只从回环可达，照样要钥匙
    Exposed { key: B3Hash },    // 能从别处可达
}
pub fn decide_bind(addr: &SocketAddr, key: Option<B3Hash>) -> BindVerdict;  // 没有钥匙即拒，两面都一样
impl BindFace { pub fn key(&self) -> &B3Hash; }
```

**强制点：`decide_bind` 是 `BindFace` 的唯一生产者，`bind` 是它的唯一调用者，`serve` 把 `Bound` 里的面经 `router` 交给壳。** 壳（`ShellState.face`）此后是「这一面要求什么」的唯一读者：`decide_frame` 与各扇门都经 `reception::Keys` 读它。

- **两个臂里的钥匙都不是 `Option`**：「回环而什么都不要」与「暴露着却不要求任何东西」都是类型上不存在的状态。回环面也要凭据，理由是 wire D54：同一台机器上的另一个用户、另一个端口上的页面、居民的工具都能到达回环端口，回环本身不是凭据。
- **钥匙是 `bind` 的入参**：它是这次服务的native key（`crates/sprawling/spec/Keying.lean` §8-22：配置过就采纳，否则当场铸）。判决只在 `bind` 里产生一次，面随 `Bound` 走到壳里，故这不是同一个事实的两个家。
- **拒绝只剩一格**：调用方没给钥匙。产品二进制总会给一把；拒绝臂留给第三方 embedder，回 `E_CONFIG_INVALID` 与恢复办法，在套接字存在之前。
- **面之外还认会话令牌**：浏览器配对之后经挑战签名换来的会话令牌（§8-93）是动态的，住在本地门的状态里；`Keys { face, sessions, now }` 把两者合成一次判定（令牌认不认看此刻，§8-93），`Keys::pairing` 是 hello 与每个 POST 共用的那一问。

#### 四 `/enroll` 等待里的第四个静默臂

`/enroll` 等的是 `secret_captured`：这条记录在广播上，于是这个等待也可能被流拉下，而**被跳过的记录不会再发一次**。旧代码对 `Lagged` 一言不发、继续等，最后由超时给出一个 202，正文写着「它还没答复」——一句可能不真的话。现在等待的结局是一个枚举：`Settled`／`Overtaken`／`Ended`，被拉下时当场结束等待，202 的正文写成它真正的样子（「城的流走过了这次请求」），因为再等下去也等不到那条记录。

#### 五 WebTransport 的重开条件

**不因「需要第二种协议」回来。** 重开条件是**两个都要实测成立**：①这座城真的跑在回环之外；②队头阻塞实测存在（即一条大帧让同一条连接上的小帧延迟到人能察觉）。理由：`ws://` 只到回环、暴露面经终止器是已记录的部署判断（根 `Cargo.toml`，`connect` 不带 TLS），而换成 QUIC 会同时改掉握手、帧界与资产通路，代价落在每一次改 wire 时；收益只在②成立时出现。`Carrier` 这个抽象不在树里：本 crate 只有一个实现，抽出它就是穿透层。
-/

namespace Wire.Reception

/-- 一个监听地址从哪里够得到：只从回环，或从城所在的机器之外（`SocketAddr::ip().is_loopback()`）。 -/
inductive Address where
  | loopback
  | beyond
  deriving DecidableEq, Repr

/-- 绑定判定给出的面，携着它向每个来者索要的native key摘要。两臂的钥匙都不是 `Option`：「什么都不要」的面在类型上不存在（§8-41 第三件）。 -/
inductive BindFace (Digest : Type) where
  | Loopback (key : Digest)
  | Exposed (key : Digest)
  deriving DecidableEq, Repr

/-- `BindFace::key`：这一面向每个来者索要的摘要。 -/
def BindFace.key {Digest : Type} : BindFace Digest → Digest
  | .Loopback key => key
  | .Exposed key => key

/-- 绑定判定的结论；拒绝时的码是 `E_CONFIG_INVALID`。 -/
inductive BindVerdict (Digest : Type) where
  | Serve (face : BindFace Digest)
  | Refuse
  deriving DecidableEq, Repr

/-- `decide_bind`：没有钥匙就拒，有钥匙就按地址给出面。 -/
def decideBind {Digest : Type} : Address → Option Digest → BindVerdict Digest
  | _, none => .Refuse
  | .loopback, some key => .Serve (.Loopback key)
  | .beyond, some key => .Serve (.Exposed key)

/-- **拒绝恰在没有钥匙时，回环与暴露一样。** -/
theorem decideBind_refuses_exactly_without_a_key {Digest : Type}
    (address : Address) (key : Option Digest) :
    decideBind address key = .Refuse ↔ key = none := by
  cases address <;> cases key <;> simp [decideBind]

/-- **一个面索要的恰是给它的那把钥匙**：判定把凭据装进面里，面之外没有第二处读它。 -/
theorem a_served_face_demands_the_given_key {Digest : Type}
    (address : Address) (key : Option Digest) (face : BindFace Digest)
    (served : decideBind address key = .Serve face) : some face.key = key := by
  cases address <;> cases key <;> simp [decideBind] at served <;> subst served <;> rfl

/-- 区间的近端：上一条已发之后的那一条；什么都没发过时是 `Seq::FIRST`，即 1。 -/
def lagStart : Option Nat → Nat
  | some last => last + 1
  | none => 1

/-- `decide_lag`：一次丢失之后会话要说出的区间。`delivered` 是这个会话上一条发出的记录，`next` 是丢失之后到达的第一条；两端都含。Rust 里 `checked_add` 溢出时答 `None`，产出 `next` 的账本到不了那一步，模型用 `Nat` 不写这一支。 -/
def decideLag (delivered : Option Nat) (next : Nat) : Option (Nat × Nat) :=
  if 1 ≤ next ∧ lagStart delivered ≤ next - 1 then some (lagStart delivered, next - 1) else none

/-- 会话对页面欠着什么（`reception::Stream`）：`Even` 是页面拿着这个会话发出的每一条，`Owed` 是流跳过了一些、下一条到达时说出区间；两者都带上一条已发的记录。 -/
inductive Stream where
  | Even (delivered : Option Nat)
  | Owed (delivered : Option Nat)
  deriving DecidableEq, Repr

/-- `Stream::delivered`。 -/
def Stream.delivered : Stream → Option Nat
  | .Even delivered => delivered
  | .Owed delivered => delivered

/-- `Stream::skipped`：还没被欢迎的会话不欠任何区间。 -/
def Stream.skipped (stream : Stream) (welcomed : Bool) : Stream :=
  if welcomed then .Owed stream.delivered else stream

/-- `Stream::before`：发出 `next` 之前要说什么，以及发出之后的状态。 -/
def Stream.before : Stream → Nat → Option (Nat × Nat) × Stream
  | .Even none, next => (none, .Even (some next))
  | .Even (some last), next => (decideLag (some last) next, .Even (some next))
  | .Owed delivered, next => (decideLag delivered next, .Even (some next))

/-- **区间恰好补上断口**：从上一条已发之后的第一条起，到 `next` 的前一条止，且不空。 -/
theorem a_lagged_range_fills_the_gap (last next first to : Nat)
    (lagged : decideLag (some last) next = some (first, to)) :
    first = last + 1 ∧ to + 1 = next ∧ first ≤ to := by
  unfold decideLag at lagged
  by_cases bounds : 1 ≤ next ∧ lagStart (some last) ≤ next - 1
  · rw [ite_eq_left bounds] at lagged
    simp only [lagStart, Option.some.injEq, Prod.mk.injEq] at lagged bounds
    omega
  · rw [ite_eq_right bounds] at lagged
    cases lagged

/-- 没有断口就不报区间。 -/
theorem consecutive_records_owe_nothing (last : Nat) :
    decideLag (some last) (last + 1) = none := by
  unfold decideLag
  have closed : ¬ (1 ≤ last + 1 ∧ lagStart (some last) ≤ last + 1 - 1) := by
    simp only [lagStart]
    omega
  rw [ite_eq_right closed]

/-- 发出一条之后，会话总是 `Even`，且记住刚发的那一条。 -/
theorem sending_leaves_the_stream_even (stream : Stream) (next : Nat) :
    (stream.before next).2 = .Even (some next) := by
  cases stream with
  | Even delivered => cases delivered <;> rfl
  | Owed delivered => rfl

/-- 一条还没发过记录的会话，`seq` 的断口不是欠账：它的视角从欢迎开始。 -/
theorem a_session_that_sent_nothing_owes_no_gap (next : Nat) :
    ((Stream.Even none).before next).1 = none :=
  rfl

/-- 还没被欢迎的会话被跳过时什么也不欠。 -/
theorem skipping_before_the_welcome_owes_nothing (stream : Stream) :
    stream.skipped false = stream := by
  simp [Stream.skipped]

/-- 一个可以实现的断口：发过 3，下一条是 7，区间是 4 到 6。 -/
example : (Stream.Owed (some 3)).before 7 = (some (4, 6), .Even (some 7)) := rfl

end Wire.Reception
