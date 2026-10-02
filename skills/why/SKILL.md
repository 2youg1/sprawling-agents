---
name: why
description: "Use for 'why does X work this way', 'why we picked Y', design rationale, regressions, postmortems, and data-backed thresholds. Sweeps every reachable evidence category — source control, issue tracker, long-form docs, team chat, observability, error tracking, analytics — records what each one returned including nothing, and answers with calibrated confidence and real citations. Use how for runtime behavior."
license: MIT
---

# Why

Investigate motivation and intent. Why was it built this way, what edge cases were considered, what product or operational constraint forced it, and what alternatives were rejected.

Companion to `how`. `how` answers what the code does. `why` answers what forces led to its shape.

> Adapted from pstack — `pstack/skills/why` in [cursor/plugins](https://github.com/cursor/plugins), the skill collection by Lauren Tan (poteto), released under the MIT licence. Modified and adapted by 2youg1, 2026. Acknowledgment and thanks to the upstream and its author. The original fans out one subagent per MCP-backed evidence category. pi has neither sub-agents nor built-in MCP, so the sweep below runs sequentially over whatever tools this environment actually has. The epistemics framework is unchanged and is the part that matters.

## Why this is hard

Historical context is scattered across seven kinds of system, and you cannot tell from the question alone which one holds the answer. So the method is coverage, not minimalism: search every category you can reach, and treat a null result as a finding rather than a dead end. "The issue tracker has no ticket for this threshold" tells the User something real about how the decision was made.

## Operating posture

Work like an investigator reconstructing a case from fragmentary records. When the record is thin, say so.

- **Evidence before narrative.** Collect the pieces, then see what story they support. Never pick a story and recruit evidence for it.
- **Precision over polish.** An exact quote with a citation beats a smooth paraphrase. Any claim must be verifiable in under a minute.
- **Consider what you have not seen.** Your evidence is a sample. Before concluding, ask what you would expect to find if a competing explanation were true, and whether you looked for it.
- **Name the gaps.** A cold thread, an unsearchable source, an unanswered question — document it rather than covering it with an authoritative-sounding guess.
- **Hedge on purpose.** Indirect evidence gets hedged language. The calibration is a feature of the output, not a stylistic choice to be smoothed away.
- **No shortcut by code-reading.** Code tells you what it does and almost never why it exists. Resist inferring intent from shape.

Read `references/epistemics.md` before writing the final answer. It defines the five confidence tiers, the words that carry confidence, the words that hedge, and the calibration check. The output must follow it.

## Step 1. Fix the target and the question

The **target** is a chunk of code, a pattern, a feature, or a named decision. The **question** is usually design rationale, a tradeoff against a rejected alternative, the edge case behind a defensive branch, an external forcing function, why dead-looking code still exists, or a broad archaeological sweep.

If the target is vague, infer it from context — open files, recent edits, what was just discussed — state your reading in one line, and proceed.

## Step 2. Anchor in code

Before searching anything else, build the anchor. It is cheap and every later step needs it: the file paths and line ranges, the key symbols, the recent commits touching them, and the PR numbers those commits reference.

```bash
git blame -L <start>,<end> <file>          # last-touch commits for the exact lines
git log --follow -p -- <file>              # full history with patches, through renames
git log --oneline -20 -- <file>            # recent commits, PR numbers visible in subjects
git log -1 --format=%B <commit>            # full message, for linked tickets
gh pr view <n> --json title,body,author,createdAt,mergedAt,labels,closingIssuesReferences,comments,reviews
```

## Step 3. Sweep the evidence categories

Work the list below **in order**, and write down what each category returned before starting the next. Source control is always available. For the other six, check what this environment actually exposes — configured MCP servers, CLI tools, `web_search` and `fetch_content` for anything with a public URL — and use it. Where nothing is available, that is a gap in the coverage map, not a category you get to skip silently.

1. **Source control** — git, `gh`, code comments, test names. Always run. Best at implementation-time rationale captured during review: PR descriptions stating the problem, review threads debating alternatives, inline comments encoding a constraint, test names that encode the motivating edge case. Most trustworthy, because it is tied to the diff that shipped.
2. **Issue / ticket tracker** — best at the product or business forcing function: customer requests, compliance deadlines, parent-initiative framing, and labels that categorize motivation.
3. **Long-form documents** — PRDs, RFCs, design docs, ADRs, postmortems. Best at written-out design rationale, especially explicit "alternatives considered" sections.
4. **Real-time team chat** — best at deliberation that never reached a document: fire-drill decisions, author-to-reviewer Q&A, and the rationale behind changes too small to warrant a doc. Matters most when the paper trail is thin.
5. **Infrastructure observability** — metrics, monitors, dashboards, traces, incidents. Best when the target reacts to an infra signal: a monitor threshold whose number matches a code constant, a metric spike just before the merge, a dashboard created as a postmortem action item.
6. **Error / exception tracking** — best for defensive code: stack traces through the target function, issues whose first-seen window brackets the ship date, an error trajectory that stops at a specific release.
7. **Product analytics warehouse** — best for flag-gated code, experiment-driven ships, and "where did this number come from": a usage curve ramping from zero on ship day, or a threshold matching the p99 of a real column.

**Do not skip by anticipation.** "Long-form docs probably don't have this" is not a reason; run the search and let the null speak. Only two justifications belong in the output: the category has no reachable tool here, or the target provably has no such surface — for example, error tracking against a build-time script with no runtime path.

If the target is a single commit whose PR description already contains the complete answer, you may answer inline, but only after saying explicitly that the remaining searches would be redundant. This should be rare.

## Step 4. Synthesize

Sort every claim into a confidence tier from `references/epistemics.md`, then write the output below. Run the calibration check in that file before delivering, and do not drop hedges to sound more authoritative — preventing exactly that is why this skill exists.

**The question.** Restate it concisely.

**The code in question.** Paths, line ranges, key symbols. One or two lines, so the reader is anchored.

**What we found.** Direct evidence only. Every bullet carries a citation: PR number, ticket ID, doc URL, chat permalink, commit hash, or a code comment with `file:line`.

**What we can reasonably infer.** Claims supported by converging indirect evidence. Each bullet makes its chain explicit — "given A and B, C is likely because D" — in hedged language.

**Competing hypotheses.** When the evidence fits several stories, give each one its evidence for and against. Do not force a winner the record does not support. Skip this section when the answer is clear.

**What we don't know.** The concrete gaps: what you were trying to answer, what you searched, what you searched for, and what came back. "We searched the tracker for 'rate limit' and found no ticket discussing this threshold" is worth far more than "we don't know."

**Sources consulted.** One line per category, including the empty ones, formatted as `- <category>: <what was searched>. <what was found, or "no relevant results", or "skipped. reason">.` This coverage map is what lets the User judge breadth and redirect you.

When the question is a precursor to changing the code, close by converting the findings into a Preserve / Change / Avoid / Risk constraint set for the change.

## Failure modes

- **Confident storytelling.** A plausible narrative from thin evidence. An uncited bullet belongs in "inferred" or "hypotheses", never in "what we found".
- **Citing code as evidence of its own intent.** "It handles null because it checks for null" is mechanics, not motivation.
- **Recency bias.** The newest commit is not authoritative; the current shape is usually accreted. Trace back.
- **Sycophantic agreement.** When the User's question embeds a hypothesis ("I assume this is for performance?"), treat it as one candidate and check it independently.
- **Skipping the gaps section.** The honest accounting of what you could not find out is part of the value.
- **Skipping a search by anticipation.** A null result is a data point; an unrun search is a blind spot.
