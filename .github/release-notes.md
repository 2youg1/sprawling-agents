⚠️ **<!-- xtask:begin maturity:titled -->Alpha<!-- xtask:end -->.** sprawling <!-- xtask:begin workspace_version -->0.0.16<!-- xtask:end --> WONDERER is usable / 可用. This release moves the wire: WIRE_V goes from 74 to 79, so a browser tab left open by 0.0.15 is refused rather than misread — reload it against the new city / 这一版移动了 wire：WIRE_V 从 74 升到 79，0.0.15 留下的浏览器标签页会被明确拒绝而不是读错，对新城市重新加载即可. The saved-data formats do not change: EVENT_LOG_V stays 2 and the configuration formats stay at format 1, so a city written by 0.0.15 opens without a new migration / 已保存数据格式不变，EVENT_LOG_V 保持 2、配置格式不变，0.0.15 写下的城市不需要新的迁移即可打开. Before replacing an installed binary, stop the city and verify a restored pre-update backup ([update guide](https://github.com/2youg1/sprawling-agents/blob/main/docs/getting-started.md#updating) / [更新指南](https://github.com/2youg1/sprawling-agents/blob/main/docs/getting-started.zh-CN.md#更新)).

**Windows signing / Windows 签名** — The Windows binary of this release is still not signed; the SignPath Foundation has not issued the project's certificate. The release workflow already signs through SignPath once the certificate and policy are available; see the [Code signing policy](https://github.com/2youg1/sprawling-agents#code-signing-policy). 本版 Windows 二进制仍未签名，SignPath Foundation 尚未为本项目签发证书。证书与签名策略就绪后，发布流程会通过 SignPath 签名；见[代码签名政策（Code signing policy）](https://github.com/2youg1/sprawling-agents#code-signing-policy)。

---

欢迎来到Sprawling v0.0.16 Alpha 261110 （WONDERER）！
目前个人开发的经济与精力变得更加缺乏了，不过进度喜人。这个版本最重要的内容当然是一系列前端优化，现在的感觉相比于之前好太多了，到了一个我基本愿意用的程度。默认字体从Maple Mono又换成了基于Geist Mono的Paper Mono，我蛮喜欢它带来的表现。我不追求花哨，夸张的设计，我只想让整体的表现卡在比彻底无聊多一点的程度上。Agentic Coding就像老虎机/パチンコ，尤其是在重置的奖励与FOMO+就业焦虑的惩罚中……我看到了令人痛心的开发者猝死事件，让我想起数年前读过的韩炳哲讨论功绩社会的自我规训，在过去我听说过大厂审核员被主管PUA加班猝死，现在则是被手下的“代理”吹捧而死。有两句出处不同的句子经常连在一起：君以此始，必以此终；求仁得仁，又何怨乎？在Agent时代不断对生产力提升的痴迷和Onlyfans色情网红产业难道不是很相似的事情吗？头部创作者、职业导师、流量课程、总有人愿意做你不想做的事情、甚至做到你不敢想的程度……然后退回一步，可能十米之内的另一个人还在研究怎么网页端大模型破甲。一定程度上来说，在这一版之前的前端没有无聊感，而是残破感，现在才是往无聊开始发展。说实话，一个无聊的WebUI不会对这个环境产生一分一毫的影响，这个世界上的一切都在想方设法抢夺你已被分完的注意力，只能说现在有一种想朝这个方向做的欲望，未来我当然也会允许你让它不那么无聊（或者在迁往Leptos的过程中引入完全的International Typographic Style，其实做Brutalism也可以，但我明确讨厌Neubrutalism设计）。不过反过来说现在的Harness还有什么自由之说吗？Frok一个仓库，直接修改代码，或者从0开始，别忘了LLM能力成长的速度是指数，而不是大部人类那样的对数。就在今天发布的Claude Fable 5.2连制作操作系统的能力都有了（见https://www.anthropic.com/claude-fable-and-mythos-5-2），什么维护，兼容，不就都只是钱的问题吗（当然就算是2022年之前，只要有钱维护和兼容也不是问题）？
在转用Cursor之后，我开始认为没必要继续再深入专研生产力方面，归根结底怎么样都不可能在这方面做得更好了吧。在我看来，以单Agent为主的生产力方向上，Cursor已经做到了几乎满分（主要扣分项来自于恼人的卡顿和冷加载缓慢问题），难怪说为什么我没听见哨声，原来起跑线就是重点线啊。另外一边，Cursor的体验也给了我新的启发，也许我们并不需要那么长的上下文，而是需要好的组织工作和交接。比起有效注意力和最大上下文极限，我更认为缓存折扣率和持续时间更多决定了实际体验。如果把信息的筛选、更新做好，在我看来就算只有256K上下文，只要让1h缓存即将过期前自动压缩开新会话，复杂任务和信息整理全部交由一次性子代理或专门工作区的子代理，就能在提供近乎完美的Bot体验的同时做到比目前其它方案更经济（当然这里要排除有官方补贴和额外维护的那些由大厂支持的Personal Agent，当然也不排除这些闭源产品的方案其实做得就和我说的差不多，我这里对比的是这些新Personal Agent出现之前的各种方案，据我所知现在它们依然流行，我大量看到一些Harness扩展整合包使用着昂贵低效、而且本质上逻辑有漏洞的传统方案）。在这个版本中我将子代理允许主Agent基于任务难度分三档使用模型，未来也许就可以组建这样的团队：让交流最亲人的Gemini作为主Agent，让Claude去处理最麻烦的任务，GPT负责验收和小型任务，一些速度快且便宜的开源模型负责一些Work工作，一个部署的本地小模型用于处理隐私内容……更关键是，这将会是一直可以变动的团队，你不再需要担心“生态”的问题。当你在某一个生态内，补贴和便利的反面就是圈养（Grok Bot真的是一个非常非常神奇的产品，但它就这样以很低的补贴从我口袋里每个月拿走30刀，幸好我也就这么点），如果Agent可以轻松地获得你最私密的聊天软件的记录，当然它会有你更多的信息（但不一定更“懂你”），但这也同时意味着厂商极低概率不记录并审核你的聊天记录并将你的反馈用于模型训练，同时厂商的广告业务和全生态订阅将会与你长期绑定成一笔难以摆脱的开销。Grok Bot很好，Cursor更好，我用得很开心，开着Auto忘记什么模型什么思考强度把一切交给它们的工程师（当然严格来说Cursor只是极快速以90%的成功率解决了我发现的BUG，并且第一次推送PR的CI一定红至少两处，对比一下Kimi K3.1Pro+Pi除了又贵又慢，每次推上去99%成功+CI绿，不过二者反正都是要被AI review打回二进宫的，最终只是比谁更便宜，唯一拉出差距的只有Claude明显质量更高，不过Cursor经常发生抢工作区打架和忘记子代理问题，也许需要更强大的版本控制器和更快的CI，当然更重要的是更好的Grok），但我还是强迫自己必须开发一个在电脑上让我每天使用的Harness，因为无论如何我不想把一切绑定在一个特定的生态上，更何况这两个Harness都闭源（或者说我太喜欢这些产品了，因而它必须得有一些开源的变体，而其中一种由我设计，尤其是确保不会未来某一天加入一个我不喜欢的功能还放在主页。而且严格来说现在大部分提供订阅的厂家都是在用补贴吸引用户，严格来说这在商业上不可持续，纯粹是和对手比谁先撑不住或者谁先发现了挖掘用户价值的新领域。很多闭源产品在用户感觉不对时其实也不好检查底层的模型，思考强度和补贴量，甚至严格来说，由于真实CoT并不提供所以实际上无法验证实际消费Token量）。我始终得有一个我理解的Harness允许我控制一切，我认为这就是Sprawling Agents，它足够你即使只懂编程也能自由地尝试各种疯狂的想法。
这个版本的测试主要基于Deeepseek V4.1 Flash 1017进行，目前LGTM(大体流程为：下载安装包并解压到喜欢的文件夹→双击"sprawling.exe"→Enter打开TUI，输入/w，点击Tab，再次Enter打开WebUI→选择Provider并填入Key，点击获取模型列表，选择模型打勾并保存，并将常用模型设置为Main→将喜欢交流的对象设置为Mayor，并将设定填入Mayor.md→点击“hall/mayor”选择城市，新建/导入工作区→点击进这栋楼→左下角新建房间→各种直接测试)。这个模型TPS很高因而适合用于测试，但我经常诟病的低Token效率问题似乎在Sprawling Agents中更加严重了，这个模型在家族中前所未有地有主见，但又没有治好过度思考和自我打断问题。也许是内置提示词不够好导致了这种问题，回头我会仔细排查一下。另外一边基于Memo.md的记忆在这个模型上表现得很差，像Claude Sonnet 3.7的表现一样（见https://www.anthropic.com/research/components-of-agent-memory），1017记忆的频率和对重点的把握很差，记忆的风格非常的繁琐。这就导致了后续查找记忆无论是关键词还是语义都非常麻烦，直接阅读非常浪费上下文（别忘了低token效率）。就算提前准备好了一个大纲1017的注意力也不够在当前的任务和过去的记录中良好地分配权重，进一步的导致它没有办法充分理解用户的指令，在极度严重的过度思考中变得偏执（这也是奖励黑客问题的一个侧面），看它的CoT是一种折磨……总而言之，我针对Deepseek做了少量修复（因为严格来说还有一些工具调用失败问题需要更深入地排除和对比不同厂商的模型进行测试），但没有针对1017做适配，也许那些基于1017开发的扩展搭配DSH让它表现得很好（当然更有可能大部分并没有让它变得更强而是短期看起来更好）。目前在开源模型中我依然推荐非官方Harness优先使用Kimi，它的适应力最强。当然内置提示词没优化是很大的问题，我一定会尽可能快的抽出时间做相关的研究和优化，根据我长期使用极简Harness的经验（我用每个Harness都只在缺失时补联网扩展，SKILL除了当AGENTS.md补充之外都不加，剩下一律保持出产设置，甚至开发Sprawling Agents要用的SKILL我全部直接直接塞仓库里方便我换各种Harness，不过Cursor我安装了pstack），越是少的上下文，少而精确的系统提示词就越能对输出产生大的影响（当然这样的影响是难理解，这里就是个例子https://arxiv.org/abs/2610.36449。另外一边，我之前写过一篇介绍CoT的文章是说CoT仅是大模型用于辅助自己让分布更接近能解决任务的一段铺垫Token，当时文章里没有明确说绝对不应该将CoT当作LLM是怎么想的，“它”也不知道自己是怎么想的），甚至有时候只是多一两个工具/SKILL/MCP的工具描述都能很大程度上改变任务推进的方向。
如果想要基于Sprawling Agents进行二次开发最好还是再等1-2个月推出V0.1.0 Beta（我希望不要有什么V0.0.20 Alpha，我自己都要受不了了），当然我也不会拒绝在不断测试打磨完善之外加入新的功能，有什么真实场景，有什么真实需求，欢迎讨论。这次版本的标题是「Wonderer (feat. ReoNa)」，来自于Fate/Grand Order 奏章Ⅲ 新霊長後継戦 アーキタイプ･インセプション TVCM楽曲，说实话剧情已经完全不记得了，可能当时也没完全看懂，但直觉告诉我适合这里。

---

**Notes / 补充**

**Known and unfixed / 已知问题**

一些报错信息处理得过于笼统（尤其是工具调用必须搭配结构化报错，错误信息需要解释和排除建议），日志的信息量不足(输入和状态变化应该有详细记录)，同时有很多译名明显不够易懂，还有一点写在脸上的感觉，需要在接下来的版本逐个深入解决。
目前在一些工作区会有git冲突导致无法记录卡住回复的问题，还需要深入排查和整理修复方案。


---

**Maturity ladder / 成熟度阶梯**

| Stage | Version | Meaning |
|---|---|---|
| pre-alpha | 0.0.x | runs end to end; nothing is promised |
| alpha | 0.0.x | usable / 可用 |
| beta | 0.1.0 | good to use / 好用 |
| stable | 1.0.0 | no compromise on experience or on long-term maintenance, with a mature maintainer or team / 体验与长期维护都不妥协，维护者或团队成熟 |
