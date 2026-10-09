-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::slash

规定 `crates/wire/src/slash.rs`：人敲的斜杠动词，一张表。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；表的形状由 Rust 的类型守住，客户端那一份由 `cargo xtask wire-ts` 从同一张表生成、由 `wire-ts` 门比对。
-/

/-!
### 8-96 斜杠动词：`wire::slash`

```rust
pub enum Slash { Help, Room, New, Stop, Halt, Release, Model, Effort, Approve, Deny, Web, Quit, Serving, Remote, Acp, Wire }
pub enum Offered { Cli, WebUi, Both }
impl Slash {
    pub const ALL: [Slash; 16];
    pub const fn spelling(self) -> &'static str;   // "/help"、"/room"……
    pub const fn takes(self) -> &'static str;      // 斜杠之后的参数写法，"" 是不带参数
    pub const fn offered(self) -> Offered;
    pub const fn says(self) -> &'static str;       // 一句英文说明，CLI 的 `/help` 印它
    pub fn parse(word: &str) -> Option<Slash>;     // 带或不带前导 `/` 都认
}
```

| 拼写 | 参数 | 提供处 |
|---|---|---|
| `/help` | | 两处 |
| `/room` | `<addr>` | 两处 |
| `/new` | | 两处 |
| `/stop` | | 两处 |
| `/halt` | `[--all]` | 两处 |
| `/release` | `[--all]` | 两处 |
| `/model` | `[<id>]` | 两处 |
| `/effort` | `[<level>]` | 两处 |
| `/approve` | `[<id>]` | CLI |
| `/deny` | `[<id>]` | CLI |
| `/web` | | CLI |
| `/quit` | | 两处 |
| `/serving` | | CLI |
| `/remote` | `<verb>` | CLI |
| `/acp` | `[<text>]` | 两处 |
| `/wire` | `<verb> [<json>]` | CLI |

- **一张表，两端读**。CLI（`bin::console`）按 `Slash::parse` 读一行的第一个词，客户端读 `client/src/wire.ts` 里由 `cargo xtask wire-ts` 生成的 `SLASH`。表里的拼写不在别处再写一次：控制台此前的 `/at` 与客户端的 `/room` 是同一个概念的两种拼写，从此只有 `/room`。
- **事实挂在变体上**：拼写、参数、提供处与说明各是 `Slash` 上一个 `const fn` 的一臂，`ALL` 列出每个变体；`match` 穷尽，加一个动词时编译器逼人写下它的四个事实。**否决的方案**：一张 `[(&str, &str, Offered, &str)]` 的静态表加一个按下标取行的函数——下标与变体之间没有东西把它们绑在一起。
- **`/approve`、`/deny` 只在 CLI**：页面在请求旁边有按钮；`/web`、`/serving`、`/remote`、`/wire` 是终端的事，`/remote` 的配对只能在城自己的控制台上做（`crates/remote_access/Spec.lean` D4）。
- `/quit` 在两处都发 `CloseCity`（`spec/Command/Kind.lean`）；页面那一端由本地门保证只有同一台电脑上的会话发得出。
- 客户端此前自有、本表没有列出的拼写（`/dispatch`、`/steer`、`/raise`、`/compact`、`/tag`、`/untag`、`/fork`、`/admit`、`/go`、`/mcp`、`/doctor`、`/diff`）仍由 `client/src/core/slash.ts` 持有；它们并进本表、各写明提供处，是这张表的下一步。
-/
