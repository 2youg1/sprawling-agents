---
name: how
description: "Use for \"how does X work\", code walkthroughs before changing something, and placement / ownership / layering questions (\"where should this live\", \"which package owns this\", \"is this the right layer\"). Explains subsystem architecture, runtime flow, and onboarding mental models, and can critique the architecture it just explained. Use why for motivation, blast-radius for what a change breaks."
license: MIT
---

# How

Answer "how does X work?" by reading the code, then explaining it at the level a senior engineer needs when onboarding onto a subsystem. The product is a working mental model, not annotated source.

Companion skills: `why` explains what forces shaped the code, `blast-radius` explains what a change to it would break.

> Adapted from pstack — `pstack/skills/how` in [cursor/plugins](https://github.com/cursor/plugins), the skill collection by Lauren Tan (poteto), released under the MIT licence. Modified and adapted by 2youg1, 2026. Acknowledgment and thanks to the upstream and its author. The original fans work out to parallel explorer and critic subagents. pi has no sub-agents, so the passes below run sequentially in one context. The cost is real and stated in "Honest limits" at the end.

## Two modes

**Explain** (default) builds the mental model. **Critique** builds it first, then attacks it. Never critique an architecture you have not already explained.

## Explain mode

### Step 1. Fix the scope

Read the question for what kind of answer it wants:

- "How does the rate limiter work?" — one subsystem
- "How do we handle billing for on-demand usage?" — one feature flow across subsystems
- "How is the auth service structured?" — an architectural overview
- "Walk me through what happens when a user submits a form" — a runtime trace

If the target is ambiguous, state your best-guess reading in one line and start working. Do not ask; let the User redirect you if you guessed wrong.

Then size the work. A single module or a narrow "how does function X work" is **simple**: explore and explain in one pass, skip to Step 3. A subsystem spanning several files or services, a cross-cutting feature, or a full overview is **complex**: run Step 2 first. When in doubt, treat it as simple, because you can always widen after the first pass hits a wall.

### Step 2. Sweep the angles (complex questions only)

Split the question into 2–4 angles that do not overlap, and take each one as a separate pass. For "how does the rate limiter work?" the angles might be data model and state, request path and enforcement, then configuration and metrics. The right split depends on the question; use judgment, and stop at 2 angles for a narrow question.

Run the angles **one at a time**, and write findings down before starting the next one. Holding three half-traced call chains in working memory is how details get invented. Each pass follows the same discipline:

- Start broad. Glob the relevant directories; grep the key types, interfaces, and class names.
- Follow the thread. From an entry point, trace the call chain: callers, callees, data flow, type definitions.
- Read the code. Never infer behavior from a file name.
- Stop when you can state the whole path from input to output, or trigger to effect, with no hand-waved step.
- Record what is surprising, non-obvious, or likely to mislead a newcomer.

Between passes, name what the last pass established and what the next pass tests. When two passes disagree, reread the code rather than averaging them.

### Step 3. Explain

Reconcile the passes into one picture and write the explanation. Follow this structure, dropping sections the question does not need.

**Overview.** One or two paragraphs: what it is, what it does, why it exists. Enough for the reader to decide whether to keep reading.

**Key concepts.** The types, services, or abstractions the rest of the explanation depends on. Brief definitions, not an inventory.

**How it works.** The core. Walk the flow: what triggers it, what happens step by step, where data goes, where the decisions are. Write prose, not pseudocode. Cite `file:line` so the reader can go look, and quote code only where a snippet is genuinely load-bearing.

**Where things live.** A short map of the files and directories someone would need to start working here.

**Gotchas.** The non-obvious things that trip people up, the historical accidents that explain why something looks wrong, and the known sharp edges.

## Critique mode

Trigger this when the User asks for architectural problems or improvements rather than understanding.

### Step 1. Explain first

Run explain mode to completion. The explanation is a deliverable in its own right and must stand alone: a reader who only wants to understand the system should never have to wade through critique to get it.

### Step 2. Run the lenses

Read `references/critique-rubric.md` and take **one lens at a time**, in separate passes, recording findings before moving on. The original skill got its adversarial signal from model diversity — several models with different blind spots reviewing the same code. One agent cannot reproduce that. What one agent can do is refuse to carry a conclusion between lenses: enter each pass without the previous pass's verdict, and let the rubric, not momentum, decide what counts as a finding.

### Step 3. Judge

You are a pragmatic lead, not an aggregator. Sort every finding into one of four buckets:

- **Act on.** Architectural problems worth fixing now.
- **Consider.** Real concerns whose cost/benefit is unclear.
- **Noted.** Valid observations, low priority.
- **Dismissed.** Wrong, missing context, or a style preference. Say which.

Present the explanation first, then the verdict below it.

## Honest limits

Say this out loud when it applies, rather than letting the output imply coverage it does not have:

- Critique mode here is one model applying several lenses, not several models disagreeing. Findings that depend on a blind spot you share with the rubric will not surface. For a high-stakes architecture, ask a second model the same question separately.
- Sequential angle sweeps cost context. On a large subsystem you may run out before the last angle. When that happens, report which angles you covered and which you did not, rather than thinning every angle to fit.
