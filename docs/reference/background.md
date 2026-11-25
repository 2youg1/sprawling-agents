# Why I built this

[简体中文](background.zh-CN.md) · [Project home](../../README.md)

I do not want to sit in front of a computer 24/7 until the five-hour quota wall hits and I finally go to sleep. Neither do you, most likely.

I have tried many harnesses. Some are conceptually behind; others overshoot what is actually useful. Take RSI: until the LLM itself leaves the stateless regime, all a harness can do is keep adapting to the newest model, and learn a company's existing process and run it faster. The first looks like an ablation study; the second needs privacy first.

More and more small companies are a few people delivering an online service with a large group of agents. Taken apart, 99% of their core asset is a well-worked set of Markdown plus a few smart people.

So I wanted a harness that keeps up with the multi-agent (graph engineering) wave and stays pragmatic about RSI and the memory fashion. I designed it around pragmatic extensibility, saving the user's attention, controlling the cost of agent scale-up, privacy and reliability, and long unattended operation. A little urban studies and sociology went into the mix, and that is how sprawling took shape.

The stronger and more numerous the agents, the more expensive human attention gets. I refuse to let sprawling become another app that tries to hijack yours. Most harnesses spend their effort writing prompts; I think sprawling is best used by turning to the loop. Design the workflow, let the agents develop and maintain sprawling itself, take over your fixed work, and go design new business, learn new things, and look in on the city occasionally.

Honestly, no multi-agent scheme yet delivers gains that justify the cost after scale-up. But research on this technology for business automation, social simulation and AI alignment is only beginning. We still need to invest considerable effort and resources in observing and studying how models interact and collaborate inside an agent cluster, and what social behaviour they exhibit.

Agent memory is one road to RSI, but not by black-box injection inside the harness. Your files, code and document libraries are the memory. Before LLMs leave the stateless regime, most attempts to make an agent truly grow with you only drag the model down.

If you already have a harness you like, any harness that speaks ACP can join the city as a resident ([integrations](../integrate/integrations.md)). Beyond migrating the business skills, MCP and ACP you need, I suggest staying lean and adding rules by hand when a concrete problem shows up, since the same model behaves completely differently under a different harness. [Kasanagi](../../Kasanagi/README.md), another project I am preparing, helps build chat software, and will later work as an MCP server for remote collaboration between several sprawling instances.

Two more things are plain reality. My own computer is modest, so I will not let multi-agent work grow its cost exponentially. That also lets sprawling run on an old laptop or a cheap cloud box. I do not sell APIs and cannot afford a disk full of your data, so data stays local by default; the city has a confidential building that calls no remote model and starts no MCP server, and with a local model it can hold private data. The price is that I cannot run very large tests in the cloud.

<p align="center">
  <a href="../images/ai-jiahao.webp"><img alt="AI 嘉豪: looks very cutting-edge, actually accomplished nothing" src="../images/ai-jiahao.webp" width="90%"></a>
</p>

sprawling is aimed at persistent operations for small teams, and at research platforms in computer science or the humanities and social sciences. It is still in development: contributions are welcome, and so is a conversation through an Issue or by email.
