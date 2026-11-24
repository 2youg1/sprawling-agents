> **sprawling <!-- xtask:begin workspace_version -->0.18.0<!-- xtask:end -->** 将 Wire 从 80 升到 82。升级前请停止城市并验证备份可恢复，升级后请重载已打开的浏览器标签页。账本格式版本仍为 2，配置仍为格式 1。Windows 构建未签名。
>
> Wire moves from 80 to 82. Stop the city and verify a restorable backup before upgrading, then reload any open browser tabs. The Ledger stays at format 2 and configuration stays at format 1. Windows builds are unsigned.

---

欢迎来到 sprawling v0.18.0（MARGINALIA）！

这个版本的头等大事还是前端。最早用的是 Solid 1，后来换到 Svelte 5，现在 SolidJS 2.0 正式发布，又换回来了。Solid 2 把异步数据也放进了细粒度响应式里，等新数据回来时可以继续显示手上的内容，状态变了也只更新用到它的地方。之前也评估过 Leptos，RefRain 现在的编辑和阅读已经接了 CodeMirror 和 PDF.js，换 Leptos 还要多处理一层 Rust/Wasm 和 JavaScript 的互操作。Rust 内核和浏览器现在通过 Wire 各做各的事，我更愿意沿用这个边界，把精力留给读写体验。

我平时要读大量的文本和资料，前前后后换了几个开源阅读器，总有些地方用起来不太满意。所以从这一版开始，我准备认真把 RefRain 当成一个阅读器来打磨，至少要做到我自己愿意用。现在可以登记本地文件夹，打开 PDF，下次回来从原来的地方接着读。读到想和 Agent 讨论的公式或图，框出那一块，就可以带进之前的 Session 继续问。当然，做到这几件事离我愿意每天用它读东西还有距离，这也是接下来要花工夫的地方。

然后说说 Cursor。单看生产力，我还是觉得它是我用过最好的 Harness。它有顶级的设计师和工程师，产品本身我几乎挑不出毛病。不开源在我这儿只是小事。一个闭源 Harness 要让我留下来，得有好模型，也得有足够的订阅补贴。按我的用法，SpaceXSI 是我用过的主流订阅里补贴最少的，Grok 4.8 等到现在也没等到，The X Subscription 又把我想用的第三方闭源旗舰模型放到订阅额度之外，按 API 原价另算。几件事加起来，一个我挑不出什么毛病的产品，最后也找不到什么理由继续用。这是我对 Cursor 最大的遗憾，也是我想认真打磨 RefRain 生产力的原因。

其实这三件事说的是同一件事。回头看 2025 年，很多 Harness 的交互从文件开始，你先打开一份文件，再决定要让 Agent 做什么。到了 2026 年，大家从 Chat 那里吸取了不少灵感，Code 和 Work 也逐渐走到一起，做一件事往往先开一个 Session。但一篇文章可以读几天、改几周，一次任务也可能涉及好几份文件，两边都需要一个能回去的地方。我想在 RefRain 里同时把这两种体验做好。可以从一份文件开始读和改，也可以从一个 Session 开始交代任务，需要看原文的时候就回到原文。

另外，Mei 还在继续开发，接下来一段时间大概会更多地打磨已有的东西，做些优化。Mei 的承诺一直是极简主义，没必要的功能就不加。Calyx 也会一起打磨，它现在是 sprawling 和 Mei 共用的终端壳。以后也许可以让更多 Harness 用上它。

---

**In English / 英文版**

Welcome to sprawling v0.18.0 (MARGINALIA)!

The main thing in this release is still the frontend. We started on Solid 1, moved to Svelte 5, and now that SolidJS 2.0 is out, we're back. Solid 2 brings asynchronous data into its fine-grained reactive system. You can keep the current view on screen while new data is loading, and when a value changes, the parts that use it update. I also evaluated Leptos. RefRain already uses CodeMirror and PDF.js for editing and reading, and switching to Leptos would add a layer of Rust/Wasm and JavaScript interop to those integrations. The Rust core and the browser already have a clear boundary in the wire protocol. I'd keep that boundary and spend the effort on reading and editing.

I read a lot of text and reference material. I've been through several open-source readers, and there was always something I wasn't quite happy with. So starting with this release, I want to take RefRain seriously as a reader and get it to a point where I'd use it myself. You can now register a local folder, open a PDF, and pick up where you left off next time. If you come across a formula or figure you want to discuss with an Agent, select that part of the page and bring it back into the earlier session. Of course, being able to do these things is some way from having a reader I'd choose every day. That's where the work goes next.

Then there's Cursor. In terms of productivity, I still think it's the best Harness I've used. It has first-rate designers and engineers, and I can hardly find fault with the product itself. Being closed source is a small thing to me. For a closed-source Harness to keep me, it needs good models and a subscription that covers enough of the work. For the way I use these tools, SpaceXSI gives me the least support for model usage of the major subscriptions I've tried. I'm still waiting for Grok 4.8, and the third-party flagship models I want to use in Cursor sit outside The X Subscription's allowance and are billed at API list price. Put that together and I have a product I can barely fault and no real reason to keep using. That's my biggest regret about Cursor, and part of why I want to take productivity in RefRain seriously.

These three things are really about the same thing. Looking back at 2025, a lot of Harness interfaces started from a file. You opened it first and then decided what you wanted the Agent to do. In 2026, more products took their cues from Chat, Code and Work started coming together, and a session often became the place you started a task. A paper can take days to read and weeks to revise, while a task can take you across several files. You need a place to come back to in either case. I want RefRain to do both kinds of work well. You can start from a file and read or edit it, or start a session and hand off a task, then return to the source when you need it.

Mei is still in active development. For a while, most of the work will probably go into refining what's there and making it faster. The promise behind Mei has always been minimalism, which means leaving out features it doesn't need. I'll also spend time on Calyx, the terminal interface shared by sprawling and Mei. Maybe more Harnesses will use it one day.

---

**下载和使用边界 / Downloads and limitations**

sprawling 和 Mei 各提供 Windows x64、macOS arm64 与 Linux x64 musl 归档。Mei 在宿主机直接执行 Python，缺少沙箱、权限确认和撤销。请使用可丢弃的项目副本和单独、可撤销、设有消费上限的 API key。

sprawling and Mei each provide archives for Windows x64, macOS arm64 and Linux x64 musl. Mei runs Python directly on the host and lacks a sandbox, permission prompts and undo. Use a disposable project copy and a separate, revocable API key with a spending limit.
