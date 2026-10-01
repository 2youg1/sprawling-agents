-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::reception

规定 `reception`、`reception::tests`、`answer::history`（`crates/wire/src/` 下同名的文件）。能不能绑、这个对端能不能录凭证、能不能欢迎它、它的帧此刻是什么意思，以及丢帧之后的区间。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-41 丢帧可见、区间补拉、暴露面凭证

三件事同一集落地，因为它们是同一条链上的三个断点：慢会话丢掉的记录没有人说、丢了之后也没有一句话能把那一段要回来、以及「暴露面必须有凭证」当时只是一句注释加一个 bool。

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

#### 三 暴露面必须有凭证：把凭证装进面里

```rust
pub enum BindFace {
    Loopback { token: Option<B3Hash> },   // 只从回环可达；配了令牌就照样要
    Exposed { token: B3Hash },            // 能从别处可达，且从不无凭证服务
}
pub fn decide_bind(addr: &SocketAddr, token: Option<B3Hash>) -> BindVerdict;
impl BindFace { pub fn token_digest(&self) -> Option<&B3Hash>; }
```

**强制点：`decide_bind` 是 `BindFace` 的唯一生产者，`bind` 是它的唯一调用者，`serve` 把 `Bound` 里的面经 `router(config, face)` 交给壳。** 壳（`ShellState.face`）此后是「这一面要求什么」的唯一读者：`decide_frame`、`decide_admission` 都拿 `&BindFace`，不再拿一个 `Option<&B3Hash>`。

- **以前是什么样**：`decide_bind` 收一个 `token_configured: bool`，`serve` 只 `match` 掉 `Refuse` 而把 `Serve(BindFace)` 丢掉；然后每一道门各自去读 `config.token_digest`。「暴露面必须有凭证」因此靠一句话与一个 bool 维持，而 `router()` 是 pub：第二个入口可以造出一个暴露着却不要求任何东西的壳。
- **现在是什么样**：`Exposed` 里**没有** `Option`——「暴露着却不要求任何东西」是一个类型上不存在的状态。这个不变量在测试里以四种格钉住（回环有无令牌、暴露有无令牌、以及 `BindingFace` 索要的摘要是不是判定它的那一个）。
- **令牌摘要是 `bind` 的入参**：它是配置说的话（谁配了令牌），面是绑定判定给出的判决。判决只在 `bind` 里产生一次，面随 `Bound` 走到壳里，故这不是同一个事实的两个家。
- **`decide_admission` 的那句注释同时兑现**：「没有配令牌的城只可能是回环城」由面的形状保证——它拿到的是一个不可能要求空的东西。

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

/-- 绑定判定给出的面，携着它向每个来者索要的配对令牌摘要。`Exposed` 的摘要不是 `Option`：「暴露着却不要求任何东西」在类型上不存在（§8-41 第三件）。 -/
inductive BindFace (Digest : Type) where
  | Loopback (token : Option Digest)
  | Exposed (token : Digest)
  deriving DecidableEq, Repr

/-- `BindFace::token_digest`：这一面向每个来者索要的摘要。 -/
def BindFace.tokenDigest {Digest : Type} : BindFace Digest → Option Digest
  | .Loopback token => token
  | .Exposed token => some token

/-- 绑定判定的结论；拒绝时的码是 `E_CONFIG_INVALID`。 -/
inductive BindVerdict (Digest : Type) where
  | Serve (face : BindFace Digest)
  | Refuse
  deriving DecidableEq, Repr

/-- `decide_bind` 的四格：只有「从别处够得到而没配令牌」一格拒绝。 -/
def decideBind {Digest : Type} : Address → Option Digest → BindVerdict Digest
  | .loopback, token => .Serve (.Loopback token)
  | .beyond, some token => .Serve (.Exposed token)
  | .beyond, none => .Refuse

/-- **拒绝恰在一格。** -/
theorem decideBind_refuses_exactly_an_exposed_address_without_a_token {Digest : Type}
    (address : Address) (token : Option Digest) :
    decideBind address token = .Refuse ↔ address = .beyond ∧ token = none := by
  cases address <;> cases token <;> simp [decideBind]

/-- **一个面索要的恰是配置给的那个摘要**：判定把凭证装进面里，面之外没有第二处读它。 -/
theorem a_served_face_demands_the_configured_digest {Digest : Type}
    (address : Address) (token : Option Digest) (face : BindFace Digest)
    (served : decideBind address token = .Serve face) : face.tokenDigest = token := by
  cases address <;> cases token <;> simp [decideBind] at served <;> subst served <;> rfl

/-- **从回环之外够得到的面总要一样东西。** -/
theorem an_exposed_city_always_demands_a_token {Digest : Type}
    (token : Option Digest) (face : BindFace Digest)
    (served : decideBind .beyond token = .Serve face) : face.tokenDigest.isSome := by
  cases token <;> simp [decideBind] at served
  subst served
  rfl

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
  · rw [if_pos bounds] at lagged
    simp only [lagStart, Option.some.injEq, Prod.mk.injEq] at lagged bounds
    omega
  · rw [if_neg bounds] at lagged
    cases lagged

/-- 没有断口就不报区间。 -/
theorem consecutive_records_owe_nothing (last : Nat) :
    decideLag (some last) (last + 1) = none := by
  unfold decideLag
  have closed : ¬ (1 ≤ last + 1 ∧ lagStart (some last) ≤ last + 1 - 1) := by
    simp only [lagStart]
    omega
  rw [if_neg closed]

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
