-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::conversation

规定 `conversation`（`crates/runtime/src/` 下同名的文件）。会话历史：已发出的消息不再被改写。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。

§8-47 与 §8-47-1 只有文字，它们写下的接口形状由 Rust 的类型守住。§8-47-2 是形式规格：信封的两条性质在下面被证明，Rust 一侧由 `runtime::conversation` 里的 proptest `only_the_user_speaks_as_user_and_a_letter_holds_its_body` 对着同一组性质检查。
-/

/-!
### 8-47 runtime::conversation：已发出的消息不再被改写，空回复之后的工具结果除外（形状 2 值类型）


```rust
impl Conversation {
    pub fn mark_sent(&mut self);   // 本次组装发出了 messages() 的全部；之后到达的 user 文本不再并入其中任何一条
}
```

- **规则**：`Conversation` 记下上次组装发出了几条消息（`sent`）。user 文本（steer、提醒）只并入**尚未发出**的最后一条 User 消息；最后一条 User 消息已经发出时，文本进一个待投槽（`held`），由下一次 `push_tool_results` 接在这一波结果之后，即词汇表里 Steer 的落点「下一份工具结果的末尾」。待投槽不在 `messages()` 里，所以它永远不会出现在一条它到达之前就已组好的请求中。
- **调用点**：活的 run 在 `Turn::assemble` 答出 `Advanced` 之后调一次（`run::lifecycle`）；`fork` 在读到本 run 的 `prompt_shape_compared` 时调一次：`prompt_assembled` 每个 run 只写一次（8-39），而 `prompt_shape_compared` 是每一回合组装之后紧接着写的那一行。两边的标记来自同一个事实（这一回合的请求组好了），所以分支按同一规则重放出同样的字节。
- **理由**：`BeforeCall`／`BeforeWave`／`BeforeToolCall`／`BeforeSpawn` 四个安全点都在组装之后，那时窗口最后一条仍是刚随请求发出的 User 消息。把 steer 并进去，下一次请求里 steer 排在一条没读过它的助手回复之前：模型看到的时间顺序是假的，而且已发出消息的字节变了，provider 的前缀缓存从这条消息起全部失效。
- **字节**：`tools/fixtures/golden-p0` 的剧本在第 0 回合收到 steer，它第二次请求的 run 区域在工具结果之后带着这条 steer。
- **例外：空回复之后的工具结果**：一条没有内容的回复不推助手消息，所以随后的 `push_tool_results` 碰到的最后一条仍是已发出的 User 消息。结果和待投文字并进这条消息，它的字节因此变了，provider 的前缀缓存从这条消息起失效。这里接受改写，因为另一条路是在它之后另开一条 User 消息，即被否的①：两条相邻的 User 消息是入口不变量要排除的形状。时间顺序仍然是真的：这条消息之后没有模型读过的回复。改写之后这条消息重新算作未发出（`sent` 退到它之前），所以在下一次组装之前到达的 steer 并进它，排在这批结果之后，不再多等一波。
- **一条回复没有任何调用时**：run 就此结束（8-37），待投文字不再有下一次组装；它已由 `steer_received` 入账，账本仍是它的来历。
- **fork 的切点落在一波之内时**：这一波整波丢弃（半个交换没有 provider 接受），分支只继承 `messages()`，待投槽里的文字不随分支走。待投文字只在「组装之后、这一波结果之前」存在，所以它针对的正是被丢弃的那一波；分支从没看到那一波，把它接到分支的第一条消息里，模型会读到一句指向不存在的上下文的话。这段文字已由 `steer_received` 入账，母 run 的账本仍是它的来历。被否：让 `Inherited` 带上待投文字——分支的首条 User 消息会以一句针对别人那一波的 steer 开头。
- **被否**：①在已发出的 User 消息之后另开一条 User 消息——两条相邻的 User 消息正是本模块入口不变量要排除的形状；②由执行器在工具结果之后再调一次 `push_steer`——待投状态会住在 `Conversation` 之外，`fork` 要复刻第二份同样的记忆，两个家会漂移。
-/

/-!
### 8-47-1 runtime::conversation：对话窗口在进程里的字节预算（形状 2 值类型）

今天 `Conversation` 把一个活的 run 交换过的每一条消息都留在进程里（`messages` 与 `held`），进程里没有按字节计的上界：模型的上下文窗口由 `runtime::compaction` 与 `runtime::offload` 压住，那是请求的上界，不是常驻内存的上界。长回合 1000 步时请求窗口约 1.2 MiB（`tools/xtask/budgets.toml` 的 `long_turn_peak` 行），所以一个 run 的常驻字节随回合数线性增长，N 个并发 run 乘以 N。

- **预算**：每个活的 run 一个常量 `CONVERSATION_RESIDENT_BYTES`（提议 4 MiB，`crates/sprawling/spec/Serving/Memory.lean` §8-173 的预算表）。
- **超过预算时**：已发出的消息（`sent` 之前的那一段）从最旧的一条起移出进程，字节进 CAS，`Conversation` 只留它们的 `Locator`；组装请求时按 `Locator` 从盘上读回。未发出的消息与待投槽总在进程里，因为它们还会被改写。
- **冻结**：run 冻结时 `Conversation` 随驱动一起丢掉，进程里不留这个 run 的任何字节；transcript 是盘上的那一份。
- **三个平台**：读回走 CAS 的文件，CAS 读同一份 std 接口，Windows、macOS、Linux 没有区别；读回的页留在操作系统的文件缓存里，不计入私有字节。
-/

/-! D34 对话窗口里已发出的消息可以移出进程、按 `Locator` 读回

理由：§8-47 保证已发出的消息不再被改写，所以移出去的字节与读回来的字节相同，下一次请求逐字节不变，provider 的前缀缓存不受影响；这正是 `crates/sprawling/spec/Serving/Memory.lean` 里 `evicted_reads_back` 证明的性质，模型里的 `disk` 就是 CAS。被否的做法：①按条数截掉最旧的消息——那会改变下一次请求的字节，是 compaction 的事，不是内存的事；②整个窗口一直留在进程里——常驻字节随回合数与并发 run 数相乘增长，违背 Roadmap M0 第 7 条「斜率在噪声以内」。代价：每次组装多读一次盘，读的是操作系统文件缓存里的热页；交互路径上这一读不让 p99 超过 16 ms，由 W7 的读数判。重开的条件：组装请求的读盘在读数里超过 16 ms。
-/

/-!
### 8-47-2 runtime::conversation：谁在说话写在类型里，居民的话装在一个它关不上的信封里（形状 2 值类型）

```rust
pub enum Speaker { Person, Resident(Letter) }
pub struct Letter { pub from: String, pub run: Option<kernel::RunId>, pub kind: LetterKind, pub sender: Option<String> }
pub enum LetterKind { Steer, Reply }
impl Speaker {
    pub fn recorded(&self) -> String;                         // 写进 steer_received.source 的拼法
    pub fn from_recorded(source: &str) -> Result<Speaker, AxError>;   // fork 把它读回来
}
impl Conversation {
    pub fn push_steer(&mut self, speaker: &Speaker, text: &str);
    pub fn push_city_note(&mut self, text: &str);             // 城市自己的话（策略变更），以 `city: ` 开头
}
```

- **规则**：`push_steer` 是 steer 进窗口的唯一入口，它按 `Speaker` 渲染。`Person` 渲染成 `user: <text>`；`Resident` 渲染成 `<letter from="@<room>" run="<run>" kind="steer|reply" sender="<state>"><body></letter>`，没有的属性不写，属性值与正文里的 `<`、`>`、`&` 写成 `&lt;`、`&gt;`、`&amp;`。所以以 `user:` 开头的一块文字只来自 User 的 steer，居民的正文里没有 `<`，关不上自己的信封，也开不出第二个。
- **构造**：`Speaker::Person` 在生产代码里只由 `accounting::worker::desk`（User 经控制面送来的 steer）构造，另一处是 `Speaker::from_recorded` 读回账本上的 `user`，那一行只由前者写下。
- **账本**：`steer_received.source` 记 `Speaker::recorded()`：`user`，或 `@<room>`，后面跟零到三个 `run=<uuid>`、`kind=steer|reply`、`sender=<state>`，以空格分开。旧的行只有 `@<room>`，读回成 `kind=steer`、没有 run 与 sender 的信；居民的回信在旧行里是正文 `<room> replied: …`，读回时仍在信封里。
- **三个平台**：只有文字与类型，Windows、macOS、Linux 上一样。
- **被否**见 `crates/collab/Spec.lean` D16。
-/

namespace Runtime.Conversation

/-- 谁在说话。居民的属性由城市写，模型里只是一串字符。 -/
inductive Speaker where
  | person
  | resident (attrs : List Char)

def escapeChar (c : Char) : List Char :=
  if c = '<' then ['&', 'l', 't', ';']
  else if c = '>' then ['&', 'g', 't', ';']
  else if c = '&' then ['&', 'a', 'm', 'p', ';']
  else [c]

def escape (text : List Char) : List Char := text.flatMap escapeChar

def userTag : List Char := ['u', 's', 'e', 'r', ':', ' ']

def closeTag : List Char := ['<', '/', 'l', 'e', 't', 't', 'e', 'r', '>']

def openTag (attrs : List Char) : List Char :=
  ['<', 'l', 'e', 't', 't', 'e', 'r', ' '] ++ escape attrs ++ ['>']

def render : Speaker → List Char → List Char
  | .person, body => userTag ++ body
  | .resident attrs, body => openTag attrs ++ escape body ++ closeTag

/-- 窗口里的块：每次 push 渲染出一块，按次序。 -/
def fold (pushes : List (Speaker × List Char)) : List (List Char) :=
  pushes.map fun push => render push.1 push.2

theorem lt_not_in_escapeChar (c : Char) : '<' ∉ escapeChar c := by
  unfold escapeChar
  by_cases h1 : c = '<'
  · simp [h1]
  · by_cases h2 : c = '>'
    · simp [h2]
    · by_cases h3 : c = '&'
      · simp [h3]
      · simp [h1, h2, h3]
        exact fun h => h1 h.symm

/-- 正文转义之后没有 `<`：它写不出 `</letter>`，也写不出 `<letter`。 -/
theorem lt_not_in_escape (text : List Char) : '<' ∉ escape text := by
  unfold escape
  simp only [List.mem_flatMap, not_exists, not_and]
  exact fun c _ => lt_not_in_escapeChar c

theorem gt_not_in_escapeChar (c : Char) : '>' ∉ escapeChar c := by
  unfold escapeChar
  by_cases h1 : c = '<'
  · simp [h1]
  · by_cases h2 : c = '>'
    · simp [h2]
    · by_cases h3 : c = '&'
      · simp [h3]
      · simp [h1, h2, h3]
        exact fun h => h2 h.symm

theorem gt_not_in_escape (text : List Char) : '>' ∉ escape text := by
  unfold escape
  simp only [List.mem_flatMap, not_exists, not_and]
  exact fun c _ => gt_not_in_escapeChar c

/-- 居民的一块恰是一个信封：开标签、转义后的正文、闭标签，正文里没有尖括号。 -/
theorem letter_holds_its_body (attrs body : List Char) :
    render (.resident attrs) body = openTag attrs ++ escape body ++ closeTag ∧
      '<' ∉ escape body ∧ '>' ∉ escape body :=
  ⟨rfl, lt_not_in_escape body, gt_not_in_escape body⟩

theorem resident_not_user (attrs body : List Char) :
    ¬ userTag.isPrefixOf (render (.resident attrs) body) := by
  simp [render, openTag, userTag, List.isPrefixOf]

/-- 对任意一串 push：以 `user: ` 开头的块只来自 User 的 steer。 -/
theorem user_line_only_from_person (pushes : List (Speaker × List Char)) (line : List Char)
    (h : line ∈ fold pushes) (hu : userTag.isPrefixOf line) :
    ∃ body, (Speaker.person, body) ∈ pushes ∧ line = render .person body := by
  unfold fold at h
  obtain ⟨⟨speaker, body⟩, hin, hline⟩ := List.mem_map.mp h
  cases speaker with
  | person => exact ⟨body, hin, hline.symm⟩
  | resident attrs =>
    subst hline
    exact absurd hu (resident_not_user attrs body)

/-- 咬得动：不转义时，正文 `</letter>` 把 `<` 带进信封。 -/
theorem withoutEscape_closes : '<' ∈ (closeTag : List Char) := by
  simp [closeTag]

end Runtime.Conversation
