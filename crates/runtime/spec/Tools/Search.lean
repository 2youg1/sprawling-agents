-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::tools::search

规定 `tools::search`、`tools::search::descent`（`crates/runtime/src/` 下同名的文件）。search：一个子串，在读界打开的城里走一遍。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `runtime::tools::search::tests`、`runtime::tools::search::tests::limits` 守住。
-/

/-!
### 8-30 runtime::tools::search（形状 1 判定＋形状 4 适配器）


**问题**：没有检索工具，找一个符号只有两条路——写 Python（要可选的 CPython-WASI 构件，很多机器上根本没有），或走 shell（Windows 上是 `findstr`，而 shell 本身是楼级配置可以关掉的）。旧对话有了一个地址，而**没有检索的地址比没有地址更糟**：模型被告知那里有东西，却够不着。
-/

/-!
#### 8-30-2 参数与结果


```rust
// args：{text, path?, context?}
// text：子串，必填且非空。**不是正则**。
// path：城相对前缀，缺省＝本 run 可读的全部：城根下每一栋楼先过读界，关上的整栋不走。走 chosen_path::admit。
// context：每侧上下文行数，缺省 0，上限 4（更大者夹到 4）；只在 content 模式下有意义。
// mode："content"（缺省）| "files" | "count"；别的值以 E_INVALID_ARGS 拒绝，nearby 列出三个。
const MATCH_CAP: usize = 64;      // 一次答的条数上限（content 数命中，files 与 count 数文件）；到顶即停走，结果自陈 truncated
const LINE_CAP: usize = 512;      // 一条命中里每行至多带这么多字节（search::hit）
const FILE_BYTE_CAP: u64 = 1 << 20; // 单文件上限 1 MiB，越界不读，计入 unreadable
const UNREAD_SHOWN: usize = 16;     // unread 列出的条数上限
```

结果：`{matches: [{path, line, from?, text}] | files: [path] | counts: [{path, count}], count, truncated, stopped?, unreadable, unread: [{path, why}]}`，三把列表键按模式恰出现一把：content 是 `matches`，files 是 `files`（有命中的文件，按遍历次序），count 是 `counts`（每个有命中的文件与它的命中行数）；`count` 是那张列表的条数。`from` 是命中所带上下文首行的行号，只在它早于 `line` 时出现，渲染器据它给上下文首行标号（§8-63）。**停走要说停在哪道上限**：`truncated` 为真时另带 `stopped`，一句话说是命中数到了 `MATCH_CAP` 还是命中文本到了 `INTERVAL_CAP_BYTES`，并说怎样缩小（收窄 `path`、换更长的 `text`、少要 `context`），措辞只在 `search::hit::Limit::said` 里写；只给一个 `truncated: true`，模型会把上限读成工具的毛病（测试城报告第六节第 8 条）。**一行只带一个窗口**：命中行超过 `LINE_CAP` 时切成围绕第一处匹配的窗口，上下文行从行首切，两端各带被切字节数的记号；测试城里一行 JSON transcript 有几十 KB，整行带着走时四条命中就花光了预算，同一目录里其余的 transcript 一个都没被看。行号仍是续读的凭据，窗口外的字节由 `read` 取。`line` 是 **0 基**，与 `read` 的 `offset` 同一套编号，所以「搜到再读那一段」是把一个数字原样递过去。`unreadable` 是遍历中没能看过的目录与文件数——打不开的，和大于 1 MiB 的：**找不到与看不了是两个答案**，把后者吐成前者就是把一次失败抹掉。`unread` 按遍历次序列出其中前 16 条，各带一句原因：超过单文件上限的那一句指它去按区间 `read`，打不开的那一句带上系统给的错误（措辞只在 `search.rs` 里写）：一个超大文件 `read` 仍能按区间读，模型要知道是哪一个才去读。按策略跳过的（二进制、保留区、机密楼）不计入它。规则读不出的楼（`ReadVerdict::RulesUnreadable`）不是策略而是失败：它照样整栋不走，但计入 `unreadable`，并在 `unread` 里以楼的地址带上规则读不出的原因——静默跳过它，模型会把「这栋楼的规则坏了」读成「这栋楼里没有」，而修规则的只能是人。二进制指读得出而不是 UTF-8；读不出的文件是「打不开」，不再被当成二进制吞掉。

**不用正则表达式**，理由是模式引擎会把回溯放在模型和它的下一个回合之间。子串扫描是线性的，且一个模型写错的正则不会变成一次挂死。
-/

/-! D117 按文件名找住在 search 里：给 `glob` 而不给 `pattern` 时，答名字合 `glob` 的文件

**决定**：不另开一件常驻的 `glob` 工具。search 照 Claude Code 的 `Grep` 收 `glob`，一个按文件名筛文件的 glob；不给 `pattern` 时，search 答 `path` 之下名字合 `glob` 的每个文件，形状与 `files_with_matches` 相同，按名字序，至多 `MATCH_CAP` 条。今天的 search 还只收 `text`、`path`、`context`、`mode`（§8-30-2），`pattern`、`glob` 与 `output_mode` 是它要取的形状（§8-14-1）。

**理由**：每件常驻工具在每一个请求里都带着它的说明与 schema，多一件就是每个请求多一段。`Grep` 本来就有 `glob` 这个按文件名筛的参数，「名字合 `glob` 的文件」就是「名字合 `glob`、且有一行合 `pattern` 的文件」去掉后一半，模型从 Claude Code 带来的 `glob` 的意思不变。Claude Code 自己在 macOS 与 Linux 上也把 `Glob` 与 `Grep` 移出了缺省工具集，让模型经 shell 找文件（<https://code.claude.com/docs/en/tools-reference.md> 的 Glob tool behavior 一节）。按名字序而不是 Claude Code 的修改时间序，是为了同一座城、同一个问题两次答得一样（ARCHITECTURE.md §10）。

**被否**：①另开一件与 Claude Code 的 `Glob` 同形的常驻 `glob`——每个请求多一件工具，而它省下的只是描述里的一句话；②放进 `tool` 的目录（休眠）——找文件是读代码的第一步，多一次 `tool` 的往返比一行 schema 贵；③给 `output_mode` 加一个只列名字的值——模型要另学一个合法值，Claude Code 的三个值里没有它。

**重开参数**：读数显示模型在本城反复调用并不存在的 `glob` 或 `Glob`。
-/

/-! D109 search 有三种模式，缺省仍是 content

**决定**：`mode` 取 `content`（逐条命中带行号与上下文）、`files`（只列有命中的文件）、`count`（每个文件的命中行数）；缺省 `content`。files 与 count 不读上下文、不受命中文本的字节预算约束，`MATCH_CAP` 数的是文件，停在这道上限时 `stopped` 说的是文件数。

**理由**：「这个符号在哪些文件里」与「有多少处」是找代码时最常问的两个问题，用 content 回答要把每处命中连同它的行都送进窗口，命中多时撞上 64 条或字节预算，模型拿到的是一个被截断的局部。Claude Code 的 Grep 以同样的三种输出回答这两个问题。缺省留在 content，是因为今天的提示与模型的习惯都按「搜到就有行号、行号递给 read」来用这个工具，换缺省会让每个不传 `mode` 的调用改变答的形状。

**被否**：①缺省 files（与 Claude Code 同）：改变每个现有调用的答；②只加 files 不加 count：count 的答只比 files 多一个数，而它回答的是另一个问题；③files 与 count 共用一个带可选 count 的列表：模型要靠缺不缺一把键分辨自己问的是哪一个。

**重开参数**：出现模型多数调用都先 content 再缩小 `path` 的读数时，重议缺省。
-/

/-!

#### 8-30-3 遍历跳过什么，以及为什么

| 跳过 | 理由 | 权威 |
|---|---|---|
| 保留区子树 | 一跑不读治理自己的东西 | `Address::is_reserved`（kernel） |
| 读界关上的楼 | 机密楼对楼外全关；规则读不出的楼同样关 | `kernel::address::may_read`（`crates/city/Spec.lean` §8-2） |
| `.git` 目录（任何深度，不分 ASCII 大小写） | 它是对象库不是文本，扫它只产出乱码命中；它与保留区同属受保护的元数据 | `kernel::address::PROTECTED_METADATA`，经 `Address::is_reserved` |
| 非 UTF-8 文件 | 二进制里没有可读的行 | 本节 |
| `land` 以 `E_GATE_DENIED` 拒绝的链接 | 链接的目标落在城外、保留区或关上的楼 | `chosen_path::land` |
| 指向目录的链接 | 顺着链接走可能绕回自己走过的地方；要搜目标目录，按它真实的地址去搜 | 本节 |

`land` 以别的码拒绝的链接不在这张表里：真实位置解析不出（`E_STORAGE_FATAL`，比如一个指回自己的链接）是盘没有作答，不是一栋关上的楼，所以它计入 `unreadable`，`unread` 里带上那个错误；`chosen_path::walked` 只把 `E_GATE_DENIED` 当作跳过，其余的错误交给遍历者。

名字拼不成地址段的项（以点或空白结尾、含反斜杠、冒号或控制字符）也不在这张表里：城里的每个读写都以地址为名，这样的项不可能作为命中交回，但它是「看不了」而不是「策略不让看」，所以 `search::descent::admissible` 把它计入 `unreadable`，并在 `unread` 里带上 `Address::parse` 的拒因（`its name is not an address: …`）。这类名字在 Windows 上大多建不出来（冒号、结尾的点与空白），在 macOS 与 Linux 上建得出来；`search::tests` 的 `a_name_that_is_no_address_is_named_among_the_unread` 直接问 `admissible`，三个平台上同一个答案。

大于 1 MiB 的文件不读——把一个大对象读进内存找子串是一次停顿——但它不在这张表里：它是「没看」，计入 `unreadable` 并在 `unread` 里说出来。

#### 8-30-4 红测试

超过 512 行的文件返回恰 512 行、并给出真实 `total_lines` 与可续的 `next_offset`；`search` 找到子串并带上下文；`search` 对保留区前缀以 `E_GATE_DENIED` 拒绝；两者共用的 `chosen_path::admit` 有且只有一组测试，读界的两种关各一条拒绝。`search` 不带路径时不走关上的楼；大于 1 MiB 的文件计入 `unreadable` 并在 `unread` 里带原因；一条超过字节上限的首个命中被切进上限并带标记。开放楼里一条指向机密楼的链接：`read` 穿过它以 `E_GATE_DENIED` 拒绝，`search` 从开放楼走下去不交出机密楼里的命中。
-/
