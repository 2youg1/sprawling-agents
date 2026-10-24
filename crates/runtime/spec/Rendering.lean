-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# runtime::rendering

规定 `rendering`、`rendering::read`、`rendering::refusal`、`rendering::search`（`crates/runtime/src/` 下同名的文件）。模型读到的工具结果文本，由账本里那一行和它记下的渲染器画出。本文件是 `crates/runtime/Spec.lean` 的一个分部；下面每一节保留它在 runtime 规格里的标签 §8-n，别处引作 `crates/runtime/Spec.lean §8-n`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `runtime::rendering::tests`、`runtime::fork::tests` 的 `each_result_line_is_rebuilt_by_the_renderer_it_names`、`runtime::fork::request_tests` 的 `a_branch_is_shown_the_mothers_reads_and_refusals_as_she_was`、`runtime::tools::edit::tests` 守住。
-/

/-!
### 8-63 runtime::rendering（形状 1 判定）

```rust
pub const CURRENT: Rendering = Rendering::V2;
pub fn shown(rendering: Rendering, name: &ToolName, answer: &ToolAnswer) -> Result<String, AxError>;
pub struct Copied<'a>;   // 从 read 答里抄出的段落，每行带它可能有的标记
impl Copied<'_> {
    pub fn of(text: &str) -> Copied<'_>;
    pub fn starts(&self) -> BTreeSet<u64>;                                // 段落首行可能落在的文件行号
    pub fn drawn_from(&self, start: u64) -> Option<(String, BTreeSet<u64>)>; // 首行在 start 时 read 会画的标记剥掉后的文本，与剥掉的号
    pub fn without(&self, numbers: &BTreeSet<u64>) -> String;            // 剥掉号在 numbers 里的标记，其余行原样
}
```

**两个调用方，一个渲染器。** 活的回合在 `turn::wave::settle` 里用 `CURRENT` 画出文本推进窗口，并在 `tool_result` 上记 `rendered: Some(CURRENT)` 与 `shown_bytes`（kernel D64）；分支与恢复经 `fork::result_block` 从账本重建对话，用那一行记下的版本再画一次（缺席读作 `V1`）。账本由 `runtime::replay::resume` 补上的「结局不明」那一行也记 `CURRENT`。所以同一行在两处画出的字节相同，跨二进制版本也相同。

**V1**：载荷的紧凑 JSON（`serde_json::to_string`），对答与拒都是。永久保留：账本里这把键出现之前的每一行都是这样给模型看的。

**V2**：
- **拒绝是一行**：`E_CODE action: subject; recovery`；`nearby` 非空时接 ` nearby: a, b`；`retry` 为 yes 时接 ` (retry)` 或 ` (retry after N ms)`，unknown 时接 ` (it may have taken effect)`；门拒时接 ` rule: <rule>`。引文里的换行换成空格，码恒在行首。谁为失败付账、哪个 provider 失败留在账本里，模型据以行动的只有上面这些。
- **read 是一行头加正文**：头是 `<path> lines <a>-<b> of <total>`，有续读点时接 `, next <k>`，有版本时接 `, version <v>`；文本为空时是 `<path> is empty`（总行数为 0 或未知）或 `<path> has <total> lines; offset <o> is past the end`。行号 0 基、闭区间，与 `offset` 和 `search` 的 `line` 同一套编号（§8-29-1、§8-30-2），所以头与标记上的数字可以原样递回去。读到末尾的答没有 `total_lines`，总数就是末行号加一。正文逐字节保留，只在 0 基行号是 10 的倍数的行与首行的行首插 `<n>|`。载荷的 `offset` 缺席即 0（§8-29-1）。
- **search 按文件分组**：头一行说答了什么——content 是 `<n> matches in <f> files`，files 是 `<n> files`，count 是 `<n> files, <m> matches`（单数时写 `1 match`、`1 file`）；停走时接 `; <stopped>`，有没看过的地方时接 `; <k> not searched`。content 下每个文件一组：文件路径独占一行，接着是它的命中块，组与组之间空一行；命中行与（`from` 早于命中行时）上下文首行的行首插与 read 同一语法的 `<n>|`，所以从搜索答里抄进 `edit` 的段落照样能剥掉标记。files 下每行一个路径；count 下每行 `<path>:<count>`。最后，`unread` 每条一行 `<path> not searched: <why>`，与上文空一行。`truncated` 不画：`stopped` 已经说了停在哪。
- **其余工具**的答本版仍画成 JSON（与 V1 同字节）；某个工具有了自己的读法时在这里加一条。V2 一旦随一个发布版写进过账本，再加读法就要进位到 V3，旧行照旧按 V2 画。
- **载荷不是渲染器认得的形状**（例如 read 的 `text` 不是串）时画成 JSON：模型仍读到每个字节，载荷形状的变动也不会让一条已记下的答重建失败。

**剥标记**：标记是行首的一个或多个 ASCII 数字加 `|`，语法只写在 `rendering::read` 一处，画与剥不会分歧。行首长成这样的行可能是渲染器画的标记，也可能是文件本来的文本，单看段落分不出；分得出的是位置：read 只在标记所写的那个行号的行上画它，且只画在答的首行或行号为 10 的倍数的行。`edit` 先按原样匹配 `old`；零命中时，对 `Copied::starts` 给出的每个可能的首行号 `start`，取 `drawn_from(start)` 剥掉的文本，只有它恰好从文件的第 `start` 行行首开始出现时才算被文件证实；恰有一个 `start` 被证实时，用剥掉的 `old`，`new` 只剥掉与 `old` 剥掉的号相同的标记，其余行原样（`tools::edit::as_the_file_spells`）。之后的唯一匹配判定照旧。
-/

/-! D101 工具结果的文本由带版本的渲染器画出，每一行记下自己的版本，V1 永久保留

**决定**：模型读到的工具结果文本是 `rendering::shown(版本, 工具名, 答)`；活的回合与从账本重建对话都只调它；`tool_result` 记下画出它的版本（kernel D64），缺席读作 V1 = 载荷的 JSON。新版本只加不改：一个版本一旦写进过账本就不再改它画出的字节。

**理由**：模型读到的历史必须与它当时据以行动的相同，一个分支的首个请求也必须与母 run 最后一个请求逐字节共享前缀，否则 provider 的缓存从第一条工具结果起失效（§8-2、§8-58）。账本存结构、模型读文本，两者之间的函数一旦能变，就必须随行记下用的是哪一个。两处各写一遍打印（今天之前 `wave::printed` 与 `fork::result_block` 就是两份）只靠注释维持一致。

**被否**：①账本只存渲染后的文本：accounting 视图与 UI 读结构，失去结构就要从文本反解析；②渲染器不带版本、改了就改：旧会话恢复后模型读到的历史变了，前缀缓存也断了；③版本记在 run 上：跨升级的 run 前后两半由不同版本画出。

**重开参数**：渲染需要行以外的输入（调用参数、房间地址、时钟）时，这些输入要随行记下，或改为记文本本身。
-/

/-! D102 read 每 10 行标一个行号，edit 原样匹配不到时剥掉抄进来的标记再匹配

**决定**：read 的 V2 正文在 0 基行号为 10 的倍数的行与首行行首插 `<n>|`；`edit` 的 `old` 原样零命中时，去掉被文件位置证实的标记后再试一次，`new` 只去掉与之同号的标记。

**理由**：在 o200k_base 上量同一批六个文件的前 512 行，逐行行号比「纯文本加一行头」多 18.4% token，每 10 行一个只多 1.0%，而 JSON 载荷多 6.8%；每 10 行一个标记让模型数行最多数 9 行就能说出一个行号。模型会把看到的标记抄进 `old`，不剥就是一次注定零命中的编辑和一个回合。先原样匹配，是因为文件里本来就可能有 `12|x` 这样的行（Markdown 表格、日志），原样能命中时剥掉就会改错地方。只剥被位置证实的标记，是因为原样零命中的段落里也可能夹着文件本来的 `7|x` 行，一概剥掉就再也匹配不上；`new` 只剥同号的标记，是因为模型在 `new` 里新加的带号行是它要写进文件的文本。

**被否**：①逐行行号：token 多 18%；②每 25 行：数行更长，而 10 行间隔已有公开的「不降引用准确率」的读数；③总是先剥再匹配：会错改本来就长得像标记的行；④剥 `old` 而不剥 `new`：抄了标记的模型也把它抄进了 `new`，会把标记写进文件；⑤不看位置、剥掉每一个像标记的行首：会剥掉文件本来的文本，也会改掉 `new` 里有意写的带号行。

**重开参数**：出现模型系统性地不抄标记、或抄错标记位置的读数时，重议间隔与 `new` 的剥法。
-/
