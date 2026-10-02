-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::answer::find

规定 `answer::find`（`crates/wire/src/` 下同名的文件）。一棵子树下名字含着一段文字的文件，有界。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-82 按名字找文件：`Query::Find { under, text }`、`Answer::Find`

```rust
Query::Find { under: Address, text: String }
Answer::Find(FindAnswer)

pub const FIND_MAX: usize = 50;          // 一答至多列出的文件
pub const FIND_WALK_MAX: usize = 20_000; // 一次走树至多看过的条目
pub struct FindAnswer {
    pub under: Address,     // 问题的回声
    pub text: String,       // 问题的回声
    pub paths: Vec<Address>, // 相对 `under` 的路径；浅的在前，同一层按名字序
    pub walked: Walked,
}
pub enum Walked {
    Whole, // `under` 下的每个条目都看过了：没列出的文件就是名字不含这段文字
    Cut,   // 走到 FIND_MAX 个文件或 FIND_WALK_MAX 个条目时停下，后面可能还有
}
```

- **为什么。** refrain 路线图 §3-11 让 Accel-P 直接按名字找一个文件，§3-14 第 9 行要「应用内的会话／文件查找」。`Query::Listing` 一次只答一层，页面要找一个名字就得一层一层把整棵树问回来，而一棵楼的树可以有上万个条目；页面也不能声称它没问过的目录里没有这个名字。这一问在城里走树，答出前若干个名字匹配的文件，并说出这次走完了没有。
- **走的是 `Query::Listing` 读的那棵树，读法也相同。** 盘是权威，问的时候读；在视图锁放开之后读（与 `Listing`、`Document` 同一类 `Prepared`）。逐层走，先走完一层再进下一层，每层的条目按名字序，所以浅的文件在前、两台机器答出同样的次序。读不了的目录当作空目录，与 `Listing` 一样。
- **匹配。** 只看文件的名字（最后一段），不看目录，不看路径的其余部分；比较不分大小写（两边各取 `to_lowercase`）。空的 `text` 匹配每个文件，所以它答的是这棵树最浅的 `FIND_MAX` 个文件。
- **不进的目录。** `.git`（`kernel::GIT_METADATA`）不进：它里面是仓库自己的对象，一个人要打开的文档不在那里，而它一个就能耗尽走树的上限。链接不跟：链接到上层目录会让走树绕圈。名字不是一个 `Address` 的文件（例如含 `:` 的名字）不列出，因为 `Query::Document` 也打不开它。
- **有界，并说出界。** `FIND_MAX` 是一答在线上的大小，一个找文件的列表也只需要前几十个；`FIND_WALK_MAX` 是一次走树的代价上限。两者任一到达而树还没走完，答 `Walked::Cut`，页面据此说「还有没看过的」，而不是让人以为没有。`under` 不存在时答空表、`Whole`。
- **只动名字表，不进 `WIRE_V`。** 新加一个查询与一个答复，旧帧一个也没改形；名字表多一项，schema 哈希随之变（D1、§4）。
-/

/-! D18 找文件是城里的一次有界走树，不是页面一层一层地问 `Listing`

**决定**：`Query::Find` 在城里逐层走 `under` 下的树，答出至多 `FIND_MAX` 个名字含着这段文字的文件，以及这次走树有没有走完（§8-82）。

**理由**：页面每按一个键都要一个答案；逐层问 `Listing` 要一问一层、一层一个来回，深的树要几十个来回才答得出「没有」，而城就在盘的旁边，一次走树就答完。答带上 `Walked`，页面就不必声称一个它没看过的目录里没有这个名字（refrain §3-14 第 9 行）。

**被否**：①页面逐层问 `Listing` 并在浏览器里匹配——来回数随树深增长，且每一层都要传回不匹配的名字；②城维护一份文件名索引——那是树的第二份拷贝，要跟着每一次写盘、每一次检查点与人手里的改动更新，而盘本身已经是权威；③按路径的子串匹配——人按 Accel-P 时输入的是文件名的一部分，按路径匹配会让目录名把整棵子树都带进结果。

**重开参数**：一棵楼的条目数让一次走树超过页面一次按键能等的时间（约 100 ms），或人要按内容而不是按名字找时，改成一份随写盘更新的索引。
-/
