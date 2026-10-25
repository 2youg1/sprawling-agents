-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::tools::search

规定 `tools::search`、`tools::search::predicate`、`tools::search::descent`（`crates/runtime/src/` 下同名的文件）。search：一个正则或一个 glob，在读界打开的城里走一遍。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `runtime::tools::search::tests`、`runtime::tools::search::tests::limits` 守住。
-/

/-!
### 8-30 runtime::tools::search（形状 1 判定＋形状 4 适配器）


**问题**：没有检索工具，找一个符号只有两条路——写 Python（要可选的 CPython-WASI 构件，很多机器上根本没有），或走 shell（Windows 上是 `findstr`，而 shell 本身是楼级配置可以关掉的）。旧对话有了一个地址，而**没有检索的地址比没有地址更糟**：模型被告知那里有东西，却够不着。
-/

/-!
#### 8-30-2 参数与结果


```rust
// args：{pattern?, path?, glob?, output_mode?, -i?, -A?, -B?, -C?, context?}——Claude Code 的 `Grep` 的名字（§8-14-1）
// pattern：正则，`regex` crate 的语法，即 ripgrep 缺省引擎的语法（D120）；逐行匹配，非空。
//   不给 pattern 时必须给 glob，答名字合 glob 的文件（D117）；两者都不给＝E_INVALID_ARGS，恢复语说出 pattern 与 glob。
// path：城相对前缀，缺省＝本 run 可读的全部：城根下每一栋楼先过读界，关上的整栋不走。走 chosen_path::admit。
// glob：ripgrep 的 glob 语法（`globset`，`*` 不跨 `/`）；不含 `/` 时对文件名匹配，含 `/` 时对文件相对 `path` 的路径匹配；只筛文件，不筛目录。
// output_mode："files_with_matches"（缺省）| "content" | "count"；别的值以 E_INVALID_ARGS 拒绝，nearby 列出三个（D109）。
//   不给 pattern 时只收 files_with_matches：名字合 glob 的文件没有命中行可数。
// -i：为真时不分大小写。
// -B、-A：命中行之前、之后的上下文行数；-C 与 context 同义，是两侧的行数；单侧的盖过两侧的；
//   每侧缺省 0，上限 4（更大者夹到 4）；只在 content 下有意义。
// multiline 为真、type、非 0 的 offset：E_INVALID_ARGS，恢复语说出本城的走法；-n、-o、head_limit 不读（D121）。
const MATCH_CAP: usize = 64;      // 一次答的条数上限（content 数命中，files 与 count 数文件）；到顶即停走，结果自陈 truncated
const LINE_CAP: usize = 512;      // 一条命中里每行至多带这么多字节（search::hit）
const FILE_BYTE_CAP: u64 = 1 << 20; // 单文件上限 1 MiB，越界不读，计入 unreadable
const UNREAD_SHOWN: usize = 16;     // unread 列出的条数上限
```

结果：`{matches: [{path, line, from?, text}] | files: [path] | counts: [{path, count}], count, truncated, stopped?, unreadable, unread: [{path, why}]}`，三把列表键按模式恰出现一把：content 是 `matches`，files_with_matches 与只给 glob 的调用是 `files`（有命中或名字合 glob 的文件，按遍历次序），count 是 `counts`（每个有命中的文件与它的命中行数）；载荷的键名不随参数改名：账本里已有的答照旧按它们画（§8-63）。`count` 是那张列表的条数。`from` 是命中所带上下文首行的行号，只在它早于 `line` 时出现，渲染器据它给上下文首行标号（§8-63）。**停走要说停在哪道上限**：`truncated` 为真时另带 `stopped`，一句话说是命中数到了 `MATCH_CAP` 还是命中文本到了 `INTERVAL_CAP_BYTES`，并说怎样缩小（收窄 `path` 或 `glob`、把 `pattern` 写得更具体、少要上下文），措辞只在 `search::hit::Limit::said` 里写；只给一个 `truncated: true`，模型会把上限读成工具的毛病（测试城报告第六节第 8 条）。**一行只带一个窗口**：命中行超过 `LINE_CAP` 时切成围绕第一处匹配的窗口，上下文行从行首切，两端各带被切字节数的记号；测试城里一行 JSON transcript 有几十 KB，整行带着走时四条命中就花光了预算，同一目录里其余的 transcript 一个都没被看。行号仍是续读的凭据，窗口外的字节由 `read` 取。`line` 与 `from` 是 **0 基下标**，与 read 载荷里的 `offset` 同一套；渲染器把它们画成与 read 的调用 `offset` 同一套的行号（V3 从 1 数起，D119），所以「搜到再读那一段」是把模型读到的一个数字原样递过去。`unreadable` 是遍历中没能看过的目录与文件数——打不开的，和大于 1 MiB 的：**找不到与看不了是两个答案**，把后者吐成前者就是把一次失败抹掉。`unread` 按遍历次序列出其中前 16 条，各带一句原因：超过单文件上限的那一句指它去按区间 `read`，打不开的那一句带上系统给的错误（措辞只在 `search.rs` 里写）：一个超大文件 `read` 仍能按区间读，模型要知道是哪一个才去读。按策略跳过的（二进制、保留区、机密楼）不计入它。规则读不出的楼（`ReadVerdict::RulesUnreadable`）不是策略而是失败：它照样整栋不走，但计入 `unreadable`，并在 `unread` 里以楼的地址带上规则读不出的原因——静默跳过它，模型会把「这栋楼的规则坏了」读成「这栋楼里没有」，而修规则的只能是人。二进制指读得出而不是 UTF-8；读不出的文件是「打不开」，不再被当成二进制吞掉。

**正则不回溯**（D120）：`regex` crate 以有限自动机匹配，时间与输入长度成线性，一个模型写错的正则不会变成一次挂死；编不出来的正则（语法错、超过编译的大小上限）以 `E_INVALID_ARGS` 拒绝，主语带上 crate 给的诊断，恢复语说转义的写法。`glob` 编不出来同码拒。

**按文件名找不读文件**：只给 `glob` 时，遍历照旧过读界、保留区与链接的判定（§8-30-3），对名字合 glob 的文件只列名字，不打开它，所以大于 1 MiB 的与非 UTF-8 的文件照样列出；`MATCH_CAP` 数的是文件。
-/

/-! D117 按文件名找住在 search 里：给 `glob` 而不给 `pattern` 时，答名字合 `glob` 的文件

**决定**：不另开一件常驻的 `glob` 工具。search 照 Claude Code 的 `Grep` 收 `glob`，一个按文件名筛文件的 glob；不给 `pattern` 时，search 答 `path` 之下名字合 `glob` 的每个文件，形状与 `files_with_matches` 相同，按名字序，至多 `MATCH_CAP` 条。glob 的语法与 ripgrep 的 `--glob` 相同，由同一个实现（`globset`）判：不含 `/` 的 glob 对文件名匹配，所以 `*.rs` 找出每一层的 Rust 文件；含 `/` 的对文件相对 `path` 的路径匹配，所以 `src/**/*.ts` 与 Claude Code 的 `Glob` 在同一个 `path` 下答同一批文件。

**理由**：每件常驻工具在每一个请求里都带着它的说明与 schema，多一件就是每个请求多一段。`Grep` 本来就有 `glob` 这个按文件名筛的参数，「名字合 `glob` 的文件」就是「名字合 `glob`、且有一行合 `pattern` 的文件」去掉后一半，模型从 Claude Code 带来的 `glob` 的意思不变。Claude Code 自己在 macOS 与 Linux 上也把 `Glob` 与 `Grep` 移出了缺省工具集，让模型经 shell 找文件（<https://code.claude.com/docs/en/tools-reference.md> 的 Glob tool behavior 一节）。按名字序而不是 Claude Code 的修改时间序，是为了同一座城、同一个问题两次答得一样（ARCHITECTURE.md §10）。

**被否**：①另开一件与 Claude Code 的 `Glob` 同形的常驻 `glob`——每个请求多一件工具，而它省下的只是描述里的一句话；②放进 `tool` 的目录（休眠）——找文件是读代码的第一步，多一次 `tool` 的往返比一行 schema 贵；③给 `output_mode` 加一个只列名字的值——模型要另学一个合法值，Claude Code 的三个值里没有它。

**重开参数**：读数显示模型在本城反复调用并不存在的 `glob` 或 `Glob`。
-/

/-! D109 search 的 `output_mode` 取 Claude Code 的三个值，缺省 files_with_matches

**决定**：`output_mode` 取 `files_with_matches`（只列有命中的文件）、`content`（逐条命中带行号与上下文）、`count`（每个文件的命中行数），缺省 `files_with_matches`，三个拼写与缺省都与 Claude Code 的 `Grep` 相同（D114）。files_with_matches 与 count 不读上下文、不受命中文本的字节预算约束，`MATCH_CAP` 数的是文件，停在这道上限时 `stopped` 说的是文件数。

**理由**：「这个符号在哪些文件里」与「有多少处」是找代码时最常问的两个问题，用 content 回答要把每处命中连同它的行都送进窗口，命中多时撞上 64 条或字节预算，模型拿到的是一个被截断的局部。从 Claude Code 带来习惯的模型要行时就写 `output_mode: "content"`，不写时等的是文件名单；缺省给 content，它不写的那些调用就多花一窗命中行，而它本来只要名单。改名之后旧的调用一律被拒（D115），换缺省不再改变任何一个照旧能跑的调用的答。

**被否**：①缺省留在 content——上面的理由；②只加 files 不加 count：count 的答只比 files 多一个数，而它回答的是另一个问题；③files 与 count 共用一个带可选 count 的列表：模型要靠缺不缺一把键分辨自己问的是哪一个；④保留本城的拼写 `files`：模型要另学一个合法值，传来 Claude Code 的拼写被拒一次。

**重开参数**：出现模型多数调用都先 files_with_matches 再对同一个 `pattern` 要 content 的读数时，重议缺省。
-/

/-! D120 `pattern` 是 ripgrep 语法的正则，由 `regex` crate 逐行判

**决定**：search 的 `pattern` 是正则，由 `regex` crate 编译与匹配，语法就是 ripgrep 缺省引擎的语法（ripgrep 的缺省引擎正是这个 crate）；`-i` 为真时不分大小写。一行一行地匹配：`.` 不匹配换行，`^` 与 `$` 是行首行尾。依赖与它的被否写在 `crates/runtime/Spec.lean` §13。

**理由**：Claude Code 的 `Grep` 是 ripgrep，模型写给它的 `fn\s+main`、`impl.*Tool`、`interface\{\}` 照 ripgrep 的语法写；子串搜索把这些字面地找一遍，答「没有」，而文件里明明有，这正是「找不到与看不了是两个答案」要防的那种错答。以前不用正则，理由是模式引擎会把回溯放在模型和它的下一个回合之间；`regex` crate 不回溯，以有限自动机匹配，时间与输入长度成线性，编译有大小上限，这个理由对它不成立。逐行匹配让行号、上下文与 content 的窗口照旧按行算（§8-63），也与 ripgrep 的缺省一致。

**被否**：①保留子串——上面的理由；②`fancy-regex` 或 PCRE2——多了环视与反向引用，代价是回溯，恰是原来那条理由要挡的；③在 `exec` 里跑 `rg`——不是每台主机都有 `rg`，City Hall 的工具集没有 `exec`（`crates/city/Spec.lean` §8-22），而经 `exec` 的答没有读界与续读的行号；④整份文件一次匹配、支持跨行——见 D121。

**重开参数**：读数显示模型常写环视或反向引用而被拒；或 ripgrep 换了缺省引擎。
-/

/-! D121 本城答不了的 Grep 参数：会让答案误导的拒，只改篇幅的不读

**决定**：`multiline` 为真、给了 `type`、`offset` 不为 0 时，search 以 `E_INVALID_ARGS` 拒绝，恢复语说出本城的走法：跨行的写成逐行能匹配的 `pattern`，按语言筛的用 `glob`（如 `*.rs`），翻页的收窄 `path`、`glob` 或 `pattern`。`-n`、`-o`、`head_limit` 不读：行号恒在，`-o` 与 `head_limit` 只让答更短，答里已经说了停在哪（`stopped`）。

**理由**：不读一把键，模型拿到的答要么照它的意思，要么比它要的多一些；这对 `-n`、`-o`、`head_limit` 成立。对另外三把不成立：逐行匹配一个跨行的 `pattern` 答「没有」，忽略 `type` 让每个文件都入选、看起来却像筛过，忽略 `offset` 让翻页的模型一次次拿回同一页。这三种答都是错的而不是多的，拒绝花一个回合，错答花的是模型据以行动的那个事实。§8-14-1 的「调用里多出来的键照旧不读」只对前一类成立。

**被否**：①全都不读——上面的三种错答；②全都拒——`-n: true` 是 Claude Code 的模型最常带的一把键，拒它只换来一个回合；③实现 `multiline`、`type` 与 `offset`——跨行匹配要整份文件一次匹配并把命中换算回行，`type` 要一张语言到扩展名的表（ripgrep 的那张有上百行），分页要一个跨调用稳定的次序与游标，三者都比一句恢复语贵得多，而 `glob` 与收窄已经答了同样的问题。

**重开参数**：读数显示模型在本城反复传这三把键，被拒之后仍不改。
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

大于 1 MiB 的文件不读——把一个大对象读进内存找匹配是一次停顿——但它不在这张表里：它是「没看」，计入 `unreadable` 并在 `unread` 里说出来。

#### 8-30-4 红测试

超过 512 行的文件返回恰 512 行、并给出真实 `total_lines` 与可续的 `next_offset`；`search` 按正则找到命中并带上下文，只给 `glob` 时按名字列出文件；`search` 对保留区前缀以 `E_GATE_DENIED` 拒绝；两者共用的 `chosen_path::admit` 有且只有一组测试，读界的两种关各一条拒绝。`search` 不带路径时不走关上的楼；大于 1 MiB 的文件计入 `unreadable` 并在 `unread` 里带原因；一条超过字节上限的首个命中被切进上限并带标记。开放楼里一条指向机密楼的链接：`read` 穿过它以 `E_GATE_DENIED` 拒绝，`search` 从开放楼走下去不交出机密楼里的命中。
-/
