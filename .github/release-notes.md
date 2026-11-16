> **<!-- xtask:begin maturity:titled -->Alpha<!-- xtask:end --> · sprawling <!-- xtask:begin workspace_version -->0.0.17<!-- xtask:end --> · 可用 / usable.** Wire 与存储格式未变（自 v0.0.16）；升级前停止城市并验证备份可恢复。Windows 二进制仍未签名。
>
> Wire and stored-data formats are unchanged from v0.0.16. Stop the city and verify a restorable backup before upgrading. The Windows binary is unsigned.

---

欢迎来到 sprawling v0.0.17 Alpha 261115（MEI）

这个版本 sprawling 本体没有大的功能变化，主要的更新是仓库里的 Archetype:Mei（在顶层简称为 Mei）。这是一个全新、真正极简的 Harness，目前作为实验性项目，具体大家请看 `Mei/README.md` 吧。现在大厂的 Harness 都在往大而全去设计，或者说设计生态，一开始 sprawling 的想法是克制，但做着逐渐发现变得越来越大了：在 v0.0.7 Pre-Alpha 260927 (CITIOR) 时 Windows 包体只有 6MB，而到了 v0.0.16 Alpha 261110 (WONDERER) 时已达 17.4MB，接近 3 倍。我认为需要回头基于现在 API tool、CLI 以及模型本身编程能力越来越强大的一个基准点，它的目的是尽可能小，同时完成 95% 的任务（不过我相当怀疑其实 Mei 只需要少量优化就可以比 Sprawling 本体在 90% 的任务上强大很多）。我想基于这一个对照反过来思考 sprawling 本身的设计以及做深入的优化。

这个 Harness 的 AX 面只有 Python 一个工具和 255 个字节的内置提示词（见 `Mei/src/MEI.md`），接入 Provider 靠目前大热的 Harness 配置工具 magpie，CLI 复用了 sprawling 本体的 CLI（非常值得优化一下 CLI 表现，但我本人不是什么 CLI 爱好者，实在是不知道怎么下手，之前针对 sprawling 做优化时我想着 CLI 本质是 Web 启动器）。当然，Archetype:Mei 的本体非常欢迎二次开发，我将除了发现新的必要功能或者必要的稳定性/性能优化之外尽可能不改动其代码，我完全允许您（基于 MPL2.0 协议）自由地修改它并开发自己的 Agent 或者大型项目中的一部分，我也很期待用 Mei 开发出的其它好玩的项目（但严格来说你可以不告诉我）。

现在让我们来正式聊一下 V0.1.0 Beta（或者说 sprawling 的“正式版”）的设想，我之前只是初略地聊过，但这次相对成型，不过目前没有准备写成一个带有 milestone 的 roadmap，可能会基于实际能力和资源动态调整：短期内主要解决 Issue 71–73、83、101、112、220，针对任务完成能力和成本进行针对性优化，继续优化内置提示词以及自定义的自由度（目前有些入口不是很方便，而且调起来不够明显），看情况把 phone use 做了。中期计划主要是做成一个 monorepo：1，用一个根 Cargo Workspace 管理 Rust 产品 crate 与开发工具，共享 `Cargo.lock`、`target/`、依赖注册和机械规范，删除笼统的 crates 目录，重新将现有内容分到 `Sprawling/` 与 `Tools/`，然后开发一些新的 tools。2，将 `RefRain/` 改为 `RefRain/`，把浏览器壳发展成独立 WebUI，面向重写作业务（办公修订、人文社科长稿改写、科研论文等）的 agent 工作台，同时**不再只针对 sprawling，而是自由接入各种 harness**，让前端本身成为一个工作台（其实就是我之前 RefRain 项目的一个复活，我后面想通了其实 RefRain 本质上就约等于一个前端，当然也没必要做什么 GUI）。到时候可能会有一个模式切换，默认还是目前 sprawling 这样的纯 Vibe 的 Zen 模式，然后添加切换到面向单一文档高频互动小幅修改的 Kara 模式（RefRain 原本的设计）。这一块主要以基于现有 client（Bun、TypeScript、Svelte、Vite、Effect 与 CodeMirror）升级为主，不做大迁移或者重写。3，在完成了 Issue 92–97 以及一系列安全升级之后，基于项目重新将 kasanagi 完整迁入顶层，继续维护 CLI、npm 分发、文档、对抗测试和未来的 crates.io 发布，并把“上手到第一条验证过的消息”做得足够快，让 Agent 用 CLI 或 MCP 进行事件式互通。这里一方面是提供一个足够安全的加密交流软件的替代（这也意味着 Issue 80 我会在随后关闭，一方面是 NapCat 收到了律师函，另一方面我意识到作为 PersonalAgent 的 QQ Bot 核心是要 Bot 而不是 QQ，甚至没有 QQ 更好），kasanagi 会针对侵入和社会工程攻击做专门的优化，让 PersonalAgent 足够 Personal。合并 monorepo 之后可以让根安全策略覆盖三个产品，供应链检查和安全修复更方便地进行，而且我的工作流也已经做到了 sprawling agents 仓库内（见 AGENTS.md），多仓库开发非常繁琐，一些开发工具我也懒得多仓库移动来移动去……目前还没有长期计划，我个人会定位在表现稳定可靠，当然可能回头还会把 CLI 单独提出来大做特做也不一定，只不过目前是一点点想法都没有，如果还有不错的点子或者有意思的功能，我也会做一下。权当学习大项目管理了，如果有好的经验，欢迎传授给我。

欢迎参与到项目开发中。

---

**In English / 英文版**

Welcome to sprawling v0.0.17 Alpha 261115 (MEI)!

There are no major functional changes to sprawling itself in this release. The main addition is Archetype:Mei in the repository, usually shortened to Mei at the top level. It is a new, genuinely minimal Harness and is currently an experimental project; see `Mei/README.md` for details. Large vendors are designing their Harnesses to do everything, or designing ecosystems, while sprawling began with restraint. As it grew, I realised it was becoming large itself: the Windows package was only 6MB at v0.0.7 Pre-Alpha 260927 (CITIOR), but had reached 17.4MB at v0.0.16 Alpha 261110 (WONDERER), nearly three times as large. I think we need to go back to a baseline at a time when API tools, CLIs and models' own programming ability keep getting stronger. The aim is to be as small as possible while completing 95% of tasks (though I strongly suspect Mei would only need a little optimisation to outperform sprawling itself on 90% of tasks). I want to use this comparison to rethink sprawling's design and optimise it deeply.

This Harness's AX surface consists of one Python tool and a 255-byte built-in prompt (see `Mei/src/MEI.md`). Providers are connected through the currently popular Harness configuration tool magpie, and the CLI reuses sprawling's own CLI (its presentation deserves optimisation, but I am no CLI enthusiast and honestly do not know where to begin; when optimising sprawling before, I thought of the CLI as essentially a Web launcher). Archetype:Mei itself is very open to secondary development. Apart from newly discovered necessary features and necessary stability or performance work, I intend to change its code as little as possible. You are entirely free, under MPL-2.0, to modify it and build your own Agent or part of a larger project. I look forward to seeing other interesting projects built with Mei (although, strictly speaking, you do not have to tell me).

Now let us talk properly about the idea of V0.1.0 Beta, or sprawling's “formal release”. I have only sketched it before, but this time it is relatively formed. I am not ready to turn it into a roadmap with milestones, and it may be adjusted as actual capacity and resources change. In the short term, the main work is Issues 71–73, 83, 101, 112 and 220; targeted optimisation of task completion and cost; further work on built-in prompts and freedom of customisation (some entry points are inconvenient, and the effects of changing them are not visible enough); and possibly phone use. The medium-term plan is primarily a monorepo. 1. Use one root Cargo Workspace to manage Rust product crates and development tools, sharing `Cargo.lock`, `target/`, dependency registration and mechanical rules; remove the generic crates directory and redistribute its current contents between `Sprawling/` and `Tools/`, then build some new tools. 2. Rename `RefRain/` to `RefRain/` and develop the browser shell into a standalone WebUI: an agent workbench for rewriting work (office revisions, long-form rewriting in the humanities and social sciences, research papers, and so on), while **no longer serving only sprawling, but freely connecting to different Harnesses**, making the frontend a workbench in its own right. This would revive my earlier RefRain project; I later realised RefRain is essentially a frontend, and there is no need for a GUI as such. There may be a mode switch: the default remains sprawling's pure-vibe Zen mode, with an optional Kara mode for frequent interaction and small edits around one document (RefRain's original design). This part would mostly upgrade the existing client (Bun, TypeScript, Svelte, Vite, Effect and CodeMirror), not a large migration or rewrite. 3. After Issues 92–97 and a series of security upgrades, migrate kasanagi fully back to the top level based on its project, and continue maintaining its CLI, npm distribution, documentation, adversarial tests and eventual crates.io publication. Make the path from getting started to the first verified message fast enough, and let Agents exchange events through CLI or MCP. One purpose is to offer a sufficiently secure alternative to encrypted communication software (which also means I will close Issue 80 afterwards: partly because NapCat received a lawyer's letter, and partly because I realised the core of a PersonalAgent QQ Bot is the Bot, not QQ — perhaps no QQ is even better). Kasanagi will be optimised specifically against intrusion and social-engineering attacks, making a PersonalAgent genuinely Personal. Once the monorepo is combined, the root security policy can cover all three products, and supply-chain checks and security fixes become easier. My workflow already lives in the sprawling-agents repository (see AGENTS.md); developing across repositories is cumbersome, and I do not want to keep moving development tools between them. There is no long-term plan yet. My own aim is stable, reliable behaviour, though perhaps I will eventually take the CLI out and build it up substantially — right now I have absolutely no ideas for that. If there are other good ideas or interesting features, I may build those too. Treat this as learning to manage a large project; if you have useful experience, you are welcome to share it.

You are welcome to participate in the project's development.

---

**Mei installation and boundary / 安装与边界**

- npm / npm：`bunx @sprawling/mei`（macOS arm64、Linux x64 musl）
- crates.io：`cargo install sprawling-mei --locked`
- Mei is experimental, POSIX-only and has no sandbox, permission prompts or undo. Use a disposable project copy and a separate, revocable API key with a spending limit; native Windows is not published, so use WSL or another POSIX environment / Mei 是实验性 POSIX 原型，没有沙箱、权限确认或撤销；只使用可丢弃的项目副本和单独、可撤销且设好消费上限的 API key。不提供原生 Windows 包，请使用 WSL 或其它 POSIX 环境。

**Known and unfixed / 已知问题** — Existing diagnostics can still be too broad, logs too sparse, and some workspace git conflicts can still stop a reply from being recorded / 既有诊断仍可能过于笼统、日志信息量不足，部分工作区的 git 冲突仍可能阻止回复记录。

---

**Maturity ladder / 成熟度阶梯**

| Stage | Version | Meaning |
|---|---|---|
| pre-alpha | 0.0.x | runs end to end; nothing is promised |
| alpha | 0.0.x | usable / 可用 |
| beta | 0.1.0 | good to use / 好用 |
| stable | 1.0.0 | no compromise on experience or on long-term maintenance, with a mature maintainer or team / 体验与长期维护都不妥协，维护者或团队成熟 |
