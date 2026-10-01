-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# city::gitignore

规定 `gitignore`（`crates/city/src/` 下同名的文件）。一栋楼的哪些字节进历史。本文件是 `crates/city/Spec.lean` 的一个分部；下面每一节保留它在 city 规格里的标签 §8-n，别处引作 `crates/city/Spec.lean §8-n`，决定引作 `city D<n>`。
-/

/-!
### 8-21 city::gitignore：一栋楼承诺的东西进历史，一次会话在想的东西不进

**需求**：`building::create`（raise）与 `building::adopt` 立起一栋楼时，同时放下两样东西：一份 `SPEC.md`，和一份 `.gitignore`。

**接口**：

```rust
// city::gitignore（一张从各模块取名的规则表＋一个幂等落盘动作）
pub const GITIGNORE_FILE: &str = ".gitignore";
/// 把本城的忽略规则补进这栋楼的 .gitignore。已有的字节一行不删，
/// 缺哪行补哪行；文件不存在则整份写出。
pub(crate) fn place(building_root: &Path) -> Result<(), AxError>;
/// 把城自己的保留子树补进城根的 .gitignore，同一条只追加的路。
/// 门面上叫 `city::ignore_city_records`：裸的 `place_city` 在装配层里说不出放下的是什么。
pub fn place_city(city_root: &Path) -> Result<(), AxError>;
/// 城根一块，加上每栋楼一块，缺哪行补哪行。开城时调；门面上叫 `city::keep_records_out_of_git`。
pub fn place_everywhere(city_root: &Path) -> Result<(), AxError>;

// city::spine_files
pub const SPEC_FILE: &str = "SPEC.md";   // 字节来自 crates/city/templates/SPEC.md（include_str!）
```

- **不对称本身就是这一条的全部内容**：`SPEC.md` 与这栋楼自己保留子树里的五份承诺进版本库；工作文档与对话记录一份都不进（D5 定规）——`Roadmap.md`、`Memo.md`、`Handoff.md`、`JOB.md`、`URBANITE.md`、`Archive/` 与各个房间。一栋楼向外承诺的东西必须在历史里，任何一次克隆都读得到；一次会话当时在想什么不是承诺，它留在运行中的机器上。
- **`SPEC.md` 是十七节 crate SPEC 的压缩式，不是第二种形状**：`crates/city/templates/SPEC.md` 的十二节逐节对应 crate SPEC 的节次（需求／验收／假设／权威／命名／边界／接口／错误／依赖／硬编码／测试／决策），只是把「现状分析、工作流程、实现逻辑、影响面、模型体验、文档同步」这几节留给 crate 自己。它压缩，不另起。
- **`place` 只追加，从不重写**：被收编的目录往往已经有一份 `.gitignore`，里面写着这个项目自己的东西。整份覆盖会把它们冲掉，而那正是 adopt 承诺不会碰的字节。依据是逐行比对（去空白后相等即视为已有），因此重复 raise 不会把同一段追加两次。
- **显式的反忽略**：`!SPEC.md` 与保留子树的放行行写进块里，而不是靠「没人忽略它们」这个默认。被收编的仓库可能已经忽略了 `*.md` 或一切点开头的目录；那时「这栋楼的承诺在历史里」就是假的，而没有人会发现。
- **保留子树逐文件放行，不整棵放行**：块里先 `.sprawling/` 忽略任意深度的保留子树，再 `!/.sprawling/` 只把这栋楼自己的那一棵放回来，`/.sprawling/*` 把它清空，最后逐行放行五份承诺——`RULES.toml`、`CONFIG.toml`、`FILTERS.toml`、`DESKTOP.toml` 与 `skills/`。三个理由：①城自己的保留子树同名，账本、对象库与金库引用住在那里，一行 `!.sprawling/` 把它们一并放回版本控制的可见面；②那一行不带斜杠，因此对楼下每一个居民、每一个房间的保留子树同样生效，而那些是机器上的东西，不是这栋楼的承诺；③`CONFIG.toml` 正是 MCP 凭据的落点，它进历史的前提是 §8-4b 的逐值判定同时成立——两件事是同一次改动。
- **五个名字都从写它的模块取**：`kernel::layout` 的 `CONFIG_FILE`／`FILTERS_FILE`／`BUILDING_SHELF`、`policy` 的 `RULES_FILE`／`DESKTOP_SCOPE_FILE`、`spine_files` 的四份脊柱文档名。因此 `BLOCK` 由 `&[&str]` 常量改为 `block() -> Vec<String>`：一个改了名的文档不会在这里留下一条谁都不写的规则。
- **顺序就是文法**：git 认最后一条命中的规则，所以「忽略—放回目录—清空—逐行放行」这四步不能重排。这一条由 git 自己验过：外层再写 `.*` 与 `*.md`，`SPEC.md` 与 `.sprawling/CONFIG.toml` 仍然进历史，`.sprawling/ledger/` 仍然不进，一个嵌套目录自己的 `.sprawling/CONFIG.toml` 也不进。
- **城根也有一块，只有一行 `/.sprawling/`**：城根的保留子树装着账本、对象库、工作树与金库引用，全是城运行时留下的记录，不是项目的内容。人让城围着一个工作区立起来时，那个工作区常常就是一个 git 仓库（项目本身），而楼这一层的块只管楼自己的保留子树，城根的那一棵就会以几百兆的未跟踪目录出现在项目的 `git status` 里。`place_city` 在立城时（`form_city`，`init`、`up` 与「用一个已有的文件夹」三条路都经过它）把这一行补进城根的 `.gitignore`，与 `place` 同一个只追加、逐行比对的规矩；锚在根上（带前导斜杠），因此楼与房间自己的保留子树仍由楼那一块逐文件放行。城根的 `City.md` 与 `hall/` 不在这一行里：前者是人要改的城规，后者是一栋楼，它自己的块已经说了它哪些进历史。
- **工作文档按名忽略，不靠房间封条**：块里 `JOB.md`、`URBANITE.md`、`Handoff.md`、`Roadmap.md`、`Memo.md` 与一次 run 的对话记录（`runtime::transcript` 写在房间里的 `<run>.jsonl`，按 `kernel::layout::RUN_ID_PATTERN` 与 `TRANSCRIPT_EXT` 拼成 `????????-????-????-????-????????????.jsonl`）不带斜杠，在楼下任意深度都不进历史；`/Archive/` 锚在楼根，那里存的是人说过的偏好、做过的决定与纠正。按名忽略是必要的，因为房间封条只盖在城自己建的目录上：人把活派到一个项目里本来就有的子目录（`proj/src`），城不能往那里放一个 `*`——那会让这个源码目录里此后的每一个新文件都悄悄进不了历史。
- **城替派活新建的房间都封上**：派活开房间的那一步（sprawling-SPEC §8-40 的 `room_for`，城为一次派活写下的第一件东西）遇到一个没经过 `open` 的房间地址时调 `city::claim_room`：房间还不存在就建出它并封上，与 `room::open` 同一个 `seal_room`。放在这一步而不是写 `JOB.md` 时，是因为会话冻下的形状（`write_session`）在任务单之前就写进房间自己的 `.sprawling/CONFIG.toml`，那一写会先把目录建出来。一个直接派到 `building/room` 地址、没经过 `open` 的派活因此不再留下一个没封的房间。已存在的目录不封（理由同上一条），它里面的城文件由按名的规则挡住。
- **每次开城补一遍**：`place_everywhere` 在城的写者打开时（sprawling `RunWorker::holding`，serve、resume 与立城都经过它）对城根与 `building::all` 列出的每栋楼各补一遍缺的行。规则表是会长的：它长出一行时，早先立起的楼要在下一次开城就拿到这一行，而不是只有新立的楼才有——D5 的定规对一座已经在跑的城同样成立。只追加、逐行比对，所以一栋规则齐全的楼一个字节都不会被写。一份写不进去的 `.gitignore` 让开城以 `E_STORAGE_FATAL` 拒绝：城说不出它的对话记录会不会进 git 时，不接活。
- **房间由房间自己忽略**：`room::open` 在新开的房间里放一份只有 `*` 一行的 `.gitignore`。楼这一层的 `.gitignore` 写不出「房间」——房间是人当场命名的普通子目录，立楼时它们还不存在，而在被收编的仓库里按通配符去猜哪个子目录是房间会误伤源码目录。
- 被否：在楼的 `.gitignore` 里写 `*/JOB.md`、`*/URBANITE.md` 一类通配。它只忽略房间里的某几个文件名，会让一次会话的其余产物照样进历史，等于把这条规则写成一半。
-/

/-! D5 定规：项目文件夹里的工作文档与对话记录恒不进 git

**决定**：城在项目文件夹里写下的工作文档与对话记录——计划、备忘、交接、任务单、居民身份、归档的偏好与决定、房间里的一切、城根保留子树里的账本与对象库——在城写过的每一个 git 边界上都被忽略：城根一块（§8-21 `place_city`），每栋楼一块（`place`），每个城建的房间一张封条（`seal_room`）。进历史的只有一栋楼的承诺：`SPEC.md` 与它保留子树里的五份治理文件。

**理由**：这些文件写的是人与居民之间说过的话和人的偏好，是隐私；项目的历史会被推送、克隆、分享，一旦进去就收不回来。城的检查点提交（`crates/storage/Spec.lean` §8-8）经 libgit2 的 `add_all` 与 `is_path_ignored`，同样遵守这些规则，所以忽略规则同时是人的提交与城自己的检查点的边界。

**为什么不合成一份文件**：git 只读一个仓库工作树之内的 `.gitignore`，一个嵌套的仓库不读它父目录的那一份（在一个父仓库里写 `.sprawling/`、在子仓库里建 `.sprawling/ledger`，`git -C 子仓库 status` 照样列出它）；而一份文件里的不带斜杠的规则对它所在目录之下的任意深度生效，写在城根的 `Roadmap.md` 会连带忽略项目自己根上的同名文件。工作区是装着几个项目的父目录时，每个项目是它自己的仓库，规则只能写在项目自己的根上。所以规则只有一张表（`gitignore` 模块），写进每一个它必须生效的位置。

**重开参数**：git 若允许一个仓库读取其外的忽略文件，或城改为只在城自己的仓库里工作，这一条的落点可以收成一处。
-/
