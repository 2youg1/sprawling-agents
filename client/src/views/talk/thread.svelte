<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // One run, read as a stretch of conversation: what the person asked,
  // what the resident said turn by turn under a head naming who and when,
  // one quiet line per call it made, and - while it is still going - the
  // words as they arrive. The rounds come from the server; only the live
  // text, the posture and the rhythm the words arrived in come from what
  // this page has seen itself.
  //
  // Every entry a person could have branched the conversation at - their
  // own words, a reply, a call - carries the one fork action. Hovering
  // or focusing shows it; the `fork.here` chord answers for the entry
  // under the hand (roadmap S2, ux A7).
  import { readAnswer } from "../../core/answered";
  import { QUERIES } from "../../core/asking";
  import type { RunBelief } from "../../core/belief";
  import { keymap } from "../../core/keys";
  import { drawsCalls } from "../../core/results";
  import { fill, say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { clock, count } from "../../core/time";
  import { untrack } from "svelte";
  import { readable } from "svelte/store";
  import type { Query, RunId, Seq } from "../../wire";
  import { ui } from "../../ui";
  import Failed from "./failed.svelte";
  import Unanswered from "../parts/unanswered.svelte";
  import Person from "./person.svelte";
  import Saying from "./saying.svelte";
  import { phaseOf } from "../runs/lineage";
  import TurnView from "./turn.svelte";
  import { heard, landed, lost } from "./arrivals.svelte";
  import { callWord } from "./calls";
  import { firstHeadSaid } from "./frozen";
  import { restriction, sandboxQuery, sandboxSaid } from "./sandbox";
  import { called, dispatcherOf } from "./naming";
  import { planFork } from "./forking";
  import { turnsAround } from "./around";
  import { cutOff, silentRun } from "./silence";
  import type { Phase } from "./silence";
  import type { ForkEntry, ForkPlan } from "./forking";

  interface Props {
    readonly run: RunBelief;
    // Absent where the page holding the thread cannot branch: the run
    // page draws the same conversation with nowhere to fork to.
    readonly onFork?: ((plan: ForkPlan) => void) | undefined;
    // The run came back with no words: say the task again as a new run.
    // Absent for the same reason.
    readonly onRetry?: ((task: string) => void) | undefined;
    // Whether this run opens the stretch of the room on screen, so its
    // first head states the model the session froze (docs/frontend-method.md §7D).
    // A later run of the same session leaves that to the first.
    readonly opens?: boolean;
    // A letter's line in this run: only the turns around it are drawn,
    // with a link to the whole session (client D91). Absent everywhere
    // else, where the thread is the whole session.
    readonly around?: Seq | null;
  }

  const { run, onFork, onRetry, opens = true, around = null }: Props = $props();

  const u = ui();
  const { lang } = u;
  const held = u.prefs.held;

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
  const question: Query = { rounds: { run: readRun } };
  const rounds = u.conn.asking.ask(question);
  const read = $derived(readAnswer($rounds, (held) => ("rounds" in held ? held.rounds : undefined)));
  const answer = $derived(read.kind === "held" ? read.value : undefined);
  const turns = $derived(answer?.turns ?? []);
  const drawn = $derived(turnsAround(turns, around));
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
  const phase = $derived<Phase>(frozen ? "frozen" : "live");
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
  // page's own text until the record holds it. When the call returns,
  // belief clears `saying` and the settled record takes over, so nothing
  // has to decide when the growing text stops growing.
  const streaming = $derived(!frozen && run.saying.length > 0);
  const closedAt = $derived(answer?.closing?.at ?? null);
  // A frozen run that said nothing at all is one card with the reason
  // and a way out, not one grey box per turn (ux: the zero-output run).
  const emptyRun = $derived(read.kind === "held" && silentRun(turns, phase));
  const why = $derived(cutOff(turns.at(-1)?.stopped));
  const whole = $derived(drawsCalls($held.showing));
  // A call still running is already a line of its own, with its timer and
  // what it waits for; the posture line under it would say it twice.
  const lineSaysIt = $derived(
    whole && (run.doing.kind === "calling" || run.doing.kind === "waiting") &&
      (turns.at(-1)?.calls.some((call) => call.outcome === "waiting") ?? false),
  );
  // Who speaks in this run: the name its session froze, which a rename
  // after the session began does not reach (refrain §3-13).
  const who = $derived(
    run.addr === null ? say($lang, "talk_resident") : called(run.addr, answer?.opening?.names?.mayor, $lang),
  );
  // Who the opening task came from (`Opening.dispatched_by`): the User's
  // own words are "you", a task a resident handed down names that
  // resident and links to its room, and one the city's desk dispatched
  // says so. The link goes to the run that delegated the task
  // (`Opening.parent`), whose page is the parent's session; a ledger
  // written before the parent was recorded links to the resident's room.
  const from = $derived.by((): { readonly label: string; readonly href: string | undefined } => {
    const by = dispatcherOf(answer?.opening?.dispatched_by);
    const parent = answer?.opening?.parent ?? null;
    if (by === null) return { label: say($lang, "talk_you"), href: undefined };
    switch (by.kind) {
      case "person":
        return { label: say($lang, "talk_you"), href: undefined };
      case "city":
        return { label: say($lang, "talk_dispatched_city"), href: undefined };
      case "resident":
        return {
          label: fill(say($lang, "talk_dispatched_by"), { who: called(by.address, answer?.opening?.names?.mayor, $lang) }),
          href: toFragment(parent === null ? { kind: "talk", address: by.address } : { kind: "run", run: parent }),
        };
    }
  });
  // The session facts each round's head states: on the first head the
  // session's model, the effort and policy its run was dispatched with,
  // and the sandbox when one restricts the room's building; afterwards
  // the model again only where a round answered with a different one.
  // A fact that is not known is left out, never written as "none".
  const firstModel = $derived(turns.at(0)?.model ?? null);
  const building = $derived(opens && run.addr !== null ? u.conn.asking.ask(sandboxQuery(run.addr)) : readable(undefined));
  const bounded = $derived(restriction($building));
  const firstStated = $derived.by(() => {
    const facts = [firstModel ?? "", firstHeadSaid(answer?.opening, $lang), bounded === null ? "" : sandboxSaid(bounded, $lang)]
      .filter((part) => part !== "")
      .join(" · ");
    return facts === "" ? null : facts;
  });
  const stated = $derived(
    turns.map((turn, at) => {
      const model = turn.model ?? null;
      if (at === 0) return opens ? firstStated : null;
      return model !== null && model !== (turns[at - 1]?.model ?? null) ? model : null;
    }),
  );

  // What this page sees of the reply while it streams, filed under the
  // round it becomes (`arrivals.svelte.ts`). That round is opened before
  // its first word arrives, so it is the newest round still wordless
  // while the words grow; a reply whose round the page never saw is
  // dropped rather than drawn on another round.
  let streamedInto: Seq | null = null;
  let wasStreaming = false;
  $effect(() => {
    const length = run.saying.length;
    untrack(() => {
      heard(run.run, length, u.now());
    });
  });
  $effect(() => {
    const now = streaming;
    const newest = turns.at(-1);
    if (now && newest !== undefined && (newest.said ?? "") === "") streamedInto = newest.opened;
    if (now || !wasStreaming) {
      wasStreaming = now;
      return;
    }
    wasStreaming = false;
    if (streamedInto === null) lost(run.run);
    else landed(run.run, streamedInto);
    streamedInto = null;
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
      case "awaiting_reply":
        return say($lang, "run_doing_awaiting_reply");
      case "unknown":
        return say($lang, "city_at_work");
      case "frozen":
        return "";
    }
  });

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

</script>

<svelte:window onkeydown={forkKey} />

<section aria-label={run.run} class={frozen ? "settled" : undefined}>
  {#if task !== ""}
    <Person
      text={task}
      label={from.label}
      labelHref={from.href}
      at={run.started ?? undefined}
      entry={taskEntry}
      run={run.run}
      {onFork}
      onHover={hoverFork}
    />
  {/if}
  {#if read.kind === "unavailable"}
    <Unanswered query={read.query} asked={question} />
  {/if}
  {#each turns.slice(drawn.from, drawn.to) as turn, shown (turn.opened)}
    {@const at = shown + drawn.from}
    <TurnView
      {turn}
      run={run.run}
      {who}
      model={stated[at] ?? null}
      live={phase}
      doing={at === turns.length - 1 && !frozen ? run.doing : undefined}
      showEmpty={!emptyRun}
      {ceiling}
      {whole}
      {onFork}
      onCall={onFork === undefined ? undefined : planCall}
      onHover={hoverFork}
    />
  {/each}
  {#if drawn.cut}
    <p class="my-snug text-note text-text-faint">
      <a class="underline hover:text-text" href={toFragment({ kind: "run", run: run.run })}>{say($lang, "letter_whole_talk")}</a>
    </p>
  {/if}
  <!-- No `aria-live` on the growing text: a screen reader told every
       token hears noise (ux B2). The frozen line below is what speaks,
       and it speaks once. -->
  {#if !frozen && run.thinking.length > 0 && whole}
    <details class="my-tight text-note text-text-faint" open>
      <summary class="cursor-pointer rounded-control px-tight marker:text-text-faint hover:bg-chrome hover:text-text-quiet">
        <span class="text-text-faint">{say($lang, "talk_reasoning")}</span>
        {fill(say($lang, "talk_reasoning_length"), { n: count(run.thinking.length) })}
      </summary>
      <div class="mt-tight border-l border-edge-panel pl-base whitespace-pre-wrap break-words">{run.thinking}</div>
    </details>
  {/if}
  {#if streaming}
    <Saying text={run.saying} {who} />
    <!-- While the model is still saying this, a steer is heard at the
         end of it: the pin stands where the words stop (refrain §3-5). -->
    <span class="steer-pin" aria-hidden="true"></span>
  {/if}
  {#if emptyRun}
    <!-- The zero-output run: what happened where the reply would have
         been, and one way out, the verb spelled as the command it sends
         (client/Spec.lean §4-10). A run the person stopped says so rather
         than blaming the model. -->
    {@const stopped = (answer?.closing?.completion ?? (run.doing.kind === "frozen" ? run.doing.completion : null)) === "cancelled"}
    <Failed
      what={stopped
        ? say($lang, "talk_failed_cancelled")
        : ceiling === null
          ? say($lang, "talk_said_nothing")
          : fill(say($lang, "talk_said_nothing_capped"), { n: count(ceiling) })}
      settings={stopped ? "absent" : "offered"}
      onRetry={onRetry === undefined ? undefined : () => {
        onRetry(task);
      }}
    />
    {#if why !== null}
      <div class="text-note text-text-faint">{fill(say($lang, "talk_cut_off"), { why })}</div>
    {/if}
  {/if}
  {#if !frozen && !streaming && !lineSaysIt}
    <div class="my-snug flex items-center gap-snug text-note text-text-faint">
      <span class="inline-block size-dot animate-pulse rounded-pill bg-accent"></span>
      <span>{posture}</span>
      {#if run.doing.kind === "thinking"}
        <span class="steer-pin" aria-hidden="true"></span>
      {/if}
    </div>
  {/if}
  {#if frozen}
    <div class="my-wide flex items-center gap-base text-note text-text-faint" role="status" data-wear={phaseOf(run.doing)}>
      <span class="h-px flex-1 bg-raised"></span>
      <a href={toFragment({ kind: "run", run: run.run })} class="hover:text-text-quiet">
        {completion}{#if closedAt !== null} · {clock($lang, closedAt)}{/if}
      </a>
      <span class="h-px flex-1 bg-raised"></span>
    </div>
  {/if}
</section>
