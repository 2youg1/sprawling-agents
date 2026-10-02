-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::answer::github

规定 `answer::github`（`crates/wire/src/` 下同名的文件）。从城那台机器上的 GitHub CLI 读到的一个候选用户 ID，或没有它的那个有名字的原因。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-67 从 GitHub CLI 读一个候选用户 ID：`Query::GithubLogin`

```rust
// Query
GithubLogin(Option<String>),                 // 问哪台主机 → Answer::GithubLogin(GithubLoginAnswer)；None 即 github.com
pub struct GithubLoginAnswer {
    pub host: String,                        // 问的是哪台主机：页面显示它，保存时写进 imported_from
    pub reading: GithubReading,
}
pub enum GithubReading {
    Found { login: String },                 // 这台主机当前认证身份的 login：一个候选，不是已保存的名字
    NoCli,                                   // 服务这座城的机器的搜索路径上没有 gh
    NotLoggedIn,                             // gh 在，这台主机没有登录（gh 的退出码 4）
    Failed { exit: Option<i32> },            // 别的失败：网络、gh 自己的错误；None 是没有退出码（没起来，或超时被停下）
    Stuck { why: String },                   // gh 超时，城停不下它：那个进程还在城的机器上跑，why 是停的时候出了什么错
    NotAHost,                                // host 不是一个主机名，gh 没有被启动
}
```

- **只在人按下时跑。** 这是一条查询，页面在人按「从 GitHub CLI 导入」时问一次；开城、刷新、换页都不问，城也不周期地问。读到的 login 只是候选：页面把它填进「你的 ID」卡，人保存时经 `PutIdentity` 写下（`imported_from` 记 `host`）。答复不改任何文件，所以多账号或环境令牌换了 gh 的当前身份，也不会悄悄改掉已保存的名字。
- **跑在服务这座城的那台机器上。** 城执行 `gh api --hostname <host> user --jq .login`，stdin 为空、`GH_PROMPT_DISABLED=1`，所以 gh 不会停下来等人登录；有上限地等它结束，超时就停下它，答 `Failed { exit: None }`；停不下时答 `Stuck`，因为一个城起了却收不回的进程是人要处理的事实，与「没读到」不同。不需要管理员权限，不读、不记令牌：stdout 只取 login 那一行，stderr 不读。远程设备上的页面问到的是城那台机器的 gh，页面在导入之前说明这一点。
- **主机由人选，默认 `github.com`。** 查询带的主机缺席即 `github.com`，这个默认只写在城一处（`accounting::views::answering::github`）。不是主机名的串（空、以 `-` 开头、带空白或 `/`）答 `NotAHost`，gh 不被启动，所以一个以 `-` 开头的串不会被 gh 读成一个选项。
- **每种失败各有一个名字，页面按它给出路。** `NoCli`：说明装 gh 或手填；`NotLoggedIn`：说明在那台机器上运行 `gh auth login --hostname <host>`，城不替人启动登录；`Failed`：照样留着手填的草稿；`Stuck`：说明那台机器上还有一个 gh 进程；`NotAHost`：说明主机名的写法。没有一种失败清掉卡上已有的值。
- **读的人是视图，跑的人是二进制。** 视图经 `Views::ask_github_through` 收下一个 `fn(&str) -> GithubReading`（与 `ask_upstream_through` 同形），在放开快照之后调用它；生产的那一个是 `bin::doctor::github::login`，找 gh 走 `doctor::host::find_program` 那一条搜索路径，起子进程、停子进程走 doctor 起程序的同一套规矩。没有交这个函数的视图（一次性的 `views::ask`）答 `Unavailable`。
- **`WIRE_V` 不另进位**：`GithubLogin` 是新名字，哈希自己会变（D1）。
- 验收：`bin::doctor::github` 的三条——搜索路径上没有 gh 答 `NoCli`、退出码 4 答 `NotLoggedIn`、退出码 0 加一行 login 答 `Found`；accounting 的 `a_github_login_is_asked_of_the_reader_the_views_were_handed`（缺席的 host 问的是 `github.com`，不是主机名的串不去问）。
-/

/-! D7 GitHub 导入是一条只读查询，由二进制跑 gh；指南进度是一个按城的文件，整份写

**决定**：(a) 从 gh 读用户 ID 是 `Query::GithubLogin`，答一个候选与它来自的主机，不写任何东西；跑 gh 的函数住二进制（`bin::doctor::github`），由服务中的城交给视图（§8-67）。(b) 上手指南的进度是 `Query::Guide` 与 `Command::PutGuide`，存进城保留子树里的 `GUIDE.toml`，整份写、后到者为准、不写账本行（§8-68）。

**理由**：(a) 导入只给出一个候选，保存仍经 `PutIdentity` 与它的基线守卫，所以「谁改了称呼」只有一条路；gh 的当前身份会随多账号、环境令牌而变，一条把它直接写进文件的命令会让一次按键悄悄改掉已保存的名字。起子进程是碰主机的事，按 `crates/accounting/Spec.lean` §12 第 9、10 条它住 `sprawling`，经一个 `fn` 指针交进来；视图在放开快照之后才调用它，所以一次慢的网络请求不挡折叠。(b) 进度要跨浏览器、跨重开，所以在城里；它治理的是这座城给人看什么，所以在保留子树里、没有写域够得到；文件就是线上值的序列化，键名只有一处。它是光标而不是正文，基线守卫只会把一次普通的翻页变成冲突。

**被否**：(a) ①一条命令读 gh 并直接写进 `PREFERENCES.md`：跳过了人对候选的确认与基线守卫；②在开城时读一次 gh：违背「只在人要求时跑」，并让每次开城都碰一次网络；③在 accounting 里直接起 gh：本 crate 的端口规矩（accounting D9）就是为了让 citysim 与测试换得掉碰主机的那一步。(b) ①存进浏览器：换一个浏览器就从头再来；②存进人的 `~/.sprawling/config.toml`：一个人有几座城，每座城的配置不同，进度也不同；③每一步一条命令：进度的形状一变就多几条帧，而整份写的唯一代价是两个浏览器同时翻页时少一个「看过」；④写一行账本：要一个新的事件种类，而界面的位置不是这座城做过的事。

**重开参数**：(a) 页面要列出 gh 已登录的全部主机供人选时，加一条读 `gh auth status` 的查询；(b) 同一座城常有几个人各开一个浏览器、各走各的指南时，进度改为按人存。
-/
