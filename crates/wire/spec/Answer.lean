-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::answer

规定 `answer`、`answer::listing`（`crates/wire/src/` 下同名的文件）。一个 Query 答回来的形状，与它们的闭集。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-23 新客户端第一次真正用这条线，线上缺的四件事

```rust
// RunSummary 多两个字段（storage::RunHot 从 run_started 记下，`crates/storage/Spec.lean` §8-5）
pub struct RunSummary { pub run: RunId, pub who: String, pub frozen: bool,
                        pub last_seq: Seq, pub last_kind: EventKind,
                        pub addr: Option<Address>, pub started: Option<TimeMs> }

// CityAnswer 多一个字段：被 halt 的 scope 名（`city`、`<building>`、`<workshop>`），BTreeSet 序
pub struct CityAnswer { ..., pub halted: Vec<String> }

// RoundsAnswer 多开场与收场
pub struct RoundsAnswer { pub run: RunId, pub turns: Vec<Turn>, pub opened_at: Option<GitOid>,
                          pub opening: Option<Opening>, pub closing: Option<Closing> }
pub struct Opening { pub task: String, pub goal: String, pub at: TimeMs,
                     pub dispatched_by: Option<Who> }   // §8-48；policy §8-76，effort 与 names §8-79
pub struct Closing { pub completion: String, pub at: TimeMs }

// Query 第 21、22 条（声明序，QUERY_NAMES 同序追加）
Listing  { at: Option<Address> },   // → Answer::Listing(ListingAnswer)；None 是城根
Document { at: Address },           // → Answer::Document(Box<DocumentAnswer>)

pub struct ListingAnswer { pub at: Option<Address>, pub entries: Vec<Entry> }
pub struct Entry { pub name: String, pub kind: EntryKind }
pub enum EntryKind { Directory, File { bytes: u64 } }          // 目录在前、文件在后，各按名字序
// DocumentAnswer 的形状见 §8-69
```

**这四件事都是同一个发现**：ARCHITECTURE §8 说「线就是全部 API」，而旧客户端从没把这句话当真——它在浏览器里折叠 `history` 的原始记录，所以从来没问过线「这次跑在哪个房间」。新客户端只问线不折历史，四处空白一次全露出来。

- **`RunSummary.addr`／`started`**：`who` 是这次跑第一条记录的作者，恒为 `city`，不是房间。房间是 `run_started` 记录自己的 `addr`，热视图在那一条上记下它（`crates/storage/Spec.lean` §8-5）。没有它，页面无法把 `city_view` 列出的 run 归到 `hall/mayor`，「与 Mayor 的对话」拼不出来。`Option`：热视图可能只看到没有开场的一段尾巴，看不到的事不猜。
- **`CityAnswer.halted`**：`city_halted` 是记录，`halted_by` 是 `bin::assembly` 工作线程的判定，而页面刷新后两者都够不到——它只收此后的事件。答里带上被 halt 的 scope 名，一个刚打开的页面才知道城是不是停着的，而不是等下一次 dispatch 被拒才发现。名字与 `HaltScope` 的 `scope_name` 同拼法，页面按名字画。
- **`RoundsAnswer.opening`／`closing`**：回合的折叠从第一条 `model_called` 开始，所以人说的第一句（`run_started.task`）与这次跑怎么结束的（`run_frozen.completion`）都不在答里；一段对话缺开头与结尾就不是对话。两个都是 `Option`，理由同 `addr`：`HISTORY_MAX` 那段窗口可能不含开场。
- **`Listing`／`Document`**：这座城是一棵目录树，而目录树本身就是产品（glossary：「那个层级就是目录树——不是它的模型，是树本身」）；`building_view` 只回楼根的 `.md` 与房间名，房间里的 `URBANITE.md`／`JOB.md`／`Handoff.md`／`<run>.jsonl` 页面看不到，于是这个设计在界面上是不可见的。两条查询让页面能走完整棵树。**路径经 `Address` 文法把关**（非绝对、无 `..`、无 `\`、无 `:`），所以走不出城根；`.sprawling/` **允许读**——它正是要展示的那部分，且这条线只答回环（或持配对 token 的）人，与工具层对居民的拒绝不是一个门。`Document` 答什么、在哪里切、怎样判文本，见 §8-69：答复带版本，缺失、读不了、空各是一种答复。
- **`WIRE_V` 16→17，一次进位管四件事**：四件事同一提交同一哈希。
- **被否**：（a）让客户端自己折 `history` 找 `run_started`——那是旧客户端的做法，也是这四处空白存在的原因；（b）`Document` 直接回任意大小——同 §8-20／§8-21 拒绝整批的理由；（c）`Listing` 排除 `.sprawling/`——排除了要展示的东西。
-/

/-!
### 8-42 关停范围在答案里是一个类型，不是一个地址串

**这一笔只改一处**：`CityAnswer.halted` 由 `Vec<String>` 改成 `Vec<HaltScope>`（`answer.rs`）。

```rust
pub struct CityAnswer { …, pub halted: Vec<HaltScope> }   // 原为 Vec<String>
```

**根缺陷是同一件事的两种拼法，而读到两种拼法的那个读者刚好把它们比错了。** 一个 scope 在这座城里有两处书写方式，这是 `kernel::event::scope` 记下的分工：**账本**持 `city`／`building:<addr>`／`workshop:<addr>`，因为每一份已写下的历史都是这样；而**命令帧**持 §8-40 的 `HaltScope` 那样的带标签形状。答案面此前把它降成账本的那个串，于是客户端拿到的是 `building:lab` 与裸 `lab` 两个东西，`views/building.svelte:115` 拿后者去比前者——**被停的楼永远读成未停，release 永远不出现**，而两侧各自诚实，没有任何测试会红。

**答案用命令面的类型，因为那是提问的词汇。** 一页宁可拿 `HaltScope` 去比 `HaltScope`，也不要为了知道看的是哪栋楼而把一个字符串拆开——拆开就是给文法安第二个家，而这正是这一笔在关的东西。转换只发生在服务端一处（`views::answering` 的 `named`），账本的书写方式一个字未动。

**客户端那侧的两处消费也归一处**：`core/scope.ts` 是两种拼法唯一的接缝（`scopeOf`／`sameScope`／`buildingIsShut`／`cityIsShut`），`city_halted` 记录折进 `halted` 时经它转一次；其余每个比较点都比较类型。

**与事件载荷不冲突**：`city_halted` 的载荷里 `scope` 是 `kernel::Scope`，经 `schemars(with = "String")` 在 schema 上呈现为字符串。答案面改用 `HaltScope` 与那条覆盖不冲突——两者是不同的帧，各写各的读者。
-/

/-!
### 8-48b 一行 run 带上它的结局、它开的 PR 与它等的事

```rust
pub struct RunSummary {
    // …既有字段…
    pub completion: Option<String>, // run_frozen.completion；未冻结或窗口外为 None
    pub pr: Option<String>,         // 最近一条 pr_opened 的 branch（城里的 PR 以分支为名）
    pub ask: Option<String>,        // 最后一条记录是 approval_requested 时，它的 action_desc
}
```

- **为什么要上线**：只看结果的城把冻结的 run 分成做完、失败与已结束，并在一行末尾写出 PR 或等你做的事。页面重载后它只有 `city_view` 的答，没有这三件，每个冻结 run 都落进「已结束」，等你的 run 只知道在等、不知道等什么。
- **三件都由 `storage::RunHot` 折出**（`crates/storage/Spec.lean` §8-5），`summarize` 照抄，不读账本。
- **`ask` 只在 `last_kind` 为 `approval_requested` 时有值**：页面判断等待用的是 `last_kind`，`ask` 跟着同一条记录走，二者不会一个说在等、一个说不等。
- **`pr` 是分支名而不是数字**：城里的 PR 是 `collab::OpenRequest`，它的身份是分支（`node` 由分支解析而来），账本里没有别的编号；编一个序号就是给人一个线外查不到的名字。
- **被否：把 `Rounds.closing` 让城逐行去问**。那是每行一次查询，一座两百个 run 的城首屏要问两百次，而这三件热视图本来就在折。
-/

/-!
### 8-48e 一行 run 带上人交给它的任务与目标

```rust
pub struct RunSummary {
    // …既有字段…
    pub task: Option<String>,       // run_started.task；空串或窗口外为 None
    pub goal: Option<String>,       // run_started.goal；同上
}
```

- **为什么要上线**：run 板以人说的第一句话给一行 run 起名，没有这句话就以目标起名。页面只从事件流里听到 `run_started` 时才知道这两句；重载之后它只有 `city_view` 的答，每一行都只能叫「某房间里的一次 run」，同一个房间里的几次 run 在板上没法分开。
- **两件都由 `storage::RunHot` 从 `run_started` 折出**（`crates/storage/Spec.lean` §8-5），`summarize` 照抄，不读账本。空串记作 `None`：一句空的任务不是一个名字，页面对 `None` 与空串本来就得同样处理，线上只留一种拼法。
- **被否：让页面逐行问 `Rounds` 拿 `opening`**。与 §8-48b 否掉逐行问 `closing` 同一个理由：一座两百个 run 的城首屏要问两百次，而热视图本来就在折 `run_started`。
- **`WIRE_V` 42→43**：给既有答面类型加字段是「语法换形而名字没换」那一类，golden 随之变；`client/src/wire.ts` 由 `cargo xtask wire-ts --write` 同集重生成。
-/

/-!
### 8-63 页面上「历史已证明到哪一条」：`CityAnswer.proved`

```rust
pub struct CityAnswer { /* …既有字段… */ pub proved: Option<Seq> }
```

- **意思。** `Some(n)`：账本到 `n` 为止整条链已经证明完好，写者在接受命令（`crates/sprawling/Spec.lean` §8-122 的 M3）；证明之后写下的每一行都由这个已证明的写者接在链上，所以 `n` 就是视图此刻折到的最后一条。`None`：服务中的城还在后台证明（命令此时答 `E_HISTORY_UNPROVEN`），或者证明发现链断了；视图照常答查询（S10 Q2 (a)），页面据此标明「历史还在核对」。
- **读法。** 视图持有写者挂上的那个 `storage::ChainHalt` 的一份句柄（`crates/sprawling/Spec.lean` §8-134）；`proved()` 为真时答视图的头。一次性查询（`views::ask`）与测试里的视图起步前已经同步证明过整条链，答它们的头。
- 名字不变而形状变，与本批共用 `WIRE_V` 45。
-/
