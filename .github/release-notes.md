⚠️ **Pre-alpha.** sprawling 0.0.7 runs a city end to end, but data formats, the wire and the screens still change between versions, and nothing here is promised to keep working. Try it on a throwaway directory.

⚠️ **Pre-alpha。** 0.0.7 已经能从头到尾跑起一座城，但数据格式、协议和界面在版本之间仍会变，这里的一切都不保证继续可用。请在一个可以丢掉的目录里试。

**What this version is for / 这一版做什么** — in the author's words, below in Chinese: this release optimises as much as possible for experiments at scale, and adds the tools common work needs, such as survey and screenshot. The author's note on why, and where the project is going, follows.

---

这个版本的核心是为了规模化的实验做尽可能多的优化，并为一些常见工作提供必要的工具，例如survey/screenshot。九月的风气和八月完全不同，我越发相信少即是多，那些设计编排工作流的harness，尤其是/goal和/deepresearch，如果半年后还持续维护的话，这些现在覆盖率极高的Scaffold90%都会拆除。sprawling的核心是探索规模化拓展Agent时的涌现行为，在我看来，当你越有一个好的Scaffold，你的成本-智力曲线就越接近对数函数，而harness提供的自由度越高，就越接近一个原点在x轴下方的二次/一次函数（取决于任务类型），当它们越过x轴（也就是互相打架以及一些表现不佳的局部最优）时，增长的速度就已经大幅领先了。归根结底，LLM训练语料里面早已包含任何一个管理学家一生都读不完的资料和高管一生看不完的案例，然后我们居然还要浪费Token教它们怎么像商业公司/封建王朝一样运作而不是思考人类目前主流组织形式的低效是否不适合拓张LLM的规模协作能力。现在我们还需要关注的是另一件事，OpenAI用一万个Agent解决NS方程，不过一个月500美元的套餐包含了传言能提供1Ktps跑在Cerebars上面的Sol，现实的趋势是前沿智能的成本越来越低而使用的成本越来越高：一年前大家很少使用Agent因为API的价格高昂能做得也不多，现在越来越多人开始使用agent swarm，一年前少有人AI订阅开支大于200美元，而现在大部分中型项目维护者/小公司的人均月开销超过500美元，到了明年那些还在维护的成熟项目普遍都必须能一天更新几次版本解决上百个PR/Issue，人均AI开销在1-1.5K美元上下（做个对比，像Theo3这样的名人现在开着5个Claude200刀的套餐，同时还有OpenAI的模型测试资格），这个数字已经是很多大厂外包程序员的月薪。那么人工会重新变得有价值吗？就目前Opus5.5的表现来看，Agent相比于人工的性价比越来越高，而且AI拓展规模并不遵循人月神话——甚至一定程度上AI甚至可以进行颅内交流（考虑到可能就在一个机房里面）更大程度节省成本。而且想象一下假如你的项目需要结局A和B两方面问题，你有一个很优秀的员工很会做A，那么他可能就没那么擅长B导致你需要另一个擅长B同时又和A合得来的人，而LLM很会做A，还很会做B，更关键是如果A/B工作量大它可以自我复制而人不行，想象一下两个在某个领域很有经验却互不认识的人很大概率在具体细节上有着各种各样的冲突，往往这些人还会更难协作或过度妥协跳过交流，但LLM就连交流/文书工作都不会偷懒。

对于我个人而言是现在回到社科领域去考研（搞了半年Agent）还是找个实习（我又是文科生）都很困难，创作和开发Agent的经历似乎也没那么突出，而且我还没做好选择。另外一边，这个项目有相当大的希望可以在一周内结束pre-alpha，我希望当我宣布进入alpha的时候项目进入可用状态，宣布进入Beta的时候项目进入好用状态，届时会是V0.1.0，宣布进入V1.0.0的时候应该是一个体验和长期维护都不妥协的状态，同时我个人或者项目继任者/协作团队进入成熟。我没有把V1.0.0放得太低主要是模型的进步会很方便我们在半年之后咬咬牙以一定的成本让体验变得不错，但如果没有团队和维护人员的稳定那么这种不错的体验不会变成真正稳定的体验，那些没人愿意长期投入精力/资金维护的项目未来可能连语料都不一定配当。

如果你可以帮到我或者项目，欢迎在我主页的邮箱联系我。

---

**Maturity ladder / 成熟度阶梯**

| Stage | Version | Meaning |
|---|---|---|
| pre-alpha | 0.0.x | runs end to end; nothing is promised |
| alpha | 0.0.x | usable / 可用 |
| beta | 0.1.0 | good to use / 好用 |
| stable | 1.0.0 | no compromise on experience or on long-term maintenance, with a mature maintainer or team / 体验与长期维护都不妥协，维护者或团队成熟 |
