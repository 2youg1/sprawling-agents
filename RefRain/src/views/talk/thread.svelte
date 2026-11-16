

<script lang="ts">
  
  
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
  import AsideLink from "./aside_link.look.svelte";
  import Reasoning from "./reasoning.svelte";
  import Rule from "./rule.look.svelte";
  import SteerPin from "./steer_pin.look.svelte";
  import { heard, landed, lost } from "./arrivals.svelte";
  import { callWord } from "./calls";
  import { firstHeadSaid } from "./frozen";
  import type { FrozenFacts } from "./seat";
  import { restriction, sandboxQuery, sandboxSaid } from "./sandbox";
  import { called, dispatcherOf } from "./naming";
  import { planFork, roundsThrough } from "./forking";
  import { turnsAround } from "./around";
  import { cutOff, silentRun } from "./silence";
  import type { Phase } from "./silence";
  import type { ForkEntry, ForkPlan } from "./forking";

  interface Props {
    readonly through?: Seq | null;
    readonly run: RunBelief;
    
    
    readonly onFork?: ((plan: ForkPlan) => void) | undefined;
    
    
    readonly onRetry?: ((task: string) => void) | undefined;
    
    
    readonly facts?: FrozenFacts;
    
    
    readonly around?: Seq | null;
  }

  const { run, onFork, onRetry, facts = "head", around = null, through = null }: Props = $props();

  const u = ui();
  const { lang } = u;
  const held = u.prefs.held;

  
  let active: ForkEntry | null = $state(null);
  const hoverFork = (entry: ForkEntry | null): void => {
    active = entry;
  };

  
  // svelte-ignore state_referenced_locally (the capture is the point: the question is minted once, for the run this thread is keyed to)
  const readRun: RunId = run.run;
  const question: Query = { rounds: { run: readRun } };
  const rounds = u.conn.asking.ask(question);
  const read = $derived(
    readAnswer($rounds, (held) => ("rounds" in held ? held.rounds : undefined)),
  );
  const answer = $derived(read.kind === "held" ? roundsThrough(read.value, through) : undefined);
  const turns = $derived(answer?.turns ?? []);
  const drawn = $derived(turnsAround(turns, around));
  const task = $derived(answer?.opening?.task ?? run.task ?? "");
  
  
  const taskEntry = $derived.by((): ForkEntry | null => {
    const first = turns.at(0);
    return first === undefined ? null : { kind: "message", turn: first, text: task };
  });
  
  
  const endpoints = u.conn.asking.ask(QUERIES.endpoints);
  const ceiling = $derived.by((): number | null => {
    const held = $endpoints;
    if (held === undefined || !("endpoints" in held)) return null;
    return held.endpoints.chosen.find((each) => each.tag === "main")?.max_output_tokens ?? null;
  });
  const frozen = $derived(run.doing.kind === "frozen");
  const phase = $derived<Phase>(frozen ? "frozen" : "live");
  
  
  const word = $derived(
    answer?.closing?.completion ?? (run.doing.kind === "frozen" ? run.doing.completion : null),
  );
  const completion = $derived.by((): string => {
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
  
  
  const streaming = $derived(through === null && !frozen && run.saying.length > 0);
  const closedAt = $derived(answer?.closing?.at ?? null);
  
  
  const emptyRun = $derived(read.kind === "held" && silentRun(turns, phase));
  
  
  const failure = $derived(answer?.closing?.failure ?? null);
  
  
  const stopped = $derived(word === "cancelled");
  
  
  const wayOut = $derived<"offered" | "absent" | undefined>(
    failure !== null ? undefined : stopped ? "absent" : "offered",
  );
  const why = $derived(cutOff(turns.at(-1)?.stopped));
  const whole = $derived(drawsCalls($held.showing));
  
  
  const lineSaysIt = $derived(
    whole &&
      (run.doing.kind === "calling" || run.doing.kind === "waiting") &&
      (turns.at(-1)?.calls.some((call) => call.outcome === "waiting") ?? false),
  );
  
  
  const who = $derived(
    run.addr === null
      ? say($lang, "talk_resident")
      : called(run.addr, answer?.opening?.names?.mayor, $lang),
  );
  
  
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
          label: fill(say($lang, "talk_dispatched_by"), {
            who: called(by.address, answer?.opening?.names?.mayor, $lang),
          }),
          href: toFragment(
            parent === null ? { kind: "talk", address: by.address } : { kind: "run", run: parent },
          ),
        };
    }
  });
  
  
  const firstModel = $derived(turns.at(0)?.model ?? null);
  const building = $derived(
    facts === "head" && run.addr !== null
      ? u.conn.asking.ask(sandboxQuery(run.addr))
      : readable(undefined),
  );
  const bounded = $derived(restriction($building));
  const firstStated = $derived.by(() => {
    const facts = [
      firstModel ?? "",
      firstHeadSaid(answer?.opening, $lang),
      bounded === null ? "" : sandboxSaid(bounded, $lang),
    ]
      .filter((part) => part !== "")
      .join(" · ");
    return facts === "" ? null : facts;
  });
  const stated = $derived(
    turns.map((turn, at) => {
      const model = turn.model ?? null;
      if (at === 0) return facts === "head" ? firstStated : null;
      return model !== null && model !== (turns[at - 1]?.model ?? null) ? model : null;
    }),
  );

  
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
      onHover={hoverFork}
    />
  {/each}
  {#if drawn.cut}
    <p class="my-snug text-note text-text-faint">
      <AsideLink
        text={say($lang, "letter_whole_talk")}
        href={toFragment({ kind: "run", run: run.run })}
      />
    </p>
  {/if}
  
  {#if through === null && !frozen && run.thinking.length > 0 && whole}
    <div class="my-tight">
      <Reasoning text={run.thinking} open />
    </div>
  {/if}
  {#if streaming}
    <Saying text={run.saying} {who} />
    
    <SteerPin />
  {/if}
  {#if emptyRun}
    
    <Failed
      what={stopped
        ? say($lang, "talk_failed_cancelled")
        : ceiling === null
          ? say($lang, "talk_said_nothing")
          : fill(say($lang, "talk_said_nothing_capped"), { n: count(ceiling) })}
      error={failure ?? undefined}
      settings={wayOut}
      onRetry={onRetry === undefined
        ? undefined
        : () => {
            onRetry(task);
          }}
    />
    {#if why !== null}
      <div class="text-note text-text-faint">{fill(say($lang, "talk_cut_off"), { why })}</div>
    {/if}
  {/if}
  {#if !frozen && !streaming && !lineSaysIt}
    <div class="my-snug flex items-center gap-snug text-note text-text-faint">
      <span class="thinking-mark" aria-hidden="true"><i></i><i></i><i></i></span>
      <span>{posture}</span>
      {#if run.doing.kind === "thinking"}
        <SteerPin />
      {/if}
    </div>
  {/if}
  {#if frozen}
    <div class="my-wide">
      <Rule
        parts={[
          {
            kind: "link",
            text: closedAt === null ? completion : `${completion} · ${clock($lang, closedAt)}`,
            href: toFragment({ kind: "run", run: run.run }),
          },
        ]}
        mark="none"
        wire={{ role: "status", "data-wear": phaseOf(run.doing) }}
      />
    </div>
  {/if}
</section>
