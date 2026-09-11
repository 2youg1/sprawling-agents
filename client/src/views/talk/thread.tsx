// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One run, read as a stretch of conversation: what the person asked,
// what the resident said turn by turn, what it did folded into one
// quiet line per turn, and - while it is still going - the words as
// they arrive. The rounds come from the server; only the live text and
// the posture come from what this page has folded itself.

import { For, Match, Show, Switch, createMemo, createSignal } from "solid-js";

import type { RunBelief } from "../../core/belief";
import { toFragment } from "../../core/route";
import { clock, count, usd } from "../../core/time";
import type { Call, Note, RoundsAnswer, RunId, Turn } from "../../wire";
import { useLang, useSay, useUi } from "../../ui";
import { Prose } from "../prose";

// How many characters at the growing edge are drawn faint. Wide enough
// that text emerges instead of appearing, narrow enough that the band a
// reader's eye sits on is not the shimmering one.
const EDGE = 10;

export interface ThreadProps {
  readonly run: RunBelief;
  readonly who: string;
}

function Person(props: { readonly text: string; readonly label: string; readonly at?: number | undefined }) {
  const lang = useLang();
  return (
    <div class="my-base flex flex-col items-end">
      <div class="max-w-measure rounded-panel bg-g2 px-pane py-base text-body leading-relaxed whitespace-pre-wrap">
        {props.text}
      </div>
      <div class="mt-tight text-note text-text-disabled">
        {props.label}
        <Show when={props.at}>{(at) => <span> · {clock(lang(), at())}</span>}</Show>
      </div>
    </div>
  );
}

function callWord(call: Call): string {
  return call.subject === null || call.subject === undefined ? call.tool : `${call.tool} ${call.subject}`;
}

// The provider's own words for a reply that ended the way replies end.
// Anything else - the ceiling, and whatever a provider adds next - is a
// reply that was cut off, and a reader is told so.
const FINISHED: readonly string[] = ["end_turn", "tool_use"];

function Calls(props: { readonly calls: readonly Call[]; readonly run: RunId }) {
  const say = useSay();
  const [open, setOpen] = createSignal(false);
  // Few calls are named one by one; many are counted by tool.
  const summary = createMemo(() => {
    if (props.calls.length <= 3) {
      return props.calls.map(callWord).join(" · ");
    }
    const byTool = new Map<string, number>();
    for (const call of props.calls) {
      byTool.set(call.tool, (byTool.get(call.tool) ?? 0) + 1);
    }
    return [...byTool.entries()]
      .map(([tool, n]) => (n === 1 ? tool : `${tool} ×${String(n)}`))
      .join(" · ");
  });
  return (
    <div class="my-tight text-note text-text-faint">
      <button
        type="button"
        class="rounded-control px-tight hover:bg-g1 hover:text-text-quiet"
        onClick={() => setOpen((held) => !held)}
        aria-expanded={open()}
      >
        <span class="inline-block w-pane">{open() ? "▾" : "▸"}</span>
        <span class="text-text-disabled">{say("talk_calls")}</span> {summary()}
      </button>
      <Show when={open()}>
        <ul class="mt-tight ml-pane border-l border-g2 pl-base">
          <For each={props.calls}>
            {(call) => (
              <li class="my-tight">
                <span class={call.outcome === "failed" ? "text-alert" : call.outcome === "waiting" ? "animate-pulse" : ""}>
                  {callWord(call)}
                </span>
                <Show when={call.output}>
                  {(output) => (
                    <>
                      <pre class="mt-tight max-h-output overflow-auto rounded-card bg-g1 p-snug font-mono text-note text-text-quiet">
                        {output().head}
                        <Show when={output().cut > 0}>
                          {"\n"}
                          <span class="text-text-disabled">{say("run_cut", { n: String(output().cut) })}</span>
                        </Show>
                      </pre>
                      <Show when={output().cut > 0}>
                        <a
                          class="text-note text-text-faint hover:text-text-quiet"
                          href={toFragment({ kind: "run", run: props.run })}
                        >
                          {say("talk_call_open")}
                        </a>
                      </Show>
                    </>
                  )}
                </Show>
              </li>
            )}
          </For>
        </ul>
      </Show>
    </div>
  );
}

function NoteLine(props: { readonly note: Note; readonly who: string }) {
  const say = useSay();
  return (
    <Switch>
      <Match when={"arrived" in props.note ? props.note.arrived : undefined}>
        {(arrived) => <Person text={arrived().said} label={arrived().from} />}
      </Match>
      <Match when={"refused" in props.note ? props.note.refused.error : undefined}>
        {(error) => (
          <div class="my-snug rounded-card border border-alert/40 px-base py-snug text-note text-text-quiet">
            <span class="text-alert">{error().code}</span> · {error().action} · {error().subject}
            <Show when={error().recovery !== ""}>
              <div class="mt-tight text-text-faint">{error().recovery}</div>
            </Show>
          </div>
        )}
      </Match>
      <Match when={"waiting" in props.note}>
        <div class="my-snug text-note text-alert">{say("talk_waiting_you")}</div>
      </Match>
      <Match when={"discarded" in props.note ? props.note.discarded : undefined}>
        {(discarded) => (
          <div class="my-snug text-note text-text-faint">
            {say("talk_discarded", { n: String(discarded().count) })}
          </div>
        )}
      </Match>
    </Switch>
  );
}

function TurnView(props: {
  readonly turn: Turn;
  readonly who: string;
  readonly run: RunId;
  // The `main` model's output ceiling, when the city knows one: the
  // number a reader needs to read an empty reply as a limit rather
  // than as silence.
  readonly ceiling: number | null;
}) {
  const say = useSay();
  const tokens = createMemo(() => {
    const used = props.turn.used;
    return used === null || used === undefined ? null : used.input + used.output;
  });
  const spent = createMemo(() => {
    const held = props.turn.spent;
    return held === null || held === undefined || held === 0 ? null : held;
  });
  // A turn that said nothing and did nothing is the empty reply: the
  // run moves on and a reader is shown a blank where an answer was
  // paid for. A turn that only made tool calls is not that.
  const empty = createMemo(() => {
    const said = props.turn.said;
    return (said === null || said === undefined || said === "") && props.turn.calls.length === 0;
  });
  const cut = createMemo(() => {
    const why = props.turn.stopped;
    return why === null || why === undefined || FINISHED.includes(why) ? null : why;
  });
  return (
    <div class="my-base">
      <For each={props.turn.notes.filter((note) => "arrived" in note)}>
        {(note) => <NoteLine note={note} who={props.who} />}
      </For>
      <Show when={props.turn.calls.length > 0}>
        <Calls calls={props.turn.calls} run={props.run} />
      </Show>
      <Show when={props.turn.said}>
        {(said) => (
          <div class="text-body">
            <div class="mb-tight text-note text-text-disabled">
              {props.who}
              <Show when={tokens()}>{(n) => <span> · {say("talk_tokens", { n: count(n()) })}</span>}</Show>
              <Show when={spent()}>{(micros) => <span> · {usd(micros())}</span>}</Show>
            </div>
            <Prose text={said()} />
          </div>
        )}
      </Show>
      <Show when={empty()}>
        <div class="my-snug rounded-card border border-alert/40 px-base py-snug text-note text-alert">
          <Show
            when={props.ceiling}
            fallback={say("talk_said_nothing")}
          >
            {(n) => say("talk_said_nothing_capped", { n: count(n()) })}
          </Show>
        </div>
      </Show>
      <Show when={cut()}>
        {(why) => <div class="my-snug text-note text-alert">{say("talk_cut_off", { why: why() })}</div>}
      </Show>
      <For each={props.turn.notes.filter((note) => !("arrived" in note))}>
        {(note) => <NoteLine note={note} who={props.who} />}
      </For>
    </div>
  );
}

function Posture(props: { readonly run: RunBelief; readonly who: string }) {
  const say = useSay();
  const doing = () => props.run.doing;
  return (
    <div class="my-snug flex items-center gap-snug text-note text-text-faint" aria-live="polite">
      <span class="inline-block size-dot animate-pulse rounded-pill bg-accent" />
      <span>
        {(() => {
          const d = doing();
          switch (d.kind) {
            case "thinking":
              return say("talk_thinking", { who: props.who });
            case "calling":
              return d.subject === null ? d.tool : `${d.tool} ${d.subject}`;
            case "waiting":
              return say("talk_waiting_you");
            case "frozen":
              return "";
          }
        })()}
      </span>
    </div>
  );
}

export function Thread(props: ThreadProps) {
  const ui = useUi();
  const say = useSay();
  const lang = useLang();
  const rounds = createMemo(() => ui.conn.asking.ask({ rounds: { run: props.run.run } }));
  const answer = createMemo<RoundsAnswer | undefined>(() => {
    const held = rounds()();
    return held !== undefined && "rounds" in held ? held.rounds : undefined;
  });
  const task = createMemo(() => answer()?.opening?.task ?? props.run.task);
  // What this city told the provider a reply may be at most. Read here
  // rather than per turn, because it is one fact about the city and not
  // one fact about a turn.
  const endpoints = ui.conn.asking.ask("endpoint_view");
  const ceiling = createMemo<number | null>(() => {
    const held = endpoints();
    if (held === undefined || !("endpoints" in held)) return null;
    return held.endpoints.chosen.find((each) => each.tag === "main")?.max_output_tokens ?? null;
  });
  const frozen = () => props.run.doing.kind === "frozen";
  const completion = createMemo(() => {
    const word = answer()?.closing?.completion ?? (props.run.doing.kind === "frozen" ? props.run.doing.completion : null);
    switch (word) {
      case "done":
        return say("outcome_done");
      case "cancelled":
        return say("outcome_cancelled");
      case "limit":
        return say("outcome_limit");
      case null:
        return say("outcome_frozen");
      default:
        return word;
    }
  });
  // The turn being said right now is not yet in the rounds; it is the
  // page's own text until the record holds it.
  const streaming = createMemo(() => !frozen() && props.run.saying.length > 0);
  // The last few characters are drawn faint, so text emerges rather than
  // appearing. Derived from the text and nothing else: no timer, no
  // queue, no per-character node. When the call returns, belief clears
  // `saying` and the settled record takes over, so nothing has to decide
  // when the edge stops being an edge.
  const settled = createMemo(() => props.run.saying.slice(0, -EDGE));
  const edge = createMemo(() => props.run.saying.slice(-EDGE));

  return (
    <section aria-label={props.run.run} class={frozen() ? "settled" : undefined}>
      <Show when={task()}>
        {(text) => <Person text={text()} label={say("talk_you")} at={props.run.started ?? undefined} />}
      </Show>
      <For each={answer()?.turns ?? []}>
        {(turn) => <TurnView turn={turn} who={props.who} run={props.run.run} ceiling={ceiling()} />}
      </For>
      <Show when={streaming()}>
        <div class="my-base text-body">
          <div class="mb-tight text-note text-text-disabled">{props.who}</div>
          <div class="whitespace-pre-wrap leading-relaxed">
            {settled()}
            <span class="text-text-faint">{edge()}</span>
            <span class="ml-tight inline-block h-caret w-hair animate-pulse bg-accent align-text-bottom" />
          </div>
        </div>
      </Show>
      <Show when={!frozen() && !streaming()}>
        <Posture run={props.run} who={props.who} />
      </Show>
      <Show when={frozen()}>
        <div class="my-wide flex items-center gap-base text-note text-text-disabled">
          <span class="h-px flex-1 bg-g2" />
          <a href={toFragment({ kind: "run", run: props.run.run })} class="hover:text-text-quiet">
            {completion()}
            <Show when={answer()?.closing?.at}>{(at) => <span> · {clock(lang(), at())}</span>}</Show>
          </a>
          <span class="h-px flex-1 bg-g2" />
        </div>
      </Show>
    </section>
  );
}
