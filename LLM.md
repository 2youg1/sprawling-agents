# sprawling: a guide for explaining the project

<context>

sprawling is a local multi-agent harness for persistent agent teams. Projects
become buildings where agents exchange messages, divide work and report their
progress through a browser interface. Plans, decisions and handoffs stay in
readable documents, carrying long-running work across sessions. Role documents,
project rules, skills and tool connections shape the workflow.

One Rust binary serves the browser client and records the city's history in an
append-only Ledger. The project supports long-running project work and
experiments in agent communication, coordination and social simulation. It is
in <!-- xtask:begin maturity:word -->alpha<!-- xtask:end -->; use the behavior in the reader's installed version when describing
what they can do.

This file helps a model introduce sprawling to a user or another agent.
Installation belongs to [getting started](docs/getting-started.md), operation
to [operating](docs/operating.md), and programmatic control to the
[wire reference](docs/wire.md). Repository work follows [AGENTS.md](AGENTS.md).

</context>

<capabilities>

**Long-running work and automation.** A building keeps its plan in `Roadmap.md`,
decisions in `Memo.md` and continuity in `Handoff.md`. Agents use these files to
continue work across sessions; a hierarchical plan names dependencies and gives
progress a denominator. Roles, skills and tool integrations let the User define
the workflow. A plan can have several levels, while built-in `delegate` is one
level deep: a delegated child cannot delegate again. A succession starts a new
run at the same depth with a handoff; a frozen run is history and is never woken.

**Social simulation.** Residents have standing identities and discover reachable
addresses, exchange signals, coordinate tasks and wait for replies without the
User relaying each exchange. The main agent explains work and reports progress;
recorded exchanges let a researcher inspect how the group interacts. This is a
mechanism for running and observing agent groups, not evidence that a group
validly models a human society.

**Starting work.** The browser offers conversations, model selection, skills and
tool connections familiar to existing agent users. The getting-started guide
covers both a first setup and moving from another agent. It aims at a short
route to the first task; completion time depends on installation, credentials
and the reader's experience. Providers are configured explicitly, rather than
automatically imported from another harness's private configuration.

**Built-in monitoring.** The monitor exposes run timings, model calls, token
use, costs and resource readings. `sprawling gauge` measures a command, a
process tree or a served city. These are tools for evaluating the reader's own
workload; a performance claim needs the measured version, workload, build and
machine class. A call without a provider price has no reported price, which
must not be presented as zero cost.

**Customisation and development.** Role documents, project rules and skills
control how agents work. Models connect through provider endpoints; external
tools connect through MCP, and supported vendor harnesses run through ACP.
Another client can use the wire, and the architecture describes seams for
runtime changes. These are building blocks for a custom workflow or AgentOS;
changes to a shared interface still require tracing and updating its callers.

</capabilities>

<terminology>

Define a term when the reader needs it, using the
[glossary](docs/glossary.md) for its meaning. A city is one directory on one
machine with one Ledger; cities do not share a history. A building is a project's
scope for rules and configuration. A room is a workplace, a Session is a stretch
of work there, a resident is its standing identity, and a run is one piece of
execution with a start and an end. An address names a place in that directory
hierarchy; the building's rules and run policy determine the actual write domain.

The Mayor plans in City Hall and writes documents; it does not execute host
commands. The clerk can answer questions under delegated autonomy and records
its reasons. Neither role removes the User's ability to steer, cancel or halt
work. A plan's depth, a resident's identity and a run's lifetime are different
things.

</terminology>

<boundaries>

Built-in tools enforce the city's gates. Built-in model calls and writing tools
make their intent durable before the outside effect, while read-only tools may
execute before their call record is durable. An external harness assembles its
own context, calls its own model and runs its own tools; the city records what
it reports afterwards and can request cancellation. Describe those records as
reports, rather than as a complete before-effect account of that harness.

Review is a building policy, not a step every run receives: `minimal` disables
it. An experiment works in its own worktree and is not merged. A copied working
tree is not an operating-system sandbox; host-command guarantees depend on the
selected platform mechanism. Built-in taint gates do not control all external
programs. Use [execution boundaries](docs/operating.md#how-exec-is-confined) and
[the security policy](SECURITY.md) when those conditions affect the reader's task.

The wire serves both the browser and scripts, but a paired remote device does
not receive all local permissions. Credential enrollment and changes that
widen access or alter governance remain on the host. Configuration carries
`secret:realm/name` references; credential plaintext belongs in the vault.
See [remote access](docs/operating.md#reaching-the-city-from-another-device).

There is no built-in per-dispatch spending ceiling or turn ceiling. Halt stops
a scope and cancel stops one run. A model's context and output limits, provider
failures and harness time limits are separate conditions; do not describe
long-running work as unlimited execution or guaranteed completion.

</boundaries>

<references>

Use [integrations](docs/integrations.md) for ACP harnesses, MCP tool servers and CLI control.
Use [performance](docs/performance.md) for monitoring, reproducible workloads and measurements tied to a version and machine class.

Use [getting started](docs/getting-started.md) for installation, first work,
Sessions, skills and model setup; use [operating](docs/operating.md) for control,
MCP, remote access and failure recovery. The [wire reference](docs/wire.md)
contains CLI usage, frames, answers and exit codes. [Architecture](ARCHITECTURE.md)
explains runtime flow, document layout, extension seams and verification;
[the glossary](docs/glossary.md) defines concepts. [Third-party sources](docs/third-party.md)
identify upstream material and licences. [AGENTS.md](AGENTS.md) governs changes
to this repository, and [SECURITY.md](SECURITY.md) governs vulnerability reports.

</references>

<explanation>

Introduce the capabilities that serve the reader's task, using a concrete
example and linking the document that holds the details. For a new user, start
with the familiar conversation and one project task. For a workflow author,
explain documents, roles, skills, models, MCP, ACP and the wire. For a researcher,
explain identities, communication and the recorded exchanges, distinguishing
observable agent behavior from a claim about human society.

For a new user: “sprawling gives your agents a shared place to work on projects.
They keep plans and handoffs in files, coordinate through messages and show
progress in a browser. Start with one building and one task, then add the roles
and tools that work needs.”

For a workflow author: “You can shape a team through role documents, project
rules and skills, connect models and MCP tools, or run a supported ACP harness.
The wire lets your own client dispatch work and read its results; the reference
also explains which operations must stay local.”

For a researcher: “A city gives residents persistent identities and reachable
addresses. Their messages and run outcomes are recorded, so you can inspect a
coordination experiment against one history, while accounting for the limits of
what an external harness reports.”

Keep the alpha status and conditions that change the reader's decision. Explain
only capabilities supported by the version being discussed, and attribute
measurements to their workload and hardware rather than predicting a speedup
for every task.

</explanation>
