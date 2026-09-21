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

import { QUERIES } from "../../core/asking";
import type { RunBelief } from "../../core/belief";
import { toFragment } from "../../core/route";
import { clock, count, usd } from "../../core/time";
import type { Note, RoundsAnswer, RunId, Turn } from "../../wire";
import { useLang, useSay, useUi } from "../../ui";
import { Prose } from "../prose";
import { Calls, callWord } from "./calls";

// How many characters at the growing edge are drawn faint. Wide enough
// that text emerges instead of appearing, narrow enough that the band a
// reader's eye sits on is not the shimmering one.
const EDGE = 10;

export interface ThreadProps {
  readonly run: RunBelief;
  readonly who: string;
}

// What a person said, and what the model said, are drawn as different
// kinds of thing rather than as the same thing with different labels.
//
// **A person is a shape; the model is the page.** A filled bubble,
// right-aligned and held to 83% of the column, reads as one utterance;
// an answer with no container at all, running the full measure, reads
// as a document. The reverse - both sides in identical blocks,
// separated only by a grey name above each - is what this thread drew
// before, and it made a two-line question and a two-page answer look
// like the same kind of object.
//
// 83% rather than the prose measure: the bubble has to be visibly
// narrower than the column it sits in, or right-alignment says nothing,
// and it has to be wide enough that a pasted paragraph does not become
// a ribbon.
const SAID_WIDTH = "max-inline-size:83%";

function Person(props: { readonly text: string; readonly label: string; readonly at?: number | undefined }) {
  const lang = useLang();
  return (
    <div class="my-base flex flex-col items-end">
      <div
        class="rounded-panel bg-speech px-pane py-base text-body leading-relaxed whitespace-pre-wrap"
        style={SAID_WIDTH}
      >
        {props.text}
      </div>
      <div class="mt-tight text-note text-text-disabled">
        {props.label}
        <Show when={props.at}>{(at) => <span> · {clock(lang(), at())}</span>}</Show>
      </div>
    </div>
  );
}

// The provider's own words for a reply that ended the way replies end.
// Anything else - the ceiling, and whatever a provider adds next - is a
// reply that was cut off, and a reader is told so.
const FINISHED: readonly string[] = ["end_turn", "tool_use"];

// How the model got to what it said, folded away.
//
// Folded by default and never by exception: reasoning is most of what
// some models produce, and a thread that opened it would be a thread
// whose answer a person has to search for. While it is still arriving
// is the one case worth watching, and `live` is read once, at the
// moment this section is first drawn, because what it settles is where
// the fold starts and not where it stays.
function Reasoning(props: { readonly text: string; readonly live?: true }) {
  const say = useSay();
  // eslint-disable-next-line solid/reactivity -- the initial fold, read once by design
  const [open, setOpen] = createSignal(props.live === true);
  return (
    <div class="my-tight text-note text-text-faint">
      <button
        type="button"
        class="rounded-control px-tight hover:bg-chrome hover:text-text-quiet"
        onClick={() => setOpen((held) => !held)}
        aria-expanded={open()}
      >
        <span class="inline-block w-pane">{open() ? "▾" : "▸"}</span>
        <span class="text-text-disabled">{say("talk_reasoning")}</span>{" "}
        {say("talk_reasoning_length", { n: count(props.text.length) })}
      </button>
      <Show when={open()}>
        <div class="mt-tight border-l border-edge-panel pl-base whitespace-pre-wrap break-words">
          {props.text}
        </div>
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
      <Show when={props.turn.thought}>{(thought) => <Reasoning text={thought()} />}</Show>
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
              return d.tool === null ? say("run_doing_calling") : callWord(d.tool, d.subject);
            case "waiting":
              return say("talk_waiting_you");
            // A run this page knows is live and cannot place: the word
            // says it is working and claims nothing about the phase.
            case "unknown":
              return say("city_at_work");
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
  const endpoints = ui.conn.asking.ask(QUERIES.endpoints);
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
      <Show when={!frozen() && props.run.thinking.length > 0}>
        <Reasoning text={props.run.thinking} live />
      </Show>
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
          <span class="h-px flex-1 bg-raised" />
          <a href={toFragment({ kind: "run", run: props.run.run })} class="hover:text-text-quiet">
            {completion()}
            <Show when={answer()?.closing?.at}>{(at) => <span> · {clock(lang(), at())}</span>}</Show>
          </a>
          <span class="h-px flex-1 bg-raised" />
        </div>
      </Show>
    </section>
  );
}
