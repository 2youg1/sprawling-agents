-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::reply

规定 `reply`（`crates/wire/src/` 下同名的文件）。回到发问者那里的东西：一条拒绝去哪、落到了哪，与外来编辑器的进度。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-! D2 写者点名的四个类型不在 `server` feature 之后

**决定**：`Reply`、`Delivered`、`AcpProgress`（`wire::reply`）与 `Pairing`（`wire::auth`）在任何 feature 组合下都编译；crate 根的名字与再导出不变。`server` feature 只带监听器：axum 的路由、WebSocket 会话、HTTP 门与把拒绝写成响应体的 `refusal_text`。

**理由**：城的唯一写者搬进 `accounting` 之后（accounting-SPEC.md 8-11），它的命令入口、桌子、欠账与 ACP 受理点名这四个类型，而 `accounting` 依赖本 crate 时关掉默认 feature，因为它要的是词汇，不是 TCP 栈。四个类型本来就是词汇：`Reply` 包一个 `Fn(AxError) -> Delivered`，正是为了不把传输层写进签名（§8-2）；另外三个是普通的枚举与结构体，不持有 tokio 或 axum 的任何东西。`Pairing` 是一次配对判定的结论，与配对令牌同住 `auth`；判定本身 `decide_admission` 仍在 `reception`。

**被否**：①给 `accounting` 的 `wire` 依赖打开 `server`：它把 tokio 与 axum 拉进一个从不监听的 crate，`wire` 不带 `server` 的那份构建（`just features`）也就不再守住「词汇不需要监听器」；②在 `accounting` 里另写一份同形的类型再在装配根互转：同一个拒绝去向有两个定义，互转是第二个权威。

**守护**：`just features` 编译不带 `server` 的 `wire`，`accounting` 的每一次编译也是。四个类型之一若被挪回 `server` 之后，`accounting` 编不过。
-/
