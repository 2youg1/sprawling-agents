-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# citysim::wire_script

规定 `citysim::wire_script`（`tools/citysim/src/wire_script.rs`）：替身 provider 回放的那份脚本，`parse` 收下哪些脚本，以及 `Replay` 把一个请求放进哪个 run 的哪一条。套接字、记录文件与把脚本文件再读一遍在 `wire_script::exchange`，由 `spec/WireScript/Exchange.lean` 规定。本文件是 `tools/citysim/Spec.lean` 的一个分部；下面每一节保留它在 citysim 规格里的标签 §8-n，别处引作 `tools/citysim/Spec.lean §8-n`。

能写成定理的是放置规则：一个续轮的位置只取决于它带回来的调用 id，与此前开启过几个 run、别的 run 怎么交错无关；一个合法脚本里每个 id 认出写下它的那一条回复；带回两个 run 的 id、用尽的 run、没有剩下的 run 都被拒，而不是重复最后一条；第一轮按次序开启下一个 run；接上的脚本不改动已经握着的 run。替身的每一种拒绝都不在城会重发的状态码里，这是 `Refusal.status` 那张常数表的事实，不写成定理（见 §8-10）。回复本身的线上 JSON 怎样读是 gateway 的事（`gateway::response_from_wire`），模型只留下放置要用的那一样：一条回复调用了哪些 id。正文里哪些字符串是 id、怎样从 JSON 里收集它们，是 Rust 的 `Replay::collect`，由 `wire_script::tests` 守着。
-/

/-!
### 8-10 替身 provider：回放一份线上脚本，记下每一次交换（`citysim::wire_script`、`bin/provider`）

形状：`wire_script` 是 decision 加一个 adapter——哪一个请求得到哪一个回答由 `Replay` 判，套接字与记录文件的读写在 `wire_script::exchange`；`bin/provider` 是 adapter。决定见 D11。

```rust
// citysim::wire_script
pub struct WireScript { /* face: DialectKind, models: Vec<String>, runs: Vec<Vec<Value>>, ids: BTreeMap<String, Turn> */ }
impl WireScript {
    /// `{"face": "open_ai", "models": ["…"], "runs": [[<线上 JSON>, …], …]}`；每条回复经
    /// 城读回复的那个函数读一遍（§8-13）。
    pub fn parse(text: &str) -> Result<WireScript, AxError>;   // E_CONFIG_INVALID，subject 写键路径
}
pub struct ScriptedProvider { /* listener, script 的路径, replay, record */ }
impl ScriptedProvider {
    /// 读入并解析 `script`；记录文件在这里被清空：一次回放一份记录。
    pub fn open(listener: TcpListener, script: &Path, record: &Path) -> Result<ScriptedProvider, AxError>;
    pub fn url(&self) -> Result<String, AxError>;               // http://127.0.0.1:<port>/v1
    /// 接一条连接，读完一个请求，答它，把这次交换追加进记录。
    pub fn answer_one(&mut self) -> Result<(), AxError>;
}
```

**一个请求得到什么**（`Replay`，按方法分，不按路径分：路径属于兼容格式，只有 gateway 的 `chat_path` 知道它，替身再写一份就是第二个权威，而城只会用 GET 取模型列表、用 POST 发对话）：

| 请求 | 回答 |
|---|---|
| `GET` 任意路径，脚本列了模型 | 200，`{"data":[{"id":…},…]}` |
| `GET`，脚本没列模型 | 404，码 `no_model_list`：一个不提供列表的网关，城要按人写下的 id 挂它 |
| `POST` | 按 run 作答，见 §8-13；回复以 200 送出，`content-type: application/json`——城要的是流时也照读（gateway 按媒体类型认出整段回复） |
| 其他方法 | 405，码 `method_unanswered` |

拒绝的正文是 `{"error":{"type":"<码>","message":"<一句话>"}}`，OpenAI 与 Anthropic 两种兼容格式的错误都是这个形状。替身的每一种拒绝都不在 gateway 的「可重试」之列：状态码只有 400、404、405、409、410，而 gateway 重发的是 408、429 与 5xx（那一集合的权威是 `crates/gateway/src/endpoint/failure.rs` 的 `ProviderFailure::retry`），所以城把用尽读成一次不会自己好的拒绝，而不是一直重发。**用尽要拒，不重复最后一条**：黑盒检查要看得见城比脚本多调了一次（`an_exhausted_run_is_refused`）。

**什么算一个请求、记录写什么、凭据头怎样落盘**，是 `wire_script::exchange` 的事，见 `spec/WireScript/Exchange.lean` 的 §8-10 续。

**`bin/provider`**：`provider <script.json> <record.jsonl> [<listen>]`，`listen` 缺省 `127.0.0.1:0`，不是回环地址就拒（`E_INVALID_ARGS`）。绑定之后先在标准输出印一行 `SPRAWLING_PROVIDER=<url>` 再开始回放，调用它的配方读这一行拿到 URL。一次交换失败（连接断了、答不出去）写到标准错误，替身接着答下一条；记录写不进去（`E_STORAGE_FATAL`）时退出非零，因为之后的检查读的正是那份记录。

**失败**：`parse` 以 `E_CONFIG_INVALID` 拒一份读不懂的脚本（subject 是键路径，如 `runs[1][2]`，recovery 指回那一条）；套接字的失败是 `E_TOOL_UNAVAILABLE`；记录文件写不进去是 `E_STORAGE_FATAL`。

平台：替身只绑回环地址上的 TCP 端口，读写的是 HTTP/1.1 的字节与 JSON，在 Windows、macOS、Linux 上行为相同；记录文件按 LF 分行。

Rust 检查：`wire_script::tests::the_same_request_twice_is_recorded_byte_for_byte_alike`——两个替身各回放同一份脚本，各收一次同样的模型列表请求与对话请求，两份记录逐字节相同且各有两行；`wire_script::tests::an_exhausted_script_is_refused_with_its_code`——一条回复的脚本，同一个 run 再来一轮，回答是 410 与码 `script_exhausted`。
-/

/-!
### 8-13 替身按 run 作答：一个请求放进哪个 run 的哪一条（`citysim::wire_script` 的 `Replay`）

形状：`Replay` 是 decision——请求正文进来，答出放在哪里与回答什么，不碰套接字也不碰文件；把脚本文件再读一遍是 `wire_script::exchange` 的事（D15）。决定见 D11。

```rust
// citysim::wire_script（crate 内）
pub(crate) struct Replay { /* script, opened: usize */ }
pub(crate) struct Turn { pub(crate) run: usize, pub(crate) reply: usize }
impl Replay {
    pub(crate) fn new(script: WireScript) -> Replay;
    /// 这个请求放在哪一条，以及它的回答；GET 与其他方法放不进任何 run。
    pub(crate) fn answer(&mut self, asked: Asked, body: &str) -> (Option<Turn>, Answer);
    /// 接上一份以已握着的 run 开头的脚本（D15）。
    pub(crate) fn grow(&mut self, script: WireScript) -> Result<(), AxError>;   // E_CONFIG_INVALID
}
```

**一个 `POST` 得到什么**（请求正文按 JSON 读；正文里与脚本调用 id 相等的字符串值，叫它带回来的 id）：

| 请求 | 回答 |
|---|---|
| 正文不是 JSON | 400，码 `body_unreadable` |
| 带回来的 id 分属两个 run | 409，码 `runs_crossed`：替身说不出该答哪一个 |
| 带回来的 id 都属于第 r 个 run，其中最靠后的是第 k 条回复的，r 还有第 k+1 条 | 200，第 r 个 run 的第 k+1 条 |
| 同上，第 k 条已是 r 的最后一条 | 410，码 `script_exhausted` |
| 一个 id 都没带，还有没开启的 run | 200，下一个没开启的 run 的第一条；那个 run 就此开启 |
| 一个 id 都没带，每个 run 都开启过 | 先把脚本文件再读一遍（D15）；仍没有就 410，码 `no_run_left` |

这张表就是下面的 `Replay.chat`。

**`parse` 另外拒的三种脚本**，都在城开口之前、以 `E_CONFIG_INVALID` 拒，subject 是键路径：一个空的 run（`runs[r]`：它开启之后什么都答不出）；一个调用 id 在脚本里出现第二次（`runs[r][k]`，说出它第一次出现在哪里：两个 run 共用一个 id，替身就分不清是谁的续轮）；一个 run 里不是最后一条、却一个工具都不调的回复（`runs[r][k]`：一句话就让 run 结束，它后面的回复永远答不到，而且之后的请求认不出它）。这三条是下面的 `Parsed`。

Rust 检查：`wire_script::tests::two_interleaved_runs_are_each_answered_from_their_own_replies`——两个 run 各两条回复，第一轮按 run 0、run 1 的次序到，续轮按 run 1、run 0 的次序到，每个 run 拿到的是它自己的第二条，记录里四行的 `run` 依次是 0、1、1、0；`wire_script::tests::a_run_written_after_the_script_ran_out_is_opened`——一个 run 的脚本答完之后，把第二个 run 追加进脚本文件，下一个新开的 run 拿到它的第一条；`a_script_read_again_that_rewrote_a_played_run_is_refused`——重读到的脚本改写了已在答的 run，`grow` 拒它。
-/

namespace Citysim.WireScript

/-- 脚本里的一条回复：第 `run` 个 run 的第 `reply` 条，都从 0 起（`wire_script::Turn`）。 -/
structure Turn where
  run : Nat
  reply : Nat
  deriving DecidableEq, Repr

/-- 替身眼里的一条回复：它调用的那些工具调用的 id。线上 JSON 的其余部分由 gateway 读，放置不看它。 -/
structure Reply where
  calls : List String
  deriving DecidableEq, Repr

/-- 一份脚本（`wire_script::WireScript`）。兼容格式 `face` 的拼法是 `kernel::DialectKind` 的，模型把它当一个可比较的参数。 -/
structure WireScript (Face : Type) where
  face : Face
  models : List String
  runs : List (List Reply)

variable {Face : Type}

/-- 第 `run` 个 run 从第 `reply` 条起，每条回复调用的 id 与写下它的那一条。 -/
def idsOfRun (run : Nat) : Nat → List Reply → List (String × Turn)
  | _, [] => []
  | reply, given :: rest =>
    given.calls.map (fun id => (id, ⟨run, reply⟩)) ++ idsOfRun run (reply + 1) rest

/-- 从第 `run` 个 run 起，每个 run 的 id 表依次相接。 -/
def idsFrom : Nat → List (List Reply) → List (String × Turn)
  | _, [] => []
  | run, replies :: rest => idsOfRun run 0 replies ++ idsFrom (run + 1) rest

/-- 脚本里每个调用 id 与写下它的那一条回复（`wire_script::call_ids`）。 -/
def call_ids (runs : List (List Reply)) : List (String × Turn) :=
  idsFrom 0 runs

/-- 一个 run 里除最后一条以外的每条回复都调用至少一件工具。 -/
def callsBeforeItsLast : List Reply → Prop
  | [] => True
  | [_] => True
  | given :: next :: rest => given.calls ≠ [] ∧ callsBeforeItsLast (next :: rest)

/-- `WireScript::parse` 收下的脚本：没有空的 run，每个调用 id 只写一次，每个 run 只有最后一条回复不调用工具（§8-13 的三条拒绝）。 -/
structure Parsed (script : WireScript Face) : Prop where
  no_empty_run : ∀ run ∈ script.runs, run ≠ []
  each_id_once : ((call_ids script.runs).map Prod.fst).Nodup
  every_reply_but_the_last_calls : ∀ run ∈ script.runs, callsBeforeItsLast run

/-- 脚本在一个位置上的回复；位置不在脚本里时没有（`WireScript::reply`）。 -/
def WireScript.reply (script : WireScript Face) (turn : Turn) : Option Reply :=
  (script.runs[turn.run]?).bind (fun run => run[turn.reply]?)

/-- 一个请求问的是什么，只看方法（`wire_script::Asked`）。 -/
inductive Asked where
  | ModelList
  | Chat
  | Other
  deriving DecidableEq, Repr

/-- 请求正文：读不成 JSON，或读成了 JSON，带着它全部的字符串值。 -/
inductive Body where
  | Unreadable
  | Json (strings : List String)
  deriving DecidableEq, Repr

/-- 一个请求没有拿到脚本回答的原因（`wire_script::Refusal`）。 -/
inductive Refusal where
  | NoModelList
  | ScriptExhausted
  | NoRunLeft
  | RunsCrossed
  | BodyUnreadable
  | MethodUnanswered
  deriving DecidableEq, Repr

/-- 每种拒绝的状态码（`Refusal::status`）。 -/
def Refusal.status : Refusal → Nat
  | .NoModelList => 404
  | .ScriptExhausted => 410
  | .NoRunLeft => 410
  | .RunsCrossed => 409
  | .BodyUnreadable => 400
  | .MethodUnanswered => 405

/-- 一个请求的回答（`wire_script::Answer`）。 -/
inductive Answer where
  | Models (ids : List String)
  | Reply (reply : Reply)
  | Refused (refusal : Refusal)
  deriving DecidableEq, Repr

/-- 一个请求带回来的东西：一个脚本 id 都没带，最近那条是哪一条，或带了两个 run 的 id（`wire_script::Carried`）。 -/
inductive Carried where
  | Nothing
  | Latest (turn : Turn)
  | Crossed
  deriving DecidableEq, Repr

/-- 正文的字符串值里，与脚本调用 id 相等的那些各自认出的回复（`Replay::collect`）。 -/
def located (script : WireScript Face) (strings : List String) : List Turn :=
  strings.filterMap (fun text => (call_ids script.runs).lookup text)

/-- 认出的回复读成它们共同的 run 与其中最靠后的一条（`Replay::carried`）。 -/
def carriedOf : List Turn → Carried
  | [] => .Nothing
  | first :: rest =>
    if (first :: rest).all (fun turn => turn.run == first.run) then
      .Latest ⟨first.run, (rest.map Turn.reply).foldl max first.reply⟩
    else .Crossed

/-- D11 **进程外的替身 provider 是 citysim 的一个二进制，脚本就是 `ScriptModel` 的那种线上 JSON，按 run 分开作答。**

黑盒验收要让发行件去调一个会应答的 provider，而三条已有的规则各挡住一个位置：黑盒检查写在 Lean 里（`xtask boundary`），`tools/adversary/` 不自带 HTTP 服务端（`tools/adversary/Spec.lean` §13：一个假 provider 会让那个目录变成第二个 gateway 实现），为测试写的东西不进人下载的二进制（`xtask artifact`）。剩下的位置是这里：`bin/provider` 是测试工具，由 justfile 拉起，URL 经环境变量 `SPRAWLING_PROVIDER` 交给调用它的检查，与对抗器经 `SPRAWLING_BIN` 接收二进制是同一个形状。

脚本的格式只有一种：一份 JSON 写明兼容格式（`face`，拼法取 `kernel::DialectKind` 的 serde 形）、`/models` 列出的模型 id，以及一串 run，每个 run 是它按序拿到的回复，每条回复就是那个兼容格式的 provider 会发的线上 JSON。`WireScript::parse` 把每条回复经 `gateway::response_from_wire` 与 `ModelReturn::from_response` 读一遍，与城读真 provider 的回复是同一个函数，也是 `ScriptModel::from_wire` 走的那一条，所以一份替身能回放的脚本也是 `ScriptModel` 能回放的脚本，写错一个键在解析时就被拒，而不是在城里变成一条 `E_WIRE_MISMATCH`。

**一个请求属于哪个 run，由它带回来的调用 id 认。** 城每一轮都把这个 run 至今的对话整段送回去：模型要过的每一次工具调用，连同它的 id，都在请求里（OpenAI 的 `tool_calls[].id` 与 `tool_call_id`、Anthropic 的 `tool_use.id` 与 `tool_use_id`）。脚本里每个调用 id 只出现一次（`an_id_names_the_reply_that_wrote_it`），所以请求正文里任何一个与脚本调用 id 相等的字符串值就指出了这个 run，以及它最近拿到的是第几条回复；替身答它下一条。正文里一个脚本 id 都没有的请求是一个 run 的第一轮，它开启脚本里下一个还没开启的 run。于是一个 run 拿到什么只取决于它自己走到哪一步（`a_continuation_is_answered_whatever_was_opened`）：两个 run 怎么交错、一个请求被重发几次、城被杀之后接着送同一段对话，答的都是同一条。替身认的只有城本来就送的字节，产品的请求里不为它加任何字段。

**替身不生成任何文字**（产品里不内置模型）：它只回放脚本，一个 run 的回复用尽就拒绝。

被否：①把 `accounting::worker::fixture::provider::FakeProvider` 提出来复用——它挂在 `#[cfg(test)]` 下，是 crate 内部回答 HTTP 细节的替身，脚本是散落在测试里的字符串，而且用尽后重复最后一条，黑盒检查因此看不出城多调了一次；②第三种脚本格式——替身与 `ScriptModel` 各读一种，同一段对话就要写两遍；③按请求里的一句话选路（`FakeProvider` 的 `Routed`）——黑盒那扇门派活时任务的字样是固定的，每个 run 的第一轮送的文字除了房间几乎一样，而一句也出现在共享前缀里（如邻居表）的话会同时指向两条路；④按请求里 assistant 消息的条数定位——城可能折叠或卸下旧的几轮，条数会动，最近那个 id 不会；⑤重算 gateway 的 `conversation_id`——那是城的一条规则在替身里的第二份，而且它只在预设要求的主机上随请求送出。**重开参数**：两个同时开启、要拿不同回复的 run（多日小镇的确定性版本），第一轮里没有脚本 id 可认，开启的次序由机器决定——那时脚本里的 run 要带上一句它第一轮必然送出的话，形状照 `FakeProvider` 的 `Routed`；一个分叉出来的 run 继承了母 run 的调用 id，替身今天把它读成母 run 的续轮或两个 run 交叉（§8-13）——分叉进验收时，脚本要能写明一个 run 从谁分出来；一条检查要让城收到一条读不懂的回复时，脚本加上一种不经 `parse` 验证的条目。

一份正在回放的脚本：开启过几个 run（`wire_script::Replay`）。 -/
structure Replay (Face : Type) where
  script : WireScript Face
  opened : Nat

/-- 第一轮开启下一个还没开启的 run（`Replay::opening`）。 -/
def Replay.opening (replay : Replay Face) : (Option Turn × Answer) × Replay Face :=
  match replay.script.reply ⟨replay.opened, 0⟩ with
  | some given => ((some ⟨replay.opened, 0⟩, .Reply given), { replay with opened := replay.opened + 1 })
  | none => ((none, .Refused .NoRunLeft), replay)

/-- 续轮得到它带回的那一条之后的一条（`Replay::continuing`）。 -/
def Replay.continuing (replay : Replay Face) (next : Turn) : Option Turn × Answer :=
  match replay.script.reply next with
  | some given => (some next, .Reply given)
  | none => (none, .Refused .ScriptExhausted)

/-- 一个 `POST`（`Replay::chat`），§8-13 那张表。 -/
def Replay.chat (replay : Replay Face) : Body → (Option Turn × Answer) × Replay Face
  | .Unreadable => ((none, .Refused .BodyUnreadable), replay)
  | .Json strings =>
    match carriedOf (located replay.script strings) with
    | .Crossed => ((none, .Refused .RunsCrossed), replay)
    | .Latest past => (replay.continuing ⟨past.run, past.reply + 1⟩, replay)
    | .Nothing => replay.opening

/-- 一个请求（`Replay::answer`），§8-10 那张表。 -/
def Replay.answer (replay : Replay Face) : Asked → Body → (Option Turn × Answer) × Replay Face
  | .ModelList, _ =>
    if replay.script.models.isEmpty then ((none, .Refused .NoModelList), replay)
    else ((none, .Models replay.script.models), replay)
  | .Chat, body => replay.chat body
  | .Other, _ => ((none, .Refused .MethodUnanswered), replay)

/-- `held` 是 `full` 的开头（Rust 的 `starts_with`）。 -/
def starts_with [DecidableEq α] : List α → List α → Bool
  | [], _ => true
  | _ :: _, [] => false
  | a :: held, b :: full => decide (a = b) && starts_with held full

/-- 接上再读一遍的脚本：兼容格式、模型列表相同，且以已握着的 run 开头时收下，否则拒（`Replay::grow`，D15）。开启过几个 run 不变。不以已握着的 run 开头的脚本被拒是这里的 `else` 一臂，不另写定理。 -/
def Replay.grow [DecidableEq Face] (replay : Replay Face) (script : WireScript Face) :
    Option (Replay Face) :=
  if script.face = replay.script.face ∧ script.models = replay.script.models
      ∧ starts_with replay.script.runs script.runs = true then
    some { replay with script := script }
  else none

/-! #### 一个合法脚本里，id 认出写下它的那一条 -/

theorem lookup_of_mem {table : List (String × Turn)} {id : String} {turn : Turn}
    (once : (table.map Prod.fst).Nodup) (written : (id, turn) ∈ table) :
    table.lookup id = some turn := by
  induction table with
  | nil => simp at written
  | cons head rest ih =>
    obtain ⟨key, value⟩ := head
    simp only [List.map_cons, List.nodup_cons] at once
    obtain ⟨fresh, rest_once⟩ := once
    rcases List.mem_cons.mp written with same | later
    · simp only [Prod.mk.injEq] at same
      obtain ⟨rfl, rfl⟩ := same
      simp [List.lookup]
    · have differs : id ≠ key := by
        intro equal
        subst equal
        exact fresh (List.mem_map.mpr ⟨(id, turn), later, rfl⟩)
      have unequal : (id == key) = false := by simpa using differs
      simp [List.lookup, unequal, ih rest_once later]

/-- 一个合法脚本里，正文带回的一个 id 认出的就是写下它的那一条回复：两个 run 不会共用一个 id（D11）。 -/
theorem an_id_names_the_reply_that_wrote_it (script : WireScript Face) (parsed : Parsed script)
    (id : String) (turn : Turn) (written : (id, turn) ∈ call_ids script.runs) :
    (call_ids script.runs).lookup id = some turn :=
  lookup_of_mem parsed.each_id_once written

/-- 认出的回复都属于第 `run` 个 run 时，读出来的就是这个 run 与其中最靠后的一条。 -/
theorem one_run_carried_is_read_as_its_latest (first : Turn) (rest : List Turn)
    (same : ∀ turn ∈ rest, turn.run = first.run) :
    carriedOf (first :: rest) = .Latest ⟨first.run, (rest.map Turn.reply).foldl max first.reply⟩ := by
  have all : (first :: rest).all (fun turn => turn.run == first.run) = true := by
    simp only [List.all_cons, beq_self_eq_true, Bool.true_and, List.all_eq_true, beq_iff_eq]
    exact same
  simp [carriedOf, all]

/-- 认出的回复分属两个 run 时，请求读作交叉。 -/
theorem two_runs_carried_are_crossed (first : Turn) (rest : List Turn) (other : Turn)
    (carried : other ∈ rest) (differs : other.run ≠ first.run) :
    carriedOf (first :: rest) = .Crossed := by
  have some_differs : (first :: rest).all (fun turn => turn.run == first.run) = false := by
    simp only [List.all_eq_false, List.mem_cons, beq_iff_eq]
    exact ⟨other, Or.inr carried, differs⟩
  simp [carriedOf, some_differs]

/-! #### 一个续轮的回答只取决于它带回来的 id -/

/-- 一个续轮放在哪一条、答什么，与此前开启过几个 run 无关，也不开启新的 run：两个 run 怎么交错、一个请求被重发几次、城被杀之后接着送同一段对话，答的都是同一条（D11）。 -/
theorem a_continuation_is_answered_whatever_was_opened (script : WireScript Face)
    (one other : Nat) (strings : List String) (past : Turn)
    (carried : carriedOf (located script strings) = .Latest past) :
    (Replay.chat ⟨script, one⟩ (.Json strings)).1 = (Replay.chat ⟨script, other⟩ (.Json strings)).1
      ∧ (Replay.chat ⟨script, one⟩ (.Json strings)).2.opened = one := by
  simp [Replay.chat, carried, Replay.continuing]

/-- 带回第 `run` 个 run 第 `reply` 条的续轮，在那个 run 还有下一条时得到下一条。 -/
theorem every_run_is_answered_from_its_own_replies (replay : Replay Face) (strings : List String)
    (past : Turn) (carried : carriedOf (located replay.script strings) = .Latest past)
    (given : Reply) (next : replay.script.reply ⟨past.run, past.reply + 1⟩ = some given) :
    (replay.chat (.Json strings)).1 = (some ⟨past.run, past.reply + 1⟩, .Reply given) := by
  simp [Replay.chat, carried, Replay.continuing, next]

/-- 一个 run 的回复用尽之后再来的续轮被拒，而不是重复最后一条：黑盒检查看得见城多调了一次。 -/
theorem an_exhausted_run_is_refused (replay : Replay Face) (strings : List String) (past : Turn)
    (carried : carriedOf (located replay.script strings) = .Latest past)
    (used : replay.script.reply ⟨past.run, past.reply + 1⟩ = none) :
    (replay.chat (.Json strings)).1 = (none, .Refused .ScriptExhausted) := by
  simp [Replay.chat, carried, Replay.continuing, used]

/-- 带回两个 run 的 id 的请求被拒，什么也不开启。 -/
theorem a_crossed_request_is_refused (replay : Replay Face) (strings : List String)
    (carried : carriedOf (located replay.script strings) = .Crossed) :
    replay.chat (.Json strings) = ((none, .Refused .RunsCrossed), replay) := by
  simp [Replay.chat, carried]

/-! #### 第一轮按次序开启 run -/

/-- 还有没开启的 run 时，一个第一轮开启下一个，拿到它的第一条：合法脚本没有空的 run。 -/
theorem an_opening_takes_the_next_run (replay : Replay Face) (parsed : Parsed replay.script)
    (left : replay.opened < replay.script.runs.length) :
    ∃ given, replay.opening
      = ((some ⟨replay.opened, 0⟩, .Reply given), { replay with opened := replay.opened + 1 }) := by
  have held : replay.script.runs[replay.opened]? = some replay.script.runs[replay.opened] :=
    List.getElem?_eq_getElem left
  have nonempty := parsed.no_empty_run _ (List.getElem_mem left)
  cases written : replay.script.runs[replay.opened] with
  | nil => exact absurd written nonempty
  | cons first rest =>
    refine ⟨first, ?_⟩
    simp [Replay.opening, WireScript.reply, held, written]

/-- 每个 run 都开启过时，一个第一轮被拒，回放的状态不变（之后由 `exchange` 把脚本文件再读一遍，D15）。 -/
theorem no_run_left_changes_nothing (replay : Replay Face)
    (all_opened : replay.script.runs.length ≤ replay.opened) :
    replay.opening = ((none, .Refused .NoRunLeft), replay) := by
  have none_left : replay.script.runs[replay.opened]? = none :=
    List.getElem?_eq_none all_opened
  simp [Replay.opening, WireScript.reply, none_left]

/-! #### 接上的脚本不改动已经握着的 run -/

theorem starts_with_appends [DecidableEq α] :
    ∀ (held full : List α), starts_with held full = true → ∃ more, full = held ++ more
  | [], full, _ => ⟨full, rfl⟩
  | _ :: _, [], begins => by simp [starts_with] at begins
  | a :: held, b :: full, begins => by
    simp only [starts_with, Bool.and_eq_true, decide_eq_true_eq] at begins
    obtain ⟨rfl, rest⟩ := begins
    obtain ⟨more, rfl⟩ := starts_with_appends held full rest
    exact ⟨more, rfl⟩

/-- 接上之后，已经握着的每个 run 的每一条答的仍是原来那一条，开启过几个 run 也不变（D15）。 -/
theorem a_grown_replay_answers_every_held_run_alike [DecidableEq Face] (replay grown : Replay Face)
    (script : WireScript Face) (took : replay.grow script = some grown) (turn : Turn)
    (held : turn.run < replay.script.runs.length) :
    grown.script.reply turn = replay.script.reply turn ∧ grown.opened = replay.opened := by
  unfold Replay.grow at took
  split at took
  · rename_i agrees
    obtain ⟨_, _, begins⟩ := agrees
    simp only [Option.some.injEq] at took
    subst took
    obtain ⟨more, extended⟩ := starts_with_appends _ _ begins
    refine ⟨?_, rfl⟩
    simp [WireScript.reply, extended, List.getElem?_append_left held]
  · simp at took

/-- 一份两个 run、各两条回复的脚本：第一轮依次开启 run 0 与 run 1，续轮按 run 1、run 0 的次序到，各拿到自己的第二条（`two_interleaved_runs_are_each_answered_from_their_own_replies` 那一跑的形状）。 -/
example :
    let script : WireScript Unit :=
      ⟨(), [], [[⟨["a"]⟩, ⟨[]⟩], [⟨["b"]⟩, ⟨[]⟩]]⟩
    let (first, afterFirst) := Replay.chat ⟨script, 0⟩ (.Json ["hello"])
    let (second, afterSecond) := afterFirst.chat (.Json ["hello"])
    let (third, afterThird) := afterSecond.chat (.Json ["b"])
    let (fourth, _) := afterThird.chat (.Json ["a"])
    first.1 = some ⟨0, 0⟩ ∧ second.1 = some ⟨1, 0⟩ ∧ third.1 = some ⟨1, 1⟩ ∧ fourth.1 = some ⟨0, 1⟩ := by
  decide

end Citysim.WireScript
