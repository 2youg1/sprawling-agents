-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# Reading：一条消息的上限，以及读端在拒绝之后停下

规定 `crates/agent_protocols/src/mcp/reading.rs`（`agent_protocols::mcp::reading`）：`read_one_message`、`read_whole_message` 与 `Lines`。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。

一台 MCP server 或一家 harness 是外部方，它写的每个字节都是不受信任的输入，而读它的进程同时是这座城唯一的写者：内存耗尽等于历史中断（`crates/agent_protocols/Spec.lean` §8 的消息上限一条）。所以：

1. **上限之内的消息照常读**（`reads_a_message_within_the_ceiling`）：上限是能通过的最大消息，不是最小的被拒消息。
2. **读出的消息不超过上限**（`a_message_read_is_within_the_ceiling`），**一条消息的前「上限 + 1」个字节里没有换行就被拒**（`refuses_past_the_ceiling`）：拒绝在对侧还在写时就到，本城不会握住一条无界的消息。
3. **拒绝是终止性的**（`the_reader_stops_at_its_first_refusal`）：读端线程交出的结果里，只有最后一条可以不是消息。超限消息没读完的尾巴与下一条消息在字节上分不开，所以拒绝之后不再读同一个 source。
4. **HTTP 的整段 body 受同一个上限**（`reads_a_whole_body_within_the_ceiling`、`refuses_a_whole_body_past_the_ceiling`）。

模型逐字节读；Rust 按块读（`BufRead::fill_buf` 给的块，`read_whole_message` 每次 8 KiB），每读进一块就比一次上限。两者对同一份输入的判决相同：一条消息被拒，当且仅当它在换行之前已经超过上限；差别只在拒绝那一刻本城多握了至多一块，这是内存的上界，不是判决。不是 UTF-8 的消息与读不出的底层错误由 Rust 拒（`E_WIRE_MISMATCH`、`E_TOOL_UNAVAILABLE`），不在本模型里：它们不改变上面四条的任何一条。

读端与调用方之间的交接是 `sync_channel(0)`：读端等调用方取走这一条才读下一条，一条连接在内存里至多握着一条读完待取的消息与一条正在读的。这是并发的事实，模型不表示线程；它由 `Lines::over` 的构造与 `mcp::stdio`、`mcp::sse` 的测试守住（`crates/agent_protocols/Spec.lean` §16）。
-/

namespace AgentProtocols.Mcp.Reading

/-- 分帧的那个字节。消息的其余字节对这里的性质只贡献长度。 -/
def newline : Nat := 10

/-- 读一次得到什么。不是 UTF-8 与读不出不在模型里（见文件头）。 -/
inductive Outcome where
  /-- 一条完整的消息，分帧的换行已去掉。 -/
  | message (bytes : List Nat)
  /-- 对侧在两条消息之间关了输出：子进程做完了。 -/
  | ended
  /-- 一条消息超过了上限：`E_WIRE_MISMATCH`，报出上限与是哪台 server。 -/
  | overCeiling
  /-- 输出在消息中途结束：剩下的是碎片，不是答案。 -/
  | fragment
  deriving DecidableEq, Repr

/- D6 消息上限的四条决定。
(a) 上限不是参数：签名是 `read_one_message(source, server)`，上限取自 `MESSAGE_CEILING`，不由调用方传入。一个可以由调用方选的上限，就是每个传输各选一个的上限，而拒词必须对每一台 server 报出同一个数字。测试用真实大小的输入压两边边界，不靠注入一个小上限；本模型把上限写成参数只为陈述对每一个上限都成立的性质。
(b) 数值是推导来的：`IMAGE_MAX_BYTES` 是 2 MiB，base64 后 2,796,203 B；8 MiB 让一次工具答案装得下这样一张图、它的文字与 JSON-RPC 信封，对本城已接受的最大载荷留 3 倍余量。读数与推导记在 `tools/xtask/budgets.toml` 的 `[mcp_message_ceiling]`。
(c) 拒绝是终止性的，不是跳过一条：超限消息未读完的尾巴与下一条消息在字节上无从分辨，所以拒绝之后调用方恒不再从同一个 source 读，传输回收子进程（`the_reader_stops_at_its_first_refusal`；被否的「跳过一条接着读」见文末 `withoutStopping_reads_a_tail_as_a_message`）。`Received` 是穷尽枚举而非 `Option<String>`：「对侧关了输出」是调用方要据以停读的状态，用缺席表示它就等于让每个调用点各自重推一遍。
(d) 读端不排队：读端每读一条都调 `read_one_message`，读到的交给 `sync_channel(0)`，等调用方取走这一条才读下一条，于是一条连接在内存里至多握着一条读完待取的消息与一条正在读的，上界是两倍 `MESSAGE_CEILING`；再往后的字节留在子进程的管道或 TCP 的接收窗口里，那是对侧的缓冲，不是本城的。被否：`sync_channel(N)`（N > 0）多买到的只是 reader 能先读 N 条，而一次调用只取一条答案，多读的那几条只会多占 N 倍上限的内存；无界的 `channel()` 在读端慢时把内存吃光，而「慢」正是一个被工具卡住的 Run 的常态。 -/
/-- `read_one_message`：`held` 是这条消息已读的字节，返回读出的结果与 source 里剩下的字节。
一个字节就要把已读的推过上限时当场拒，所以 `held` 恒不超过上限。 -/
def readFrom (ceiling : Nat) : List Nat → List Nat → Outcome × List Nat
  | held, [] => (if held.isEmpty then .ended else .fragment, [])
  | held, b :: rest =>
    if b = newline then (.message held, rest)
    else if ceiling < held.length + 1 then (.overCeiling, rest)
    else readFrom ceiling (held ++ [b]) rest

/-- 从一条消息的开头读。 -/
def readOne (ceiling : Nat) (source : List Nat) : Outcome × List Nat :=
  readFrom ceiling [] source

theorem readFrom_message_bounded (ceiling : Nat) :
    ∀ (source held m rest : List Nat), held.length ≤ ceiling →
      readFrom ceiling held source = (.message m, rest) → m.length ≤ ceiling
  | [], held, m, rest, _, h => by
    cases hh : held.isEmpty <;> simp [readFrom, hh] at h
  | b :: source, held, m, rest, within, h => by
    unfold readFrom at h
    by_cases nl : b = newline
    · simp only [nl, if_true, Prod.mk.injEq, Outcome.message.injEq] at h
      rw [← h.1]; exact within
    · simp only [nl, if_false] at h
      by_cases over : ceiling < held.length + 1
      · simp [over] at h
      · simp only [over, if_false] at h
        exact readFrom_message_bounded ceiling source (held ++ [b]) m rest
          (by simp; omega) h

/-- 读出的消息不超过上限。 -/
theorem a_message_read_is_within_the_ceiling (ceiling : Nat) (source m rest : List Nat)
    (h : readOne ceiling source = (.message m, rest)) : m.length ≤ ceiling :=
  readFrom_message_bounded ceiling source [] m rest (Nat.zero_le _) h

theorem readFrom_within (ceiling : Nat) (rest : List Nat) :
    ∀ (m held : List Nat), newline ∉ m → held.length + m.length ≤ ceiling →
      readFrom ceiling held (m ++ newline :: rest) = (.message (held ++ m), rest)
  | [], held, _, _ => by simp [readFrom]
  | b :: m, held, clean, fits => by
    have nl : b ≠ newline := fun e => clean (by simp [e])
    have clean' : newline ∉ m := fun e => clean (List.mem_cons_of_mem _ e)
    have room : ¬ ceiling < held.length + 1 := by simp at fits; omega
    have fits' : (held ++ [b]).length + m.length ≤ ceiling := by simp at fits ⊢; omega
    simp only [List.cons_append, readFrom, nl, if_false, room]
    rw [readFrom_within ceiling rest m (held ++ [b]) clean' fits']
    simp

/-- 上限之内、以换行结尾的消息照常读，剩下的字节留给下一次。 -/
theorem reads_a_message_within_the_ceiling (ceiling : Nat) (m rest : List Nat)
    (clean : newline ∉ m) (fits : m.length ≤ ceiling) :
    readOne ceiling (m ++ newline :: rest) = (.message m, rest) := by
  have := readFrom_within ceiling rest m [] clean (by simpa using fits)
  simpa [readOne] using this

theorem readFrom_past (ceiling : Nat) (rest : List Nat) :
    ∀ (m held : List Nat), newline ∉ m → held.length ≤ ceiling →
      held.length + m.length = ceiling + 1 →
      (readFrom ceiling held (m ++ rest)).1 = .overCeiling
  | [], held, _, within, sum => by simp at sum; omega
  | b :: m, held, clean, within, sum => by
    have nl : b ≠ newline := fun e => clean (by simp [e])
    have clean' : newline ∉ m := fun e => clean (List.mem_cons_of_mem _ e)
    simp only [List.cons_append, readFrom, nl, if_false]
    by_cases over : ceiling < held.length + 1
    · simp [over]
    · simp only [over, if_false]
      exact readFrom_past ceiling rest m (held ++ [b]) clean' (by simp; omega)
        (by simp at sum ⊢; omega)

/-- 前「上限 + 1」个字节里没有换行的输入被拒，不论后面还有什么：拒绝在对侧还在写时就到。 -/
theorem refuses_past_the_ceiling (ceiling : Nat) (m rest : List Nat)
    (clean : newline ∉ m) (length : m.length = ceiling + 1) :
    (readOne ceiling (m ++ rest)).1 = .overCeiling :=
  readFrom_past ceiling rest m [] clean (Nat.zero_le _) (by simpa using length)

/-- 两条消息之间关了输出是结束，不是拒绝。 -/
theorem a_closed_output_is_the_end (ceiling : Nat) : readOne ceiling [] = (.ended, []) := rfl

/-! ## 读端线程（`Lines::over`） -/

/-- 是不是一条消息。 -/
def Outcome.isMessage : Outcome → Bool
  | .message _ => true
  | .ended | .overCeiling | .fragment => false

/-- 读端线程交出的结果：一直读，读到第一条不是消息的结果就把它交出并停下。`fuel` 只为结构递归，
取 source 的长度加一就够（每读一条至少消耗一个字节）。 -/
def pump (ceiling : Nat) : Nat → List Nat → List Outcome
  | 0, _ => []
  | fuel + 1, source =>
    match readOne ceiling source with
    | (.message m, rest) => .message m :: pump ceiling fuel rest
    | (.ended, _) => [.ended]
    | (.overCeiling, _) => [.overCeiling]
    | (.fragment, _) => [.fragment]

/-- 只有最后一条可以不是消息。 -/
def StopsAtFirstRefusal : List Outcome → Prop
  | [] => True
  | [_] => True
  | o :: rest => o.isMessage = true ∧ StopsAtFirstRefusal rest

theorem stops_cons_message (m : List Nat) :
    ∀ rest, StopsAtFirstRefusal rest → StopsAtFirstRefusal (.message m :: rest)
  | [], _ => trivial
  | _ :: _, h => ⟨rfl, h⟩

/-- 拒绝是终止性的：读端交出的结果里，拒绝或结束只能是最后一条。 -/
theorem the_reader_stops_at_its_first_refusal (ceiling : Nat) :
    ∀ (fuel : Nat) (source : List Nat), StopsAtFirstRefusal (pump ceiling fuel source)
  | 0, _ => trivial
  | fuel + 1, source => by
    unfold pump
    split
    · exact stops_cons_message _ _ (the_reader_stops_at_its_first_refusal ceiling fuel _)
    all_goals trivial

/-! ## HTTP 的整段 body（`read_whole_message`） -/

/- D8 HTTP 的答复 body 受同一个上限：一次 POST 的答复 body 就是一条消息（或一段只带一条消息的事件流），`mcp::http` 经 `read_whole_message` 读它：持有的字节一过上限即整条拒（`E_WIRE_MISMATCH`，`Retry::Unknown`：server 收下了请求），所以一段没有尽头的 body 在还在到达时就被拒，本城至多多握一块（与按行读同样的 8 KiB）；不是 UTF-8 同样拒。上限量的是整段 body，事件流的 `event:` 行也算在内：本城在内存里握的是整段 body，上限约束的正是它。非 2xx 的答复不读 body：拒词只报状态码，一页报错文字不值得读进内存，也不该因为它太长而把 401 报成形状错误。被否：`response.text()` 整段读，那是三种传输里唯一没有上限的一条。 -/
/-- 一段 body 读到它结束为止，持有的字节一过上限就拒。 -/
def readWhole (ceiling : Nat) : List Nat → List Nat → Outcome
  | held, [] => .message held
  | held, b :: rest =>
    if ceiling < held.length + 1 then .overCeiling else readWhole ceiling (held ++ [b]) rest

theorem readWhole_within (ceiling : Nat) :
    ∀ (body held : List Nat), held.length + body.length ≤ ceiling →
      readWhole ceiling held body = .message (held ++ body)
  | [], held, _ => by simp [readWhole]
  | b :: body, held, fits => by
    have room : ¬ ceiling < held.length + 1 := by simp at fits; omega
    simp only [readWhole, room, if_false]
    rw [readWhole_within ceiling body (held ++ [b]) (by simp at fits ⊢; omega)]
    simp

theorem readWhole_past (ceiling : Nat) :
    ∀ (body held : List Nat), held.length ≤ ceiling → ceiling < held.length + body.length →
      readWhole ceiling held body = .overCeiling
  | [], held, within, past => by simp at past; omega
  | b :: body, held, within, past => by
    simp only [readWhole]
    by_cases over : ceiling < held.length + 1
    · simp [over]
    · simp only [over, if_false]
      exact readWhole_past ceiling body (held ++ [b]) (by simp; omega) (by simp at past ⊢; omega)

/-- 上限之内的 body 原样读出。 -/
theorem reads_a_whole_body_within_the_ceiling (ceiling : Nat) (body : List Nat)
    (fits : body.length ≤ ceiling) : readWhole ceiling [] body = .message body := by
  simpa using readWhole_within ceiling body [] (by simpa using fits)

/-- 超过上限的 body 整段拒，不截断后解析。 -/
theorem refuses_a_whole_body_past_the_ceiling (ceiling : Nat) (body : List Nat)
    (past : ceiling < body.length) : readWhole ceiling [] body = .overCeiling :=
  readWhole_past ceiling body [] (Nat.zero_le _) (by simpa using past)

/-!
## 咬得动的演示

拿掉上限，读端就会交出一条比上限长的消息；拿掉「拒绝之后停下」，读端就会把一条被拒消息的尾巴当作新消息交出。下面两条定理就是这两条反例，`lake build` 证明它们。
-/

/-- 没有上限的读法。 -/
def readFromUnbounded : List Nat → List Nat → Outcome × List Nat
  | held, [] => (if held.isEmpty then .ended else .fragment, [])
  | held, b :: rest =>
    if b = newline then (.message held, rest) else readFromUnbounded (held ++ [b]) rest

theorem withoutCeiling_reads_past_it :
    readFromUnbounded [] [1, 1, 1, newline] = (.message [1, 1, 1], []) := by decide

/-- 拒绝之后接着读的读端。 -/
def pumpSkipping (ceiling : Nat) : Nat → List Nat → List Outcome
  | 0, _ => []
  | fuel + 1, source =>
    match readOne ceiling source with
    | (.ended, _) => []
    | (o, rest) => o :: pumpSkipping ceiling fuel rest

/-- 上限为 2，一条四个字节的消息被拒之后，它的最后一个字节被读成了一条新消息。 -/
theorem withoutStopping_reads_a_tail_as_a_message :
    pumpSkipping 2 5 [1, 1, 1, 7, newline] = [.overCeiling, .message [7]] := by decide

end AgentProtocols.Mcp.Reading
