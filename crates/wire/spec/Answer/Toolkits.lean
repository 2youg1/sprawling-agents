-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::answer::toolkits

规定 `answer::toolkits`（`crates/wire/src/` 下同名的文件）。外包服务供哪些外部应用，每一个对这座城站在哪。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-35b 外包服务的目录与一键连接：`Query::Toolkits` 与 `Command::ConnectToolkit`

人给出一把 key，城替他列出目录并建连接，不要他去对方控制台取回 server id 与 user id：建 auth config 不必访问对方后台（`POST /api/v3/auth_configs` 可建托管配置），而正在退役的是 `initiate` 而非 `link`（`docs/third-party.md`）。

```rust
Query::Toolkits,                                        // 不带参数，也不带 key
Command::ConnectToolkit { toolkit: ToolkitSlug, idem: IdemKey },
Answer::Toolkits(Box<ToolkitsAnswer>),

pub enum ToolkitsAnswer {
    Unenrolled,                                  // 还没存 key，这是第一步而不是失败
    Shelf { toolkits: Vec<ToolkitLine> },
    Refused { refusal: Box<AxError> },
}
pub struct ToolkitLine { pub slug: ToolkitSlug, pub name: String,
                         pub auth: String, pub standing: Standing }
pub enum Standing {
    Absent,
    Awaiting { consent_url: String },
    Connected { alias: String },
    Refused { refusal: Box<AxError> },
}
```

**六条口径：**

1. **目录不由本仓库持有。** 目录由 `GET /api/v3/toolkits` 出；客户端里硬写一份应用清单就是一份保证会过期的第二权威。
2. **三态穷尽，而不是一个可能为空的列表。** 「你还没给这座城 key」「问不到对方」「对方确实没有可连的」是人接下来要做的三件不同的事，而一个空 `Vec` 把三句话说成了一句。
3. **`Standing` 四态，每一态对应一个不同的下一步**，且每一态都是对方此刻的读数，不是城记住的东西——昨天打开过一个同意页面，不构成今天已经连上的证据。
4. **不进账本的是状态，进账本的是请求。** `EventKind::ToolkitLinkOpened` 只记「谁在何时请求连接哪个应用」；站位属于 §8-34 判过的「关于此刻的事实」，而 consent URL 是一张能力凭证——记进可重放的账本就等于把它发给每一个重放的人。
5. **同意页面由客户端打开，不由城打开。** 人坐在客户端那一侧；城可能跑在另一个房间的机器上，在那里弹出的浏览器没有人在看。按钮在同一次点击手势里先开一个空白标签页，等答案回来再给它地址——浏览器会拦掉往返之后才发起的弹窗，而按了按钮却什么也没发生的人分不清是弹窗被拦还是城坏了。
6. **全程不轮询。** 页面在三个时刻重读：打开页面、按下连接、以及**从同意页面切回本窗口时**。最后一条是这条流程不需要任何定时器的原因——人离开去授权再回来，「回来」本身就是那个事件。`docs/third-party.md` 边界 2 禁的是「有什么新东西吗」的定时订阅，而带死线的一次握手收尾不是它。

**被否**：让 `ConnectToolkit` 直接把 consent URL 作为命令的答案回去。`Reply` 只运送拒绝（`wire::reply`），把一个成功结果塞进 `AxError` 是为一次往返伪造一条错误路径；URL 由随后的 `Query::Toolkits` 从对方这个「此刻」的权威取回，客户端因而只有一条渲染路径而不是两条需要互相对齐的。
-/
