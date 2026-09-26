<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // One run, read as a stretch of conversation: what the person asked,
  // what the resident said turn by turn, what it did folded into one
  // quiet line per turn, and - while it is still going - the words as
  // they arrive. The rounds come from the server; only the live text and
  // the posture come from what this page has folded itself.
  //
  // Every entry a person could have branched the conversation at - their
  // own words, a reply, a call - carries the one fork action. Hovering
  // or focusing shows it; the `fork.here` chord answers for the entry
  // under the hand (roadmap S2, ux A7).
  import { QUERIES } from "../../core/asking";
  import type { RunBelief } from "../../core/belief";
  import { keymap } from "../../core/keys";
  import { fill, say } from "../../core/lang";
  import { providerClause } from "../../core/provider_failure";
  import { toFragment } from "../../core/route";
  import { clock, count, usd } from "../../core/time";
  import type { Snippet } from "svelte";
  import type { Note, RoundsAnswer, RunId, Turn } from "../../wire";
  import { ui } from "../../ui";
  import Button from "../parts/button.svelte";
  import Prose from "../prose.svelte";
  import Calls from "./calls.svelte";
  import ForkButton from "./fork_button.svelte";
  import { callWord } from "./calls";
  import { planFork } from "./forking";
  import type { ForkEntry, ForkPlan } from "./forking";

  // The provider's own words for a reply that ended the way replies end.
  // Anything else - the ceiling, and whatever a provider adds next - is
  // a reply that was cut off, and a reader is told so.
  const FINISHED: readonly string[] = ["end_turn", "tool_use"];

  // How many characters at the growing edge are drawn faint. Wide enough
  // that text emerges instead of appearing, narrow enough that the band a
  // reader's eye sits on is not the shimmering one.
  const EDGE = 10;

  interface Props {
    readonly run: RunBelief;
    readonly who: string;
    // Absent where the page holding the thread cannot branch: the run
    // page draws the same conversation with nowhere to fork to.
    readonly onFork?: ((plan: ForkPlan) => void) | undefined;
    // The run came back with no words: say the task again as a new run.
    // Absent for the same reason.
    readonly onRetry?: ((task: string) => void) | undefined;
  }

  const { run, who, onFork, onRetry }: Props = $props();

  const u = ui();
  const { lang } = u;

  // The entry under the hand - hovered or focused - which is the one the
  // `fork.here` chord branches from. Cleared as the hand leaves, so a
  // chord pressed pages away can never reach a stale entry.
  let active: ForkEntry | null = $state(null);
  const hoverFork = (entry: ForkEntry | null): void => {
    active = entry;
  };

  // The run this thread reads, named once: the page draws one thread
  // per run and keys it to that run, so the rounds question is minted
  // once for this run and follows this id for the thread's life.
  // svelte-ignore state_referenced_locally (the capture is the point: the question is minted once, for the run this thread is keyed to)
  const readRun: RunId = run.run;
  const rounds = u.conn.asking.ask({ rounds: { run: readRun } });
  const answer = $derived.by((): RoundsAnswer | undefined => {
    const held = $rounds;
    return held !== undefined && "rounds" in held ? held.rounds : undefined;
  });
  const turns = $derived(answer?.turns ?? []);
  const task = $derived(answer?.opening?.task ?? run.task ?? "");
  // The opening task is a person's words like any other, and rides the
  // first turn - its turn's parent - because the wire gives it no line
  // of its own.
  const taskEntry = $derived.by((): ForkEntry | null => {
    const first = turns.at(0);
    return first === undefined ? null : { kind: "message", turn: first, text: task };
  });
  // What this city told the provider a reply may be at most. Read here
  // rather than per turn, because it is one fact about the city and not
  // one fact about a turn.
  const endpoints = u.conn.asking.ask(QUERIES.endpoints);
  const ceiling = $derived.by((): number | null => {
    const held = $endpoints;
    if (held === undefined || !("endpoints" in held)) return null;
    return held.endpoints.chosen.find((each) => each.tag === "main")?.max_output_tokens ?? null;
  });
  const frozen = $derived(run.doing.kind === "frozen");
  const completion = $derived.by((): string => {
    const word =
      answer?.closing?.completion ??
      (run.doing.kind === "frozen" ? run.doing.completion : null);
    switch (word) {
      case "done":
        return say($lang, "outcome_done");
      case "cancelled":
        return say($lang, "outcome_cancelled");
      case "limit":
        return say($lang, "outcome_limit");
      case null:
        return say($lang, "outcome_frozen");
      default:
        return word;
    }
  });
  // The turn being said right now is not yet in the rounds; it is the
  // page's own text until the record holds it.
  const streaming = $derived(!frozen && run.saying.length > 0);
  // The last few characters are drawn faint, so text emerges rather than
  // appearing. Derived from the text and nothing else: no timer, no
  // queue, no per-character node. When the call returns, belief clears
  // `saying` and the settled record takes over, so nothing has to decide
  // when the edge stops being an edge.
  const settled = $derived(run.saying.slice(0, -EDGE));
  const edge = $derived(run.saying.slice(-EDGE));
  const closedAt = $derived(answer?.closing?.at ?? null);
  // A frozen run that said nothing at all is one card with the reason
  // and a way out, not one grey box per turn (ux: the zero-output run).
  const emptyRun = $derived(frozen && turns.every((turn) => (turn.said ?? "") === ""));
  const why = $derived.by((): string | null => {
    const stopped = turns.at(-1)?.stopped ?? null;
    return stopped === null || FINISHED.includes(stopped) ? null : stopped;
  });
  // What the run is doing, in words. A run this page knows is live and
  // cannot place says it is working and claims nothing about the phase.
  const posture = $derived.by((): string => {
    const doing = run.doing;
    switch (doing.kind) {
      case "thinking":
        return fill(say($lang, "talk_thinking"), { who });
      case "calling":
        return doing.tool === null
          ? say($lang, "run_doing_calling")
          : callWord(doing.tool, doing.subject);
      case "waiting":
        return say($lang, "talk_waiting_you");
      case "unknown":
        return say($lang, "city_at_work");
      case "frozen":
        return "";
    }
  });

  // One note's own line in the Ledger, which is what a list of notes is
  // keyed on: every variant carries one.
  function noteAt(note: Note): number {
    if ("arrived" in note) return note.arrived.at;
    if ("refused" in note) return note.refused.at;
    if ("fenced" in note) return note.fenced.at;
    if ("waiting" in note) return note.waiting.at;
    if ("unreadable" in note) return note.unreadable.at;
    return note.discarded.at;
  }

  // The branch a call's action asks for: the entry carries the turn it
  // sits in, so the plan is built here where both are in hand.
  function planCall(entry: ForkEntry): void {
    onFork?.(planFork(run.run, entry));
  }

  // The press the key table judges: where it landed decides whether a
  // single letter is the shell's or the person's typing. This mirrors
  // the shell's own reading of `KeyboardEvent.target`; the rule itself
  // lives in `core/keys`' `matches`.
  function inField(target: EventTarget | null): boolean {
    return (
      target instanceof HTMLInputElement ||
      target instanceof HTMLTextAreaElement ||
      target instanceof HTMLSelectElement ||
      (target instanceof HTMLElement && target.isContentEditable)
    );
  }

  function forkKey(event: KeyboardEvent): void {
    const action = keymap().acting({
      key: event.key,
      ctrlKey: event.ctrlKey,
      metaKey: event.metaKey,
      shiftKey: event.shiftKey,
      altKey: event.altKey,
      target: inField(event.target) ? "field" : "page",
    });
    if (action !== "fork.here") return;
    const entry = active;
    if (entry === null || onFork === undefined) return;
    event.preventDefault();
    onFork(planFork(run.run, entry));
  }

  // The checker types a `{#snippet}` name as a void call, which the
  // lint lane rejects inside a render tag. Each name is taken again as
  // its `Snippet` type, and the template renders that.
  const person: Snippet<Parameters<typeof drawPerson>> = drawPerson;
  const reasoning: Snippet<Parameters<typeof drawReasoning>> = drawReasoning;
  const noteLine: Snippet<Parameters<typeof drawNoteLine>> = drawNoteLine;
  const turnView: Snippet<Parameters<typeof drawTurnView>> = drawTurnView;
</script>

<svelte:window onkeydown={forkKey} />

<!-- What a person said, and what the model said, are drawn as different
kinds of thing rather than as the same thing with different labels.

**A person is a shape; the model is the page.** A filled bubble,
right-aligned and held to 83% of the column, reads as one utterance; an
answer with no container at all, running the full measure, reads as a
document. -->
{#snippet drawPerson(text: string, label: string, at: number | undefined, entry: ForkEntry | null)}
  <div class="group relative my-base flex flex-col items-end">
    {#if entry !== null && onFork !== undefined}
      <ForkButton {entry} run={run.run} {onFork} onHover={hoverFork} />
    {/if}
    <div
      class="max-w-[83%] rounded-panel bg-speech px-pane py-base text-body leading-relaxed whitespace-pre-wrap"
    >
      {text}
    </div>
    <div class="mt-tight text-note text-text-disabled">
      {label}{#if at !== undefined} · {clock($lang, at)}{/if}
    </div>
  </div>
{/snippet}

<!-- How the model got to what it said, folded away.

Folded by default and never by exception: reasoning is most of what some
models produce, and a thread that opened it would be a thread whose
answer a person has to search for. While it is still arriving is the one
case worth watching, so that fold starts open. `<details>` owns the
disclosure, the keyboard and the reported state, exactly as the calls
fold below it does. -->
{#snippet drawReasoning(text: string, live: boolean)}
  <details class="my-tight text-note text-text-faint" open={live}>
    <summary
      class="cursor-pointer rounded-control px-tight marker:text-text-disabled hover:bg-chrome hover:text-text-quiet"
    >
      <span class="text-text-disabled">{say($lang, "talk_reasoning")}</span>
      {fill(say($lang, "talk_reasoning_length"), { n: count(text.length) })}
    </summary>
    <div class="mt-tight border-l border-edge-panel pl-base whitespace-pre-wrap break-words">
      {text}
    </div>
  </details>
{/snippet}

<!-- What else a turn came to beside what it said. `fenced` draws
nothing: a commit the run fenced is a fact for the run page, and the
thread's question is what this turn did or waits on. -->
{#snippet drawNoteLine(note: Note, turn: Turn)}
  {#if "arrived" in note}
    {@render person(note.arrived.said, note.arrived.from, undefined, {
      kind: "message",
      turn,
      text: note.arrived.said,
    })}
  {:else if "refused" in note}
    {@const error = note.refused.error}
    <div class="my-snug rounded-card border border-alert/40 px-base py-snug text-note text-text-quiet">
      <span class="text-alert">{error.code}</span> · {error.action} · {error.subject}
      {#if error.provider !== undefined && error.provider !== null}
        <!-- The kind is said in the reader's language; the city's own
        sentence stays folded beneath it for whoever is debugging. -->
        <div class="mt-tight text-text-faint">{providerClause($lang, error.provider)}</div>
        {#if error.recovery !== ""}
          <details class="mt-tight text-text-faint">
            <summary class="cursor-pointer">{say($lang, "notices_detail")}</summary>
            <div class="font-mono break-words">{error.recovery}</div>
          </details>
        {/if}
      {:else if error.recovery !== ""}
        <div class="mt-tight text-text-faint">{error.recovery}</div>
      {/if}
    </div>
  {:else if "waiting" in note}
    <div class="my-snug text-note text-alert">{say($lang, "talk_waiting_you")}</div>
  {:else if "discarded" in note}
    <div class="my-snug text-note text-text-faint">
      {fill(say($lang, "talk_discarded"), { n: String(note.discarded.count) })}
    </div>
  {/if}
{/snippet}

<!-- One round of the model: what it reasoned, what it called, what it
said, and what that cost. -->
{#snippet drawTurnView(turn: Turn, showEmpty: boolean)}
  {@const tokens = turn.used === null || turn.used === undefined ? null : turn.used.input + turn.used.output}
  {@const spent = turn.spent === null || turn.spent === undefined || turn.spent === 0 ? null : turn.spent}
  {@const empty = (turn.said ?? "") === "" && turn.calls.length === 0}
  {@const cut = turn.stopped === null || turn.stopped === undefined || FINISHED.includes(turn.stopped) ? null : turn.stopped}
  <div class="group relative my-base">
    {#if onFork !== undefined}
      <ForkButton entry={{ kind: "turn", turn }} run={run.run} {onFork} onHover={hoverFork} />
    {/if}
    {#each turn.notes.filter((note) => "arrived" in note) as note (noteAt(note))}
      {@render noteLine(note, turn)}
    {/each}
    {#if turn.thought}
      {@render reasoning(turn.thought, false)}
    {/if}
    {#if turn.calls.length > 0}
      <Calls
        calls={turn.calls}
        run={run.run}
        {turn}
        onFork={onFork === undefined ? undefined : planCall}
      />
    {/if}
    {#if turn.said}
      <div class="text-body">
        <div class="mb-tight text-note text-text-disabled">
          {who}
          {#if tokens !== null} · {fill(say($lang, "talk_tokens"), { n: count(tokens) })}{/if}
          {#if spent !== null} · {usd(spent)}{/if}
        </div>
        <Prose text={turn.said} />
      </div>
    {/if}
    {#if empty && showEmpty}
      <div class="my-snug rounded-card border border-alert/40 px-base py-snug text-note text-alert">
        {ceiling === null
          ? say($lang, "talk_said_nothing")
          : fill(say($lang, "talk_said_nothing_capped"), { n: count(ceiling) })}
      </div>
    {/if}
    {#if cut !== null}
      <div class="my-snug text-note text-alert">{fill(say($lang, "talk_cut_off"), { why: cut })}</div>
    {/if}
    {#each turn.notes.filter((note) => !("arrived" in note)) as note (noteAt(note))}
      {@render noteLine(note, turn)}
    {/each}
  </div>
{/snippet}

<section aria-label={run.run} class={frozen ? "settled" : undefined}>
  {#if task !== ""}
    {@render person(task, say($lang, "talk_you"), run.started ?? undefined, taskEntry)}
  {/if}
  {#each turns as turn (turn.opened)}
    {@render turnView(turn, !emptyRun)}
  {/each}
  <!-- No `aria-live` on the growing text: a screen reader told every
       token hears noise (ux B2). The frozen line below is what speaks,
       and it speaks once. -->
  {#if !frozen && run.thinking.length > 0}
    {@render reasoning(run.thinking, true)}
  {/if}
  {#if streaming}
    <div class="my-base text-body">
      <div class="mb-tight text-note text-text-disabled">{who}</div>
      <div class="whitespace-pre-wrap leading-relaxed">
        {settled}<span class="text-text-faint">{edge}</span><span
          class="blink ml-tight inline-block size-[6px] bg-accent align-baseline"
        ></span>
      </div>
    </div>
  {/if}
  {#if emptyRun}
    <!-- The zero-output run: the reason it stopped and one way out, the
         verb spelled as the command it sends (client-SPEC 4-10). -->
    <div class="my-snug rounded-card border border-alert/40 px-base py-snug text-note text-alert">
      <div>
        {ceiling === null
          ? say($lang, "talk_said_nothing")
          : fill(say($lang, "talk_said_nothing_capped"), { n: count(ceiling) })}
      </div>
      {#if why !== null}
        <div class="mt-tight">{fill(say($lang, "talk_cut_off"), { why })}</div>
      {/if}
      {#if onRetry !== undefined}
        <div class="mt-tight">
          <Button
            label={say($lang, "talk_send")}
            onPress={() => {
              onRetry(task);
            }}
          />
        </div>
      {/if}
    </div>
  {/if}
  {#if !frozen && !streaming}
    <div class="my-snug flex items-center gap-snug text-note text-text-faint">
      <span class="inline-block size-dot animate-pulse rounded-pill bg-accent"></span>
      <span>{posture}</span>
    </div>
  {/if}
  {#if frozen}
    <div class="my-wide flex items-center gap-base text-note text-text-disabled" role="status">
      <span class="h-px flex-1 bg-raised"></span>
      <a href={toFragment({ kind: "run", run: run.run })} class="hover:text-text-quiet">
        {completion}{#if closedAt !== null} · {clock($lang, closedAt)}{/if}
      </a>
      <span class="h-px flex-1 bg-raised"></span>
    </div>
  {/if}
</section>
